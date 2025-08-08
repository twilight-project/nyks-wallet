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
use twilight_client_sdk::zkvm::IOType;

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
