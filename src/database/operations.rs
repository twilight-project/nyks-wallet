#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::database::{
    connection::DbConnection,
    models::{DbZkAccount, EncryptedWallet, NewEncryptedWallet},
    schema::{encrypted_wallets, zk_accounts},
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
use std::collections::HashMap;

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseManager {
    wallet_id: String,
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
impl DatabaseManager {
    pub fn new(wallet_id: String) -> Self {
        Self { wallet_id }
    }

    pub fn get_wallet_id(&self) -> &str {
        &self.wallet_id
    }

    // ZkAccount operations
    pub fn save_zk_account(
        &self,
        conn: &mut DbConnection,
        zk_account: &ZkAccount,
    ) -> Result<(), String> {
        let new_account = DbZkAccount::from_zk_account(zk_account, self.wallet_id.clone());

        diesel::insert_into(zk_accounts::table)
            .values(&new_account)
            .on_conflict((zk_accounts::wallet_id, zk_accounts::account_index))
            .do_update()
            .set((
                zk_accounts::balance.eq(new_account.balance),
                zk_accounts::io_type_value.eq(new_account.io_type_value),
                zk_accounts::on_chain.eq(new_account.on_chain),
                zk_accounts::updated_at.eq(new_account.updated_at),
            ))
            .execute(conn)
            .map_err(|e| format!("Failed to save zk_account: {}", e))?;

        Ok(())
    }

    pub fn update_zk_account(
        &self,
        conn: &mut DbConnection,
        zk_account: &ZkAccount,
    ) -> Result<(), String> {
        let now = chrono::Utc::now().naive_utc();

        diesel::update(
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
        .execute(conn)
        .map_err(|e| format!("Failed to update zk_account: {}", e))?;

        Ok(())
    }

    pub fn remove_zk_account(
        &self,
        conn: &mut DbConnection,
        account_index: u64,
    ) -> Result<(), String> {
        diesel::delete(
            zk_accounts::table.filter(
                zk_accounts::wallet_id
                    .eq(&self.wallet_id)
                    .and(zk_accounts::account_index.eq(account_index as i64)),
            ),
        )
        .execute(conn)
        .map_err(|e| format!("Failed to remove zk_account: {}", e))?;

        Ok(())
    }

    pub fn load_all_zk_accounts(
        &self,
        conn: &mut DbConnection,
    ) -> Result<HashMap<u64, ZkAccount>, String> {
        let db_accounts: Vec<DbZkAccount> = zk_accounts::table
            .filter(zk_accounts::wallet_id.eq(&self.wallet_id))
            .load(conn)
            .map_err(|e| format!("Failed to load zk_accounts: {}", e))?;

        let mut accounts = HashMap::new();
        for db_account in db_accounts {
            let zk_account = db_account.to_zk_account()?;
            accounts.insert(zk_account.index, zk_account);
        }

        Ok(accounts)
    }

    // Wallet encryption operations
    pub fn save_encrypted_wallet(
        &self,
        conn: &mut DbConnection,
        wallet: &Wallet,
        password: &SecretString,
    ) -> Result<(), String> {
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

        diesel::insert_into(encrypted_wallets::table)
            .values(&new_wallet)
            .on_conflict(encrypted_wallets::wallet_id)
            .do_update()
            .set((
                encrypted_wallets::encrypted_data.eq(&new_wallet.encrypted_data),
                encrypted_wallets::salt.eq(&new_wallet.salt),
                encrypted_wallets::nonce.eq(&new_wallet.nonce),
                encrypted_wallets::updated_at.eq(new_wallet.updated_at),
            ))
            .execute(conn)
            .map_err(|e| format!("Failed to save encrypted wallet: {}", e))?;

        Ok(())
    }

    pub fn load_encrypted_wallet(
        &self,
        conn: &mut DbConnection,
        password: &SecretString,
    ) -> Result<Option<Wallet>, String> {
        let encrypted_wallet: Option<EncryptedWallet> = encrypted_wallets::table
            .filter(encrypted_wallets::wallet_id.eq(&self.wallet_id))
            .first(conn)
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
                Ok(Some(wallet))
            }
            None => Ok(None),
        }
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
        let mut hasher = Sha256::new();
        hasher.update(password.expose_secret().as_bytes());
        hasher.update(salt);
        let key_bytes = hasher.finalize();
        key_bytes.into()
    })
}
