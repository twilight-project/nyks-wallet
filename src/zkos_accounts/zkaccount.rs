use super::encrypted_account::{EncryptedAccount, KeyManager};
use curve25519_dalek::scalar::Scalar;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use twilight_client_sdk::{
    quisquislib::{Account, ElGamalCommitment, keys::PublicKey, ristretto::RistrettoPublicKey},
    zkvm::IOType,
};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZkAccount {
    pub qq_address: String,
    pub balance: u64,
    pub account: String,
    pub scalar: String,
    pub index: u64,
    pub io_type: IOType,
}
impl ZkAccount {
    pub fn new(
        qq_address: String,
        balance: u64,
        account: String,
        scalar: String,
        index: u64,
    ) -> Self {
        Self {
            qq_address,
            balance,
            account,
            scalar,
            index,
            io_type: IOType::Coin,
        }
    }

    pub fn from_seed(index: u64, seed: String, balance: u64) -> Self {
        let key_manager = KeyManager::from_cosmos_signature(seed.as_bytes());

        let secret_key = key_manager.derive_child_key(index);
        let pk_in = RistrettoPublicKey::from_secret_key(&secret_key, &mut OsRng);

        let rscalar = Scalar::random(&mut OsRng);
        let rscalar_str = hex::encode(rscalar.to_bytes());
        let commit_in =
            ElGamalCommitment::generate_commitment(&pk_in, rscalar.clone(), Scalar::from(balance));

        let coin_acc = Account::set_account(pk_in.clone(), commit_in.clone());
        let qq_address: EncryptedAccount = EncryptedAccount::from(coin_acc);
        let account: String = qq_address.get_address();
        let qq_address_str: String = qq_address.to_hex_str();

        Self::new(qq_address_str, balance, account, rscalar_str, index)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZkAccountDB {
    pub accounts: HashMap<u64, ZkAccount>,
    pub index: u64,
}
impl ZkAccountDB {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            index: 0,
        }
    }
    pub fn add_account(&mut self, account: ZkAccount) -> Option<ZkAccount> {
        let result = self.accounts.insert(self.index, account);
        self.index += 1;
        result
    }
    pub fn generate_new_account(&mut self, balance: u64, seed: String) -> Result<u64, String> {
        let zk_account = ZkAccount::from_seed(self.index, seed, balance);
        self.add_account(zk_account);
        Ok(self.index - 1)
    }
    pub fn try_add_account(&mut self, account: ZkAccount) -> Result<u64, String> {
        if self.accounts.contains_key(&account.index) {
            return Err(format!(
                "Account with index {} already exists",
                account.index
            ));
        }
        self.accounts.insert(self.index, account);
        self.index += 1;
        Ok(self.index)
    }

    pub fn get_account(&self, index: &u64) -> Option<ZkAccount> {
        self.accounts.get(index).cloned()
    }
    pub fn get_mut_account(&mut self, index: &u64) -> Option<&mut ZkAccount> {
        self.accounts.get_mut(index)
    }
    pub fn remove_account(&mut self, index: &u64) {
        self.accounts.remove(index);
    }
    pub fn get_all_accounts(&self) -> Vec<&ZkAccount> {
        self.accounts.values().collect()
    }
    pub fn get_all_accounts_as_json(&self) -> String {
        serde_json::to_string(&self.accounts).unwrap()
    }
    pub fn import_from_json(path: &str) -> Result<ZkAccountDB, String> {
        let json = match std::fs::read_to_string(path) {
            Ok(json) => json,
            Err(e) => return Err(format!("Failed to read file: {}", e)),
        };
        let zk_accounts_db: ZkAccountDB = match serde_json::from_str(&json) {
            Ok(zk_accounts_db) => zk_accounts_db,
            Err(e) => return Err(format!("Failed to parse json: {}", e)),
        };
        Ok(zk_accounts_db)
    }
    pub fn get_balance(&self, index: &u64) -> Option<u64> {
        self.accounts.get(index).map(|account| account.balance)
    }
    pub fn update_balance(&mut self, index: &u64, balance: u64) -> Result<(), String> {
        if !self.accounts.contains_key(index) {
            return Err(format!("Account with index {} does not exist", index));
        }
        self.accounts.get_mut(index).unwrap().balance = balance;
        Ok(())
    }
    pub fn export_to_json(&self, path: &str) -> Result<(), String> {
        match serde_json::to_string(&self) {
            Ok(json) => match std::fs::write(path, json) {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Failed to export to json: {}", e)),
            },
            Err(e) => Err(format!("Failed to export to json: {}", e)),
        }
    }
    pub fn try_export_to_json(&self, path: &str) -> Result<(), String> {
        match serde_json::to_string(&self) {
            Ok(json) => {
                // Check if file exists and rename to filename_old if it does
                if std::path::Path::new(path).exists() {
                    let old_path =
                        format!("{}_{}", path, chrono::Local::now().format("%Y%m%d_%H%M%S"));
                    if let Err(e) = std::fs::rename(path, &old_path) {
                        return Err(format!("Failed to rename existing file: {}", e));
                    }
                }
                match std::fs::write(path, json) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("Failed to export to json: {}", e)),
                }
            }
            Err(e) => Err(format!("Failed to export to json: {}", e)),
        }
    }
}
