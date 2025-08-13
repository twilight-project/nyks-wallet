use super::encrypted_account::{EncryptedAccount, KeyManager};
use address::Network;
use curve25519_dalek::scalar::Scalar;
use rand::rngs::OsRng;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use twilight_client_sdk::{
    quisquislib::{
        Account, ElGamalCommitment, RistrettoSecretKey, keys::PublicKey,
        ristretto::RistrettoPublicKey,
    },
    zkvm::{IOType, Input, Utxo},
};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZkAccount {
    pub qq_address: String,
    pub balance: u64,
    pub account: String,
    pub scalar: String,
    pub index: u64,
    pub io_type: IOType,
    pub on_chain: bool,
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
            on_chain: false,
        }
    }

    pub fn from_seed(index: u64, seed: &SecretString, balance: u64) -> Result<Self, String> {
        let key_manager = KeyManager::from_cosmos_signature(seed.expose_secret().as_bytes());

        let secret_key = key_manager.derive_child_key(index);
        let pk_in = RistrettoPublicKey::from_secret_key(&secret_key, &mut OsRng);

        let rscalar = Scalar::random(&mut OsRng);
        let rscalar_str = hex::encode(rscalar.to_bytes());
        let commit_in =
            ElGamalCommitment::generate_commitment(&pk_in, rscalar.clone(), Scalar::from(balance));

        let coin_acc = Account::set_account(pk_in.clone(), commit_in.clone());
        let qq_address: EncryptedAccount = EncryptedAccount::from(coin_acc);
        let account: String = qq_address.get_address();
        let qq_address_str: String = qq_address.to_hex_str().map_err(|e| e.to_string())?;

        Ok(Self::new(
            qq_address_str,
            balance,
            account,
            rscalar_str,
            index,
        ))
    }
    pub fn get_seed(&self, master_seed: &str) -> RistrettoSecretKey {
        let key_manager = KeyManager::from_cosmos_signature(master_seed.as_bytes());
        let secret_key = key_manager.derive_child_key(self.index);
        secret_key
    }
    pub fn get_qq_address(&self) -> Result<EncryptedAccount, String> {
        EncryptedAccount::from_hex_str(self.qq_address.clone()).map_err(|e| e.to_string())
    }
    pub fn get_new_account_input(&self) -> Result<Input, String> {
        let input = Input::input_from_quisquis_account(
            &self.get_qq_address()?.into(),
            Utxo::default(),
            0,
            Network::default(),
        );
        Ok(input)
    }
    pub fn get_input_string(&self) -> Result<String, String> {
        let input = self.get_new_account_input()?;
        serde_json::to_string(&input).map_err(|e| e.to_string())
    }
    pub fn get_scalar(&self) -> Result<Scalar, String> {
        let scalar = twilight_client_sdk::util::hex_to_scalar(self.scalar.clone())
            .ok_or("Failed to convert scalar_hex to scalar")?;
        Ok(scalar)
    }
    pub fn get_qq_account(&self) -> Result<Account, String> {
        let qq_address = self.get_qq_address()?;
        let qq_account = qq_address.into();
        Ok(qq_account)
    }
    pub fn get_qq_str(&self, account: Account) -> Result<String, String> {
        let qq_address: EncryptedAccount = EncryptedAccount::from(account);
        qq_address.to_hex_str().map_err(|e| e.to_string())
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
    pub fn generate_new_account(
        &mut self,
        balance: u64,
        seed: &SecretString,
    ) -> Result<u64, String> {
        let zk_account = ZkAccount::from_seed(self.index, seed, balance)?;
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
    pub fn get_account_address(&self, index: &u64) -> Result<String, String> {
        let account = self
            .accounts
            .get(index)
            .map(|account| account.account.clone());
        match account {
            Some(account) => Ok(account),
            None => Err(format!("Account with index {} does not exist", index)),
        }
    }
    pub fn get_account(&self, index: &u64) -> Result<ZkAccount, String> {
        match self.accounts.get(index).cloned() {
            Some(account) => Ok(account),
            None => Err(format!("Account with index {} does not exist", index)),
        }
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
    pub fn get_all_accounts_as_json(&self) -> Result<String, String> {
        serde_json::to_string(&self.accounts).map_err(|e| e.to_string())
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
        self.accounts
            .get_mut(index)
            .ok_or(format!("Account with index {} does not exist", index))?
            .balance = balance;
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
    pub fn update_io_type(&mut self, index: &u64, io_type: IOType) -> Result<(), String> {
        // if !self.accounts.contains_key(&index) {
        //     return Err(format!("Account with index {} does not exist", index));
        // }
        self.accounts
            .get_mut(index)
            .ok_or(format!("Account with index {} does not exist", index))?
            .io_type = io_type;
        Ok(())
    }
    pub fn update_scalar(&mut self, index: &u64, scalar: &str) -> Result<(), String> {
        self.accounts
            .get_mut(index)
            .ok_or(format!("Account with index {} does not exist", index))?
            .scalar = scalar.to_string();
        Ok(())
    }
    pub fn update_account_key(&mut self, index: &u64, account_key: &str) -> Result<(), String> {
        self.accounts
            .get_mut(index)
            .ok_or(format!("Account with index {} does not exist", index))?
            .account = account_key.to_string();
        Ok(())
    }
    pub fn update_on_chain(&mut self, index: &u64, on_chain: bool) -> Result<(), String> {
        // if !self.accounts.contains_key(&index) {
        //     return Err(format!("Account with index {} does not exist", index));
        // }
        self.accounts
            .get_mut(index)
            .ok_or(format!("Account with index {} does not exist", index))?
            .on_chain = on_chain;
        Ok(())
    }
    pub fn update_qq_account(&mut self, index: &u64, account: Account) -> Result<(), String> {
        // if !self.accounts.contains_key(&index) {
        //     return Err(format!("Account with index {} does not exist", index));
        // }
        let qq_address: EncryptedAccount = EncryptedAccount::from(account);
        let qq_str = qq_address.to_hex_str().map_err(|e| e.to_string())?;
        self.accounts
            .get_mut(index)
            .ok_or(format!("Account with index {} does not exist", index))?
            .qq_address = qq_str;
        Ok(())
    }
    pub fn remove_account_by_index(&mut self, index: &u64) -> Result<(), String> {
        if !self.accounts.contains_key(&index) {
            return Err(format!("Account with index {} does not exist", index));
        }
        self.accounts.remove(index);
        Ok(())
    }
}
