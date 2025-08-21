#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::database::{
    models::{
        DbOrderWallet, DbRequestId, DbUtxoDetail, DbZkAccount, EncryptedWallet, NewEncryptedWallet,
    },
    schema::{encrypted_wallets, order_wallets, request_ids, utxo_details, zk_accounts},
};
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::security::SecurePassword;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::wallet::Wallet;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::zkos_accounts::zkaccount::ZkAccount;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit, OsRng},
};
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use diesel::prelude::*;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use rand_core::RngCore;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use secrecy::{ExposeSecret, SecretString};
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use serde::{Deserialize, Serialize};
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use sha2::{Digest, Sha256};

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use std::{collections::HashMap, sync::Arc};

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::database::connection::{DbPool, get_conn};

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use chrono::NaiveDateTime;
use log::debug;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
#[derive(Debug, Clone, Serialize)]
pub struct DatabaseManager {
    wallet_id: String,
    #[serde(skip)]
    pool: Arc<DbPool>,
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletList {
    pub wallet_id: String,
    pub created_at: NaiveDateTime,
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
impl DatabaseManager {
    pub fn new(wallet_id: String, pool: DbPool) -> Self {
        Self {
            wallet_id,
            pool: Arc::new(pool),
        }
    }

    pub fn get_wallet_id(&self) -> &str {
        &self.wallet_id
    }
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    // ZkAccount operations
    pub fn save_zk_account(&self, zk_account: &ZkAccount) -> Result<(), String> {
        let new_account = DbZkAccount::from_zk_account(zk_account, self.wallet_id.clone());
        let mut conn = get_conn(self.pool())?;
        let n = diesel::insert_into(zk_accounts::table)
            .values(&new_account)
            .on_conflict((zk_accounts::wallet_id, zk_accounts::account_index))
            .do_update()
            .set((
                zk_accounts::balance.eq(new_account.balance),
                zk_accounts::io_type_value.eq(new_account.io_type_value),
                zk_accounts::on_chain.eq(new_account.on_chain),
                zk_accounts::updated_at.eq(new_account.updated_at),
            ))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to save zk_account: {}", e))?;
        debug!(
            "The upserted row: {} for account_index: {}",
            n, zk_account.index
        );
        Ok(())
    }

    pub fn update_zk_account(&self, zk_account: &ZkAccount) -> Result<(), String> {
        let now = chrono::Utc::now().naive_utc();
        let mut conn = get_conn(self.pool())?;
        let n = diesel::update(
            zk_accounts::table.filter(
                zk_accounts::wallet_id
                    .eq(&self.wallet_id)
                    .and(zk_accounts::account_index.eq(zk_account.index as i64)),
            ),
        )
        .set((
            zk_accounts::balance.eq(zk_account.balance as i64),
            zk_accounts::io_type_value.eq(zk_account.io_type.clone() as i32),
            zk_accounts::on_chain.eq(zk_account.on_chain),
            zk_accounts::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to update zk_account: {}", e))?;
        debug!(
            "The updated row: {} for account_index: {}",
            n, zk_account.index
        );
        Ok(())
    }

    pub fn remove_zk_account(&self, account_index: u64) -> Result<(), String> {
        let mut conn = get_conn(self.pool())?;
        let n = diesel::delete(
            zk_accounts::table.filter(
                zk_accounts::wallet_id
                    .eq(&self.wallet_id)
                    .and(zk_accounts::account_index.eq(account_index as i64)),
            ),
        )
        .execute(&mut conn)
        .map_err(|e| format!("Failed to remove zk_account: {}", e))?;
        debug!(
            "The deleted row: {:?} for account_index: {}",
            n, account_index
        );
        Ok(())
    }

    pub fn load_all_zk_accounts(&self) -> Result<HashMap<u64, ZkAccount>, String> {
        let mut conn = get_conn(self.pool())?;
        let db_accounts: Vec<DbZkAccount> = zk_accounts::table
            .filter(zk_accounts::wallet_id.eq(&self.wallet_id))
            .load(&mut conn)
            .map_err(|e| format!("Failed to load zk_accounts: {}", e))?;

        let mut accounts = HashMap::new();
        for db_account in db_accounts {
            let zk_account = db_account.to_zk_account()?;
            accounts.insert(zk_account.index, zk_account);
        }

        Ok(accounts)
    }
    pub fn get_max_account_index(&self) -> Result<u64, String> {
        let mut conn = get_conn(self.pool())?;
        let max_index: Option<i64> = zk_accounts::table
            .filter(zk_accounts::wallet_id.eq(&self.wallet_id))
            .select(zk_accounts::account_index)
            .order(zk_accounts::account_index.desc())
            .first(&mut conn)
            .optional()
            .map_err(|e| format!("Failed to get max account index: {}", e))?;
        Ok(max_index.unwrap_or(0) as u64)
    }

    /// List all wallet IDs that have an encrypted wallet stored, with created_at
    pub fn get_wallet_list(pool: &DbPool) -> Result<Vec<WalletList>, String> {
        let mut conn = get_conn(pool)?;
        let rows: Vec<(String, NaiveDateTime)> = encrypted_wallets::table
            .select((encrypted_wallets::wallet_id, encrypted_wallets::created_at))
            .order(encrypted_wallets::created_at.desc())
            .load::<(String, NaiveDateTime)>(&mut conn)
            .map_err(|e| format!("Failed to load wallet list: {}", e))?;
        let list = rows
            .into_iter()
            .map(|(wallet_id, created_at)| WalletList {
                wallet_id,
                created_at,
            })
            .collect();
        Ok(list)
    }

    pub fn check_wallet_id_exists(pool: &DbPool, wallet_id: &str) -> Result<bool, String> {
        let mut conn = get_conn(pool)?;
        let exists: Option<EncryptedWallet> = encrypted_wallets::table
            .filter(encrypted_wallets::wallet_id.eq(wallet_id))
            .first(&mut conn)
            .optional()
            .map_err(|e| format!("Failed to check wallet ID existence: {}", e))?;
        Ok(exists.is_some())
    }

    // Wallet encryption operations
    pub fn save_encrypted_wallet(
        &self,
        wallet: &Wallet,
        password: &SecretString,
    ) -> Result<(), String> {
        let mut conn = get_conn(self.pool())?;
        let (encrypted_data, salt, nonce) = encrypt_wallet(wallet, password)?;
        let now = chrono::Utc::now().naive_utc();

        let new_wallet = NewEncryptedWallet {
            wallet_id: self.wallet_id.clone(),
            encrypted_data,
            salt,
            nonce,
            created_at: now,
            updated_at: now,
        };

        let n = diesel::insert_into(encrypted_wallets::table)
            .values(&new_wallet)
            .on_conflict(encrypted_wallets::wallet_id)
            .do_update()
            .set((
                encrypted_wallets::encrypted_data.eq(&new_wallet.encrypted_data),
                encrypted_wallets::salt.eq(&new_wallet.salt),
                encrypted_wallets::nonce.eq(&new_wallet.nonce),
                encrypted_wallets::updated_at.eq(new_wallet.updated_at),
            ))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to save encrypted wallet: {}", e))?;
        debug!("The upserted row: {} for wallet_id: {}", n, self.wallet_id);
        Ok(())
    }

    pub fn load_encrypted_wallet(&self, password: &SecretString) -> Result<Wallet, String> {
        let mut conn = get_conn(self.pool())?;
        let encrypted_wallet: Option<EncryptedWallet> = encrypted_wallets::table
            .filter(encrypted_wallets::wallet_id.eq(&self.wallet_id))
            .first(&mut conn)
            .optional()
            .map_err(|e| format!("Failed to load encrypted wallet: {}", e))?;

        match encrypted_wallet {
            Some(enc_wallet) => {
                let wallet = decrypt_wallet(
                    &enc_wallet.encrypted_data,
                    &enc_wallet.salt,
                    &enc_wallet.nonce,
                    password,
                )?;
                Ok(wallet)
            }
            None => Err(format!(
                "No encrypted wallet found for wallet_id: {}",
                self.wallet_id
            )),
        }
    }

    // OrderWallet operations
    pub fn save_order_wallet(
        &self,
        chain_id: &str,
        seed: &str,
        relayer_config: &crate::config::RelayerEndPointConfig,
        password: &str,
    ) -> Result<(), String> {
        let new_order_wallet = DbOrderWallet::new_from_order_wallet(
            self.wallet_id.clone(),
            chain_id.to_string(),
            seed,
            relayer_config,
            password,
        )?;
        let mut conn = get_conn(self.pool())?;
        let n = diesel::insert_into(order_wallets::table)
            .values(&new_order_wallet)
            .on_conflict(order_wallets::wallet_id)
            .do_update()
            .set((
                order_wallets::chain_id.eq(&new_order_wallet.chain_id),
                order_wallets::seed_encrypted.eq(&new_order_wallet.seed_encrypted),
                order_wallets::seed_salt.eq(&new_order_wallet.seed_salt),
                order_wallets::seed_nonce.eq(&new_order_wallet.seed_nonce),
                order_wallets::relayer_api_endpoint.eq(&new_order_wallet.relayer_api_endpoint),
                order_wallets::zkos_server_endpoint.eq(&new_order_wallet.zkos_server_endpoint),
                order_wallets::relayer_program_json_path
                    .eq(&new_order_wallet.relayer_program_json_path),
                order_wallets::updated_at.eq(new_order_wallet.updated_at),
            ))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to save order wallet: {}", e))?;
        debug!("The upserted row: {} for wallet_id: {}", n, self.wallet_id);
        Ok(())
    }

    pub fn load_order_wallet(
        &self,
        password: &str,
    ) -> Result<Option<(String, String, crate::config::RelayerEndPointConfig)>, String> {
        let mut conn = get_conn(self.pool())?;
        let db_order_wallet: Option<DbOrderWallet> = order_wallets::table
            .filter(order_wallets::wallet_id.eq(&self.wallet_id))
            .filter(order_wallets::is_active.eq(true))
            .first(&mut conn)
            .optional()
            .map_err(|e| format!("Failed to load order wallet: {}", e))?;

        match db_order_wallet {
            Some(order_wallet) => {
                let seed = order_wallet.decrypt_seed(password)?;
                let relayer_config = order_wallet.to_relayer_config();
                Ok(Some((order_wallet.chain_id, seed, relayer_config)))
            }
            None => Ok(None),
        }
    }

    pub fn deactivate_order_wallet(&self) -> Result<(), String> {
        let mut conn = get_conn(self.pool())?;
        diesel::update(order_wallets::table.filter(order_wallets::wallet_id.eq(&self.wallet_id)))
            .set((
                order_wallets::is_active.eq(false),
                order_wallets::updated_at.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to deactivate order wallet: {}", e))?;

        Ok(())
    }

    // UTXO Details operations
    pub fn save_utxo_detail(
        &self,
        account_index: u64,
        utxo_detail: &twilight_client_sdk::relayer_rpcclient::method::UtxoDetailResponse,
    ) -> Result<(), String> {
        let new_utxo_detail =
            DbUtxoDetail::from_utxo_detail(self.wallet_id.clone(), account_index, utxo_detail)?;
        let mut conn = get_conn(self.pool())?;
        let n = diesel::insert_into(utxo_details::table)
            .values(&new_utxo_detail)
            .on_conflict((utxo_details::wallet_id, utxo_details::account_index))
            .do_update()
            .set((
                utxo_details::utxo_data.eq(&new_utxo_detail.utxo_data),
                utxo_details::updated_at.eq(new_utxo_detail.updated_at),
            ))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to save UTXO detail: {}", e))?;
        debug!(
            "The upserted row: {} for account_index: {}",
            n, account_index
        );
        Ok(())
    }

    pub fn load_utxo_detail(
        &self,
        account_index: u64,
    ) -> Result<Option<twilight_client_sdk::relayer_rpcclient::method::UtxoDetailResponse>, String>
    {
        let mut conn = get_conn(self.pool())?;
        let db_utxo_detail: Option<DbUtxoDetail> = utxo_details::table
            .filter(utxo_details::wallet_id.eq(&self.wallet_id))
            .filter(utxo_details::account_index.eq(account_index as i64))
            .first(&mut conn)
            .optional()
            .map_err(|e| format!("Failed to load UTXO detail: {}", e))?;

        match db_utxo_detail {
            Some(utxo_detail) => Ok(Some(utxo_detail.to_utxo_detail()?)),
            None => Ok(None),
        }
    }

    pub fn load_all_utxo_details(
        &self,
    ) -> Result<
        HashMap<u64, twilight_client_sdk::relayer_rpcclient::method::UtxoDetailResponse>,
        String,
    > {
        let mut conn = get_conn(self.pool())?;
        let db_utxo_details: Vec<DbUtxoDetail> = utxo_details::table
            .filter(utxo_details::wallet_id.eq(&self.wallet_id))
            .load(&mut conn)
            .map_err(|e| format!("Failed to load UTXO details: {}", e))?;

        let mut utxo_details_map = HashMap::new();
        for db_utxo_detail in db_utxo_details {
            let utxo_detail = db_utxo_detail.to_utxo_detail()?;
            utxo_details_map.insert(db_utxo_detail.account_index as u64, utxo_detail);
        }

        Ok(utxo_details_map)
    }

    pub fn remove_utxo_detail(&self, account_index: u64) -> Result<(), String> {
        let mut conn = get_conn(self.pool())?;
        let n = diesel::delete(
            utxo_details::table.filter(
                utxo_details::wallet_id
                    .eq(&self.wallet_id)
                    .and(utxo_details::account_index.eq(account_index as i64)),
            ),
        )
        .execute(&mut conn)
        .map_err(|e| format!("Failed to remove UTXO detail: {}", e))?;
        debug!(
            "The deleted row: {:?} for account_index: {}",
            n, account_index
        );
        Ok(())
    }

    // Request ID operations
    pub fn save_request_id(&self, account_index: u64, request_id: &str) -> Result<(), String> {
        let new_request_id = DbRequestId::new(
            self.wallet_id.clone(),
            account_index,
            request_id.to_string(),
        );
        let mut conn = get_conn(self.pool())?;
        let n = diesel::insert_into(request_ids::table)
            .values(&new_request_id)
            .on_conflict((request_ids::wallet_id, request_ids::account_index))
            .do_update()
            .set((
                request_ids::request_id.eq(&new_request_id.request_id),
                request_ids::updated_at.eq(new_request_id.updated_at),
            ))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to save request ID: {}", e))?;
        debug!(
            "The upserted row : {} for account_index: {}",
            n, account_index
        );
        Ok(())
    }

    pub fn load_request_id(&self, account_index: u64) -> Result<Option<String>, String> {
        let mut conn = get_conn(self.pool())?;
        let db_request_id: Option<DbRequestId> = request_ids::table
            .filter(request_ids::wallet_id.eq(&self.wallet_id))
            .filter(request_ids::account_index.eq(account_index as i64))
            .first(&mut conn)
            .optional()
            .map_err(|e| format!("Failed to load request ID: {}", e))?;

        Ok(db_request_id.map(|r| r.request_id))
    }

    pub fn load_all_request_ids(&self) -> Result<HashMap<u64, String>, String> {
        let mut conn = get_conn(self.pool())?;
        let db_request_ids: Vec<DbRequestId> = request_ids::table
            .filter(request_ids::wallet_id.eq(&self.wallet_id))
            .load(&mut conn)
            .map_err(|e| format!("Failed to load request IDs: {}", e))?;

        let mut request_ids_map = HashMap::new();
        for db_request_id in db_request_ids {
            request_ids_map.insert(db_request_id.account_index as u64, db_request_id.request_id);
        }

        Ok(request_ids_map)
    }

    pub fn remove_request_id(&self, account_index: u64) -> Result<(), String> {
        let mut conn = get_conn(self.pool())?;
        let n = diesel::delete(
            request_ids::table.filter(
                request_ids::wallet_id
                    .eq(&self.wallet_id)
                    .and(request_ids::account_index.eq(account_index as i64)),
            ),
        )
        .execute(&mut conn)
        .map_err(|e| format!("Failed to remove request ID: {}", e))?;
        debug!(
            "The deleted row: {:?} for account_index: {}",
            n, account_index
        );
        Ok(())
    }
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
fn encrypt_wallet(
    wallet: &Wallet,
    password: &SecretString,
) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), String> {
    // Generate salt and nonce
    let mut salt = [0u8; 32];
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce_bytes);

    // Derive key from password using PBKDF2
    let key_bytes = derive_key(password, &salt);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Serialize wallet
    let wallet_json =
        serde_json::to_vec(wallet).map_err(|e| format!("Failed to serialize wallet: {}", e))?;

    // Encrypt
    let encrypted_data = cipher
        .encrypt(nonce, wallet_json.as_ref())
        .map_err(|e| format!("Encryption failed: {}", e))?;

    Ok((encrypted_data, salt.to_vec(), nonce_bytes.to_vec()))
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
fn decrypt_wallet(
    encrypted_data: &[u8],
    salt: &[u8],
    nonce: &[u8],
    password: &SecretString,
) -> Result<Wallet, String> {
    let key_bytes = derive_key(password, salt);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce);

    let decrypted_data = cipher
        .decrypt(nonce, encrypted_data)
        .map_err(|e| format!("Decryption failed: {}", e))?;

    let wallet: Wallet = serde_json::from_slice(&decrypted_data)
        .map_err(|e| format!("Failed to deserialize wallet: {}", e))?;

    Ok(wallet)
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
fn derive_key(password: &SecretString, salt: &[u8]) -> [u8; 32] {
    // Use secure password derivation
    SecurePassword::derive_key_from_passphrase(password, salt).unwrap_or_else(|_| {
        // Fallback to simple derivation if secure method fails

        use log::info;
        info!("Fallback to simple derivation");
        let mut hasher = Sha256::default();
        hasher.update(password.expose_secret().as_bytes());
        hasher.update(salt);
        let key_bytes = hasher.finalize();
        key_bytes.into()
    })
}
