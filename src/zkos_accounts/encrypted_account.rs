use address::{AddressType, Network};
use core::convert::TryInto;
use curve25519_dalek::scalar::Scalar;
use rand::rngs::OsRng;
use serde::Deserialize;
use serde::Serialize;
use sha2::{Digest, Sha512};
use twilight_client_sdk::{
    quisquislib::{
        self, Account, ElGamalCommitment, RistrettoPublicKey, RistrettoSecretKey, keys::SecretKey,
    },
    util,
    zkvm::{
        Address,
        zkos_types::{IOType, Output, OutputCoin, OutputData},
    },
};

/// The constant message that must be signed by the user's Cosmos wallet.
/// The resulting signature is used as the master seed for all Ristretto key derivations.
pub const DERIVATION_MESSAGE: &str = "This signature is for deriving the master Twilight ZkOS Ristretto key. Version: 1. Do not share this signature.";

/// Manages the deterministic derivation of Ristretto keys from a master seed.
///
/// This struct holds the master key in memory for the duration of a session
/// and should be created upon wallet unlock by providing a signature from a
/// primary wallet (e.g., Cosmos).
pub struct KeyManager {
    master_key: RistrettoSecretKey,
}

impl KeyManager {
    /// Creates a new `KeyManager` by deriving a master key from a Cosmos signature.
    /// The signature should be from signing the constant `DERIVATION_MESSAGE`.
    pub fn from_cosmos_signature(cosmos_signature_bytes: &[u8]) -> Self {
        Self {
            master_key: derive_master_ristretto_key(cosmos_signature_bytes),
        }
    }

    /// Derives a child key for a specific account index using an HD wallet pattern.
    pub fn derive_child_key(&self, account_index: u32) -> RistrettoSecretKey {
        derive_child_key(&self.master_key, account_index)
    }
}

/// Derives the single, master Ristretto secret key from a user's Cosmos signature.
/// This function is deterministic. The same signature will always produce the same key.
fn derive_master_ristretto_key(cosmos_signature_bytes: &[u8]) -> RistrettoSecretKey {
    // The `from_bytes` function internally uses a strong hash (KDF), which is exactly
    // what we need to securely convert the signature into a key.
    SecretKey::from_bytes(cosmos_signature_bytes)
}

/// Derives a child key from a master key and an account index.
/// This creates a simple Hierarchical Deterministic (HD) path.
fn derive_child_key(master_key: &RistrettoSecretKey, account_index: u32) -> RistrettoSecretKey {
    let mut hasher = Sha512::new();

    // Hash the master key bytes concatenated with a domain separator and the account index.
    hasher.update(master_key.as_bytes());
    hasher.update(b"twilight_child_key"); // Domain separation constant
    hasher.update(&account_index.to_le_bytes());

    // The SecretKey::from_bytes function will perform another hash, which is fine.
    // It ensures the result is a valid key in the group.
    SecretKey::from_bytes(&hasher.finalize())
}

// convert the hex string into a RistrettoPublicKey
pub fn public_key_from_hex(hex_str: String) -> Result<RistrettoPublicKey, &'static str> {
    let bytes = match hex::decode(hex_str) {
        Ok(bytes) => bytes,
        Err(_) => return Err("Error decoding hex string"),
    };
    match <RistrettoPublicKey as quisquislib::keys::PublicKey>::from_bytes(&bytes) {
        Ok(pk) => Ok(pk),
        Err(_) => Err("Error converting bytes to RistrettoPublicKey"),
    }
}

// ------- ChainAccount Representation ------- //
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedAccount {
    pub(crate) address: String,            // Hex String
    pub(crate) encrypt: ElGamalCommitment, // ElGamal Encryption of the amount
}

impl EncryptedAccount {
    pub fn new(address: String, encrypt: ElGamalCommitment) -> Self {
        Self { address, encrypt }
    }
    //encode the EncryptedAccount into a hex string for storage on chain
    //convert account to bare bytes and then encode the complete sequence to hex
    pub fn to_hex_str(&self) -> String {
        //reconstruct the Address from adress hex string to recreate bytes
        // to match the chain encoding
        let address: Address = Address::from_hex(&self.address, AddressType::Standard).unwrap();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&address.as_bytes());
        bytes.extend_from_slice(&self.encrypt.to_bytes());
        let hex = hex::encode(bytes);
        hex
    }

    //decode the hex string into a EncryptedAccount with standard address
    pub fn from_hex_str(hex_str: String) -> Result<Self, &'static str> {
        let bytes = hex::decode(hex_str).unwrap();
        //let standard_address = Standard::from_bytes(&bytes[0..69]).unwrap();
        //let address = Address::Standard(standard_address);
        let address = Address::from_hex(&hex::encode(&bytes[0..69]), AddressType::Standard)?;
        let encrypt = ElGamalCommitment::from_bytes(&bytes[69..])?;
        Ok(Self {
            address: address.as_hex(),
            encrypt,
        })
    }
    // utility function to support the wallet monitoring
    pub fn verify_keypair(&self, sk: &RistrettoSecretKey) -> bool {
        //recreate account
        let account: Account = EncryptedAccount::into(self.clone());
        if account.verify_account_keypair(sk).is_ok() {
            true
        } else {
            false
        }
    }
    pub fn get_address(&self) -> String {
        self.address.clone()
    }
    pub fn get_encrypt(&self) -> ElGamalCommitment {
        self.encrypt.clone()
    }
}
// create EncryptedAccount from Taditional quisquis Account
impl From<Account> for EncryptedAccount {
    fn from(account: Account) -> Self {
        let (pk, encrypt) = account.get_account();
        let address = Address::standard_address(Network::default(), pk);
        Self {
            address: address.as_hex(),
            encrypt,
        }
    }
}
// Implement the Into trait for your custom struct
// convert the Zkos account into a traditional QuisQuis Account
impl Into<Account> for EncryptedAccount {
    fn into(self) -> Account {
        let address = Address::from_hex(&self.address, AddressType::Standard).unwrap();
        let pub_key: RistrettoPublicKey = address.into();
        let encrypt = self.encrypt.clone();
        let account = Account::set_account(pub_key, encrypt);
        account
    }
}
impl Into<Output> for EncryptedAccount {
    fn into(self) -> Output {
        let encrypt = self.encrypt.clone();
        let output: Output = Output::coin(OutputData::coin(OutputCoin::new(
            encrypt,
            self.address.clone(),
        )));
        output
    }
}

impl From<Output> for EncryptedAccount {
    fn from(output: Output) -> Self {
        //check output type.
        //This only works for Coin Output
        match output.out_type {
            IOType::Coin => {
                let out_coin = output.output.get_output_coin().unwrap().to_owned();
                let address = out_coin.owner.clone();
                EncryptedAccount::new(address, out_coin.encrypt.clone())
            }
            _ => panic!("Invalid Output. Expected Coin type"),
        }
    }
}

/// Verify Public/Private Keypair.
/// Returns true iff the EncryptedAccount public key corresponds to the provided private key.
pub fn verify_keypair_encrypted_account(
    sk: &RistrettoSecretKey,
    acc_hex: String,
) -> Result<bool, &'static str> {
    //recreate EncryptedAccount
    let acc = EncryptedAccount::from_hex_str(acc_hex)?;
    // recreate traditional quisquis Account
    let account: Account = EncryptedAccount::into(acc.clone());
    if account.verify_account_keypair(&sk).is_ok() {
        Ok(true)
    } else {
        Ok(false)
    }
}

///Verify EncryptedAccount.
/// Returns true iff the public key corresponds to the provided private key
/// and the account balance commitment is equal to the provided balance.
///
pub fn verify_encrypted_account(
    sk: &RistrettoSecretKey,
    acc_hex: String,
    balance: u32,
) -> Result<bool, &'static str> {
    //recreate account
    let encrypted_acc = EncryptedAccount::from_hex_str(acc_hex)?;
    let acc: Account = EncryptedAccount::into(encrypted_acc.clone());
    if acc.verify_account(&sk, balance.into()).is_ok() {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Generate Zero balance EncryptedAccount with the provided hex address
/// Input is hex address as string
/// Output is EncryptedAccount as Hex string
pub fn generate_zero_balance_encrypted_account_from_address(
    address_hex: String,
) -> Result<String, &'static str> {
    // Parse the string of data into a RistrettoPublicKey object.
    let address: Address = Address::from_hex(&address_hex, AddressType::Standard)?;
    let pk: RistrettoPublicKey = address.into();
    let comm_scalar = Scalar::random(&mut OsRng);
    let comm = ElGamalCommitment::generate_commitment(&pk, comm_scalar, Scalar::zero());

    let chain_account = EncryptedAccount::new(address_hex, comm);

    return Ok(chain_account.to_hex_str());
}

/// Generate zero balance Encrypted account with the provided key
/// Input : pk as Hex String
/// Output : EncryptedAccount as Hex String

pub fn generate_zero_encrypted_account_from_key(pk: String) -> Result<String, &'static str> {
    // Parse the string of data into a RistrettoPublicKey object.
    let pk = public_key_from_hex(pk)?;
    let comm_scalar = Scalar::random(&mut OsRng);
    let comm = ElGamalCommitment::generate_commitment(&pk, comm_scalar, Scalar::zero());

    let chain_account = EncryptedAccount::new(
        Address::standard_address(Network::default(), pk).as_hex(),
        comm,
    );

    return Ok(chain_account.to_hex_str());
}

/// Generate EncryptedAccount with balance in the hex string format
/// Input @pk : Public Key as Hex String
/// Input @balance : Balance as u64
/// Input @r_scalar : Random Scalar as Hex String. used for creating the ecryption
/// Output : EncryptedAccount as Hex String
pub fn generate_encrypted_account_with_balance(
    pk: String,
    balance: u64,
    r_scalar: String,
) -> Result<String, &'static str> {
    // Parse the string of data into a RistrettoPublicKey object.
    let pk = public_key_from_hex(pk)?;

    let comm_scalar = match util::hex_to_scalar(r_scalar) {
        Some(scalar) => scalar,
        None => return Err("Error converting hex string to Scalar"),
    };

    let comm = ElGamalCommitment::generate_commitment(&pk, comm_scalar, Scalar::from(balance));

    let chain_account = EncryptedAccount::new(
        Address::standard_address(Network::default(), pk).as_hex(),
        comm,
    );

    return Ok(chain_account.to_hex_str());
}
/// Decrypt EncryptedAccount
/// Returns balance iff the public key corresponds to the provided private key and the encryption is valid.
///
/// # Parameters
/// - `sk`: The `RistrettoSecretKey` corresponding to the encrypted account.
/// - `zk_acc_hex`: The hex-encoded `EncryptedAccount` to decrypt.
///
/// # Returns
/// The decrypted balance as a `u64`.
pub fn decrypt_encrypted_account_value(
    sk: &RistrettoSecretKey,
    zk_acc_hex: String,
) -> Result<u64, &'static str> {
    //recreate zkosAccount
    let trading_acc = EncryptedAccount::from_hex_str(zk_acc_hex)?;
    // get O.G Quisquis account
    let account: Account = trading_acc.into();
    //get balance
    let balance = account.decrypt_account_balance_value(&sk).unwrap();
    //convert balance into u64
    let scalar_bytes = balance.to_bytes();
    // Convert [u8; 32] into [u8; 8]
    let array_8: [u8; 8] = scalar_bytes[0..8].try_into().unwrap();
    Ok(u64::from_le_bytes(array_8))
}

/// getAddressFromEncryptedAccountHex
///
pub fn get_hex_address_from_encrypted_account_hex(acc_hex: String) -> Result<String, &'static str> {
    let account = EncryptedAccount::from_hex_str(acc_hex)?;
    Ok(account.address)
}

/// create Output from EncryptedAccount
/// Input @account : EncryptedAccount as Hex String
/// Returns Output as Json String Object.
///
pub fn create_output_for_coin_from_encrypted_account(
    account: String,
) -> Result<String, &'static str> {
    let acc = EncryptedAccount::from_hex_str(account)?;

    let output: Output = acc.into();
    match serde_json::to_string(&output) {
        Ok(str) => Ok(str),
        Err(_) => return Err("Error creating Output Json string"),
    }
}

/// create EncryptedAccount from Output (Coin)
/// Input @output : Output as Json String Object.
/// Returns EncryptedAccount as hex string.
///
pub fn extract_encrypted_account_from_output_coin(output: String) -> String {
    let out: Output = serde_json::from_str(&output).unwrap();

    let account: EncryptedAccount = EncryptedAccount::from(out.clone());
    account.to_hex_str()
}
