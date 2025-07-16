use curve25519_dalek::scalar::Scalar;
use quisquislib::keys::{PublicKey, SecretKey};
use quisquislib::ristretto::RistrettoPublicKey;
use quisquislib::ristretto::RistrettoSecretKey;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use twilight_client_sdk::encrypted_account::KeyManager; // correct path
use twilight_client_sdk::encrypted_account::{self, EncryptedAccount};
use twilight_client_sdk::quisquislib;
use twilight_client_sdk::quisquislib::Account;
use twilight_client_sdk::quisquislib::ElGamalCommitment;

#[derive(Deserialize, Serialize, Debug)]
pub struct ZkAccount {
    pub qq_address: String,
    pub balance: u64,
    pub account: String,
    pub scalar: String,
}
impl ZkAccount {
    pub fn new(qq_address: String, balance: u64, account: String, scalar: String) -> Self {
        Self {
            qq_address,
            balance,
            account,
            scalar,
        }
    }

    pub fn from_seed(index: u32, seed: String, balance: u64) -> Self {
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

        Self::new(qq_address_str, balance, account, rscalar_str)
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ZkAccountDB {
    pub accounts: HashMap<String, ZkAccount>,
}

#[cfg(test)]
mod tests {

    use crate::Wallet;

    use super::*;

    #[test]
    fn test_zkaccount_from_seed() {
        // Create a mock 64-byte signature/seed
        let wallet = Wallet::import_from_json("test.json");

        let mock_seed = "mDbWcSVCauGEhYH15ubwa8fer14iud/bL2nR6KcofD5Plm7Ebrwv4VbU8eQiB0n0Mh7R4ZnPyPylqBUga+3S0g==";

        let index = 0;
        let balance = 40000;

        // Create ZkAccount from seed
        let zk_account = ZkAccount::from_seed(index, mock_seed.to_string(), balance);
        println!("{:?}", zk_account);
    }
}
