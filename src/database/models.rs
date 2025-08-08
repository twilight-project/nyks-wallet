#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::config::RelayerEndPointConfig;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::database::schema::*;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::zkos_accounts::zkaccount::ZkAccount;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use chrono::NaiveDateTime;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use diesel::prelude::*;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use serde::{Deserialize, Serialize};

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::security::SecurePassword;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use twilight_client_sdk::{relayer_rpcclient::method::UtxoDetailResponse, zkvm::IOType};

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
#[derive(Queryable, Selectable, Insertable, AsChangeset, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = zk_accounts)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DbZkAccount {
    pub id: Option<i32>,
    pub wallet_id: String,
    pub account_index: i64,
    pub qq_address: String,
    pub balance: i64,
    pub account: String,
    pub scalar: String,
    pub io_type_value: i32,
    pub on_chain: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
#[derive(Insertable, Debug)]
#[diesel(table_name = zk_accounts)]
pub struct NewDbZkAccount {
    pub wallet_id: String,
    pub account_index: i64,
    pub qq_address: String,
    pub balance: i64,
    pub account: String,
    pub scalar: String,
    pub io_type_value: i32,
    pub on_chain: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
#[derive(Queryable, Selectable, Insertable, AsChangeset, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = encrypted_wallets)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct EncryptedWallet {
    pub id: Option<i32>,
    pub wallet_id: String,
    pub encrypted_data: Vec<u8>,
    pub salt: Vec<u8>,
    pub nonce: Vec<u8>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
#[derive(Insertable, Debug)]
#[diesel(table_name = encrypted_wallets)]
pub struct NewEncryptedWallet {
    pub wallet_id: String,
    pub encrypted_data: Vec<u8>,
    pub salt: Vec<u8>,
    pub nonce: Vec<u8>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
impl DbZkAccount {
    pub fn from_zk_account(zk_account: &ZkAccount, wallet_id: String) -> NewDbZkAccount {
        let now = chrono::Utc::now().naive_utc();
        NewDbZkAccount {
            wallet_id,
            account_index: zk_account.index as i64,
            qq_address: zk_account.qq_address.clone(),
            balance: zk_account.balance as i64,
            account: zk_account.account.clone(),
            scalar: zk_account.scalar.clone(),
            io_type_value: zk_account.io_type.clone() as i32,
            on_chain: zk_account.on_chain,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn to_zk_account(&self) -> Result<ZkAccount, String> {
        let io_type = match self.io_type_value {
            0 => IOType::Coin,
            1 => IOType::Memo,
            _ => return Err(format!("Invalid io_type_value: {}", self.io_type_value)),
        };

        Ok(ZkAccount {
            qq_address: self.qq_address.clone(),
            balance: self.balance as u64,
            account: self.account.clone(),
            scalar: self.scalar.clone(),
            index: self.account_index as u64,
            io_type,
            on_chain: self.on_chain,
        })
    }

    pub fn update_from_zk_account(&mut self, zk_account: &ZkAccount) {
        self.balance = zk_account.balance as i64;
        self.io_type_value = zk_account.io_type.clone() as i32;
        self.on_chain = zk_account.on_chain;
        self.updated_at = chrono::Utc::now().naive_utc();
    }
}

// OrderWallet related models
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
#[derive(Queryable, Selectable, Insertable, AsChangeset, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = order_wallets)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DbOrderWallet {
    pub id: Option<i32>,
    pub wallet_id: String,
    pub chain_id: String,
    pub seed_encrypted: Vec<u8>,
    pub seed_salt: Vec<u8>,
    pub seed_nonce: Vec<u8>,
    pub relayer_api_endpoint: String,
    pub zkos_server_endpoint: String,
    pub relayer_program_json_path: String,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
#[derive(Insertable, Debug)]
#[diesel(table_name = order_wallets)]
pub struct NewDbOrderWallet {
    pub wallet_id: String,
    pub chain_id: String,
    pub seed_encrypted: Vec<u8>,
    pub seed_salt: Vec<u8>,
    pub seed_nonce: Vec<u8>,
    pub relayer_api_endpoint: String,
    pub zkos_server_endpoint: String,
    pub relayer_program_json_path: String,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
#[derive(Queryable, Selectable, Insertable, AsChangeset, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = utxo_details)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DbUtxoDetail {
    pub id: Option<i32>,
    pub wallet_id: String,
    pub account_index: i64,
    pub utxo_data: String, // JSON serialized
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
#[derive(Insertable, Debug)]
#[diesel(table_name = utxo_details)]
pub struct NewDbUtxoDetail {
    pub wallet_id: String,
    pub account_index: i64,
    pub utxo_data: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
#[derive(Queryable, Selectable, Insertable, AsChangeset, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = request_ids)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DbRequestId {
    pub id: Option<i32>,
    pub wallet_id: String,
    pub account_index: i64,
    pub request_id: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
#[derive(Insertable, Debug)]
#[diesel(table_name = request_ids)]
pub struct NewDbRequestId {
    pub wallet_id: String,
    pub account_index: i64,
    pub request_id: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
impl DbOrderWallet {
    pub fn new_from_order_wallet(
        wallet_id: String,
        chain_id: String,
        seed: &str,
        relayer_config: &RelayerEndPointConfig,
        password: &str,
    ) -> Result<NewDbOrderWallet, String> {
        use secrecy::SecretString;

        // Encrypt the seed
        let password_secret = SecretString::new(password.to_string());
        let (encrypted_seed, salt, nonce) = Self::encrypt_seed(seed, &password_secret)?;

        let now = chrono::Utc::now().naive_utc();

        Ok(NewDbOrderWallet {
            wallet_id,
            chain_id,
            seed_encrypted: encrypted_seed,
            seed_salt: salt,
            seed_nonce: nonce,
            relayer_api_endpoint: relayer_config.relayer_api_endpoint.clone(),
            zkos_server_endpoint: relayer_config.zkos_server_endpoint.clone(),
            relayer_program_json_path: relayer_config.relayer_program_json_path.clone(),
            is_active: true,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn decrypt_seed(&self, password: &str) -> Result<String, String> {
        use secrecy::SecretString;

        let password_secret = SecretString::new(password.to_string());
        Self::decrypt_seed_internal(
            &self.seed_encrypted,
            &self.seed_salt,
            &self.seed_nonce,
            &password_secret,
        )
    }

    pub fn to_relayer_config(&self) -> RelayerEndPointConfig {
        RelayerEndPointConfig::new(
            self.relayer_api_endpoint.clone(),
            self.zkos_server_endpoint.clone(),
            self.relayer_program_json_path.clone(),
        )
    }

    fn encrypt_seed(
        seed: &str,
        password: &secrecy::SecretString,
    ) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), String> {
        use aes_gcm::{
            Aes256Gcm, Key, Nonce,
            aead::{Aead, KeyInit, OsRng},
        };
        use rand_core::RngCore;

        // Generate salt and nonce
        let mut salt = [0u8; 32];
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut salt);
        OsRng.fill_bytes(&mut nonce_bytes);

        // Derive key
        let key_bytes = SecurePassword::derive_key_from_passphrase(password, &salt)
            .map_err(|e| format!("Key derivation failed: {}", e))?;
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt seed
        let encrypted_data = cipher
            .encrypt(nonce, seed.as_bytes())
            .map_err(|e| format!("Encryption failed: {}", e))?;

        Ok((encrypted_data, salt.to_vec(), nonce_bytes.to_vec()))
    }

    fn decrypt_seed_internal(
        encrypted_data: &[u8],
        salt: &[u8],
        nonce: &[u8],
        password: &secrecy::SecretString,
    ) -> Result<String, String> {
        use aes_gcm::{
            Aes256Gcm, Key, Nonce,
            aead::{Aead, KeyInit},
        };

        let key_bytes = SecurePassword::derive_key_from_passphrase(password, salt)
            .map_err(|e| format!("Key derivation failed: {}", e))?;
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(nonce);

        let decrypted_data = cipher
            .decrypt(nonce, encrypted_data)
            .map_err(|e| format!("Decryption failed: {}", e))?;

        String::from_utf8(decrypted_data).map_err(|e| format!("UTF-8 conversion failed: {}", e))
    }
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
impl DbUtxoDetail {
    pub fn from_utxo_detail(
        wallet_id: String,
        account_index: u64,
        utxo_detail: &UtxoDetailResponse,
    ) -> Result<NewDbUtxoDetail, String> {
        let utxo_data = serde_json::to_string(utxo_detail)
            .map_err(|e| format!("Failed to serialize UTXO detail: {}", e))?;

        let now = chrono::Utc::now().naive_utc();

        Ok(NewDbUtxoDetail {
            wallet_id,
            account_index: account_index as i64,
            utxo_data,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn to_utxo_detail(&self) -> Result<UtxoDetailResponse, String> {
        serde_json::from_str(&self.utxo_data)
            .map_err(|e| format!("Failed to deserialize UTXO detail: {}", e))
    }
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
impl DbRequestId {
    pub fn new(wallet_id: String, account_index: u64, request_id: String) -> NewDbRequestId {
        let now = chrono::Utc::now().naive_utc();

        NewDbRequestId {
            wallet_id,
            account_index: account_index as i64,
            request_id,
            created_at: now,
            updated_at: now,
        }
    }
}
