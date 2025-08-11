use crate::config::WalletEndPointConfig;
use crate::security::print_secret_to_tty;
// use crate::nyks_rpc::rpcclient::txrequest::NYKS_LCD_BASE_URL;
use crate::{faucet::*, generate_seed};
use anyhow::anyhow;
use bip32::{DerivationPath, XPrv};
use bip39::{Error as Bip39Error, Language as B39Lang, Mnemonic};
use cosmrs::AccountId;
use cosmrs::crypto::{PublicKey, secp256k1::SigningKey};
use log::{debug, error, info};
use reqwest::Client;
use ripemd::Ripemd160;
use rpassword::prompt_password;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::time::{Duration, sleep};
use zeroize::ZeroizeOnDrop;
pub const BECH_PREFIX: &str = "twilight";
pub type NYKS = u64;
pub type SATS = u64;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Balance {
    pub nyks: NYKS,
    pub sats: SATS,
}

/// Generate a random Cosmos account (for testing)
pub fn generate_cosmos_account(prefix: &str) -> anyhow::Result<(SigningKey, PublicKey, AccountId)> {
    let signing_key = SigningKey::random();
    let public_key = signing_key.public_key();
    let sha256_hash = Sha256::digest(public_key.clone().to_bytes());
    let ripemd160_hash = Ripemd160::digest(&sha256_hash);
    let account_id = AccountId::new(prefix, &ripemd160_hash).map_err(|e| anyhow!("{}", e))?;
    Ok((signing_key, public_key, account_id))
}

/// Import account from mnemonic phrase
fn account_from_mnemonic() -> anyhow::Result<(SigningKey, PublicKey, AccountId)> {
    let phrase = prompt_password("Enter your mnemonic phrase (12 or 24 words): ")?;

    let cleaned = phrase.trim().to_lowercase();
    let word_count = cleaned.split_whitespace().count();
    println!("Validating mnemonic with {} words...", word_count);

    // First, surface spelling issues or bad checksum explicitly.
    if let Err(e) = Mnemonic::parse_in(B39Lang::English, &cleaned) {
        match e {
            Bip39Error::UnknownWord(index) => {
                return Err(anyhow!(
                    "Unknown word  at position {} — double‑check spelling against the official BIP‑39 list.",
                    index + 1
                ));
            }
            Bip39Error::InvalidChecksum => {
                return Err(anyhow!(
                    "Checksum mismatch: every word is valid, but the overall phrase isn't — a word is out of place or mistyped."
                ));
            }
            other => return Err(anyhow!("Mnemonic validation error: {}", other)),
        }
    }

    // Safe to parse now.
    let mnemonic = Mnemonic::parse_in(B39Lang::English, &cleaned)?;

    println!("✅ Mnemonic validated successfully");

    let seed = mnemonic.to_seed("");
    let path: DerivationPath = "m/44'/118'/0'/0/0"
        .parse()
        .map_err(|e| anyhow!("Invalid derivation path: {}", e))?;

    let xprv = XPrv::derive_from_path(&seed, &path)
        .map_err(|e| anyhow!("Key derivation failed: {}", e))?;

    let private_key_bytes = xprv.private_key().to_bytes();
    println!("{}", hex::encode(private_key_bytes));
    let signing_key = SigningKey::from_slice(&private_key_bytes)
        .map_err(|e| anyhow!("Invalid private key: {}", e))?;

    let public_key = signing_key.public_key();
    let account_id = public_key
        .account_id(BECH_PREFIX)
        .map_err(|e| anyhow!("Address generation failed: {}", e))?;

    println!("✅ Cosmos account derived from mnemonic successfully");

    Ok((signing_key, public_key, account_id))
}

/// Import account from hex private key
fn account_from_private_key_hex() -> anyhow::Result<(SigningKey, PublicKey, AccountId)> {
    let hex_pk = prompt_password("Enter 32-byte private key hex (64 chars): ")?;
    let hex_pk = hex_pk.trim();

    if hex_pk.len() != 64 {
        return Err(anyhow!("Private key must be exactly 64 hex characters"));
    }

    let bytes = hex::decode(hex_pk).map_err(|e| anyhow!("Invalid hex: {}", e))?;
    if bytes.len() != 32 {
        return Err(anyhow!("Private key must be 32 bytes"));
    }

    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&bytes);

    let signing_key =
        SigningKey::from_slice(&key_bytes).map_err(|e| anyhow!("Invalid key: {}", e))?;
    let public_key = signing_key.public_key();
    let account_id = public_key
        .account_id(BECH_PREFIX)
        .map_err(|e| anyhow!("{}", e))?;

    Ok((signing_key, public_key, account_id))
}

pub async fn check_code(lcd_endpoint: &str, faucet_endpoint: &str) -> anyhow::Result<()> {
    println!("🚀 Twilight Market Maker Client");
    println!("================================");

    println!("\nChoose account import method:");
    println!("1. Generate new random account (testing)");
    println!("2. Import from mnemonic phrase");
    println!("3. Import from private key hex");

    let choice = prompt_password("Enter choice (1, 2, or 3): ")?;

    let (signing_key, public_key, account_id) = match choice.trim() {
        "1" => {
            println!("\n📱 Generating new random account...");
            generate_cosmos_account(BECH_PREFIX)?
        }
        "2" => {
            println!("\n📱 Importing account from mnemonic...");
            account_from_mnemonic()?
        }
        "3" => {
            println!("\n📱 Importing account from private key...");
            account_from_private_key_hex()?
        }
        _ => {
            println!("Invalid choice, using random account");
            generate_cosmos_account(BECH_PREFIX)?
        }
    };

    println!("✅ Account ready!");
    println!("   Address: {}", account_id);

    // Step 1: Get testnet tokens from faucet
    println!("\n💰 Requesting testnet tokens from faucet...");
    match get_nyks(&account_id.to_string(), faucet_endpoint).await {
        Ok(_) => println!("✅ Successfully received testnet tokens"),
        Err(e) => {
            eprintln!("❌ Failed to get tokens from faucet: {}", e);
            println!("💡 You may need to wait or try again later");
        }
    };

    // Wait a bit before next operation
    sleep(Duration::from_secs(5)).await;

    // Step 2: Register Bitcoin deposit address
    println!("\n🔗 Registering Bitcoin deposit address...");
    // let btc_address = "bc1qxdlfjgffe9a4sc9yswdvnaxtjz8e46jnu3vkqu"; // Example address
    // Generate a random Bitcoin testnet address using existing crypto
    let random_key = SigningKey::random();
    let pubkey_bytes = random_key.public_key().to_bytes();
    let btc_address = format!(
        "bc1q{}",
        hex::encode(&pubkey_bytes[..19])
            .chars()
            .take(38)
            .collect::<String>()
    );
    debug!("   Generated BTC Address: {}", btc_address);

    match sign_and_send_reg_deposit_tx(
        signing_key,
        public_key,
        account_id.to_string(),
        btc_address.to_string(),
        lcd_endpoint,
    )
    .await
    {
        Ok(_) => {
            info!("Successfully registered BTC deposit address!");
            debug!("   BTC Address: {}", btc_address);
            debug!("   Amount: 50,000 satoshis");
        }
        Err(e) => {
            eprintln!("❌ Failed to register BTC deposit address: {}", e);
        }
    };

    // Wait a bit before next operation
    sleep(Duration::from_secs(5)).await;

    // Step 3: Mint test satoshis
    println!("\n🪙 Minting test satoshis...");
    match mint_sats(&account_id.to_string(), faucet_endpoint).await {
        Ok(_) => println!("✅ Successfully minted test satoshis"),
        Err(e) => {
            eprintln!("❌ Failed to mint satoshis: {}", e);
            println!("💡 You may need to wait or try again later");
        }
    };

    println!("\n🎉 Market maker client operations completed!");
    println!("   Check your account balance on the Twilight explorer");
    println!("   https://explorer.twilight.rest");

    Ok(())
}

pub async fn check_balance(address: &str, lcd_endpoint: &str) -> anyhow::Result<Balance> {
    let url = format!("{}/cosmos/bank/v1beta1/balances/{}", lcd_endpoint, address);
    let client = Client::new();
    let response = client.get(url).send().await?;
    let balance: Value = response.json().await?;
    let mut balance_nyks = 0;
    let mut balance_sats = 0;
    if let Some(balances) = balance.get("balances").and_then(|b| b.as_array()) {
        for balance in balances {
            if let (Some(amount), Some(denom)) = (
                balance.get("amount").and_then(|a| a.as_str()),
                balance.get("denom").and_then(|d| d.as_str()),
            ) {
                println!("Balance: {} {}", amount, denom);
                if denom == "nyks" {
                    balance_nyks = amount.parse::<NYKS>().unwrap_or(0);
                } else if denom == "sats" {
                    balance_sats = amount.parse::<SATS>().unwrap_or(0);
                }
            }
        }
    }
    Ok(Balance {
        nyks: balance_nyks,
        sats: balance_sats,
    })
}

pub async fn create_and_export_randmon_wallet_account(name: &str) -> anyhow::Result<()> {
    let write_path = format!("{}.json", name);
    if std::path::Path::new(&write_path).exists() {
        return Err(anyhow!(
            "{} already exists. Please remove or rename it before creating a new account.",
            write_path
        ));
    }

    let mnemonic = Mnemonic::generate_in(B39Lang::English, 24)?;
    let seed = mnemonic.to_seed("");
    let path: DerivationPath = "m/44'/118'/0'/0/0"
        .parse()
        .map_err(|e| anyhow!("Invalid derivation path: {}", e))?;

    let xprv = XPrv::derive_from_path(&seed, &path)
        .map_err(|e| anyhow!("Key derivation failed: {}", e))?;

    let private_key_bytes = xprv.private_key().to_bytes();

    let signing_key = SigningKey::from_slice(&private_key_bytes)
        .map_err(|e| anyhow!("Invalid private key: {}", e))?;
    let public_key = signing_key.public_key();
    println!("{}", hex::encode(public_key.to_bytes()));
    let account_id = public_key
        .account_id(BECH_PREFIX)
        .map_err(|e| anyhow!("Address generation failed: {}", e))?;

    println!("twilight account address: {}", account_id);
    let random_key = SigningKey::random();
    let pubkey_bytes = random_key.public_key().to_bytes();
    let btc_address = format!(
        "bc1q{}",
        hex::encode(&pubkey_bytes[..19])
            .chars()
            .take(38)
            .collect::<String>()
    );

    let account_info = serde_json::json!({
        "mnemonic": mnemonic.to_string(),
        "private_key": hex::encode(private_key_bytes),
        "public_key": hex::encode(public_key.to_bytes()),
        "twilightaddress": account_id.to_string(),
        "btc_address": btc_address.to_string(),
        "btc_address_registered": false,
        "balance_nyks": 0,
        "balance_sats": 0,
        "sequence": 0,
    });

    let json_string = serde_json::to_string_pretty(&account_info)?;
    std::fs::write(write_path.clone(), json_string)?;

    println!("✅ Account information saved to {}", write_path);

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, ZeroizeOnDrop)]
pub struct Wallet {
    pub private_key: Vec<u8>,
    pub public_key: Vec<u8>,
    pub twilightaddress: String,
    pub balance_nyks: NYKS,
    pub balance_sats: SATS,
    pub sequence: u64,
    pub btc_address: String,
    pub btc_address_registered: bool,
    #[zeroize(skip)]
    pub account_info: Option<Account>,
    #[zeroize(skip)]
    pub chain_config: WalletEndPointConfig,
}
impl Wallet {
    pub fn new(chain_config: Option<WalletEndPointConfig>) -> anyhow::Result<Self> {
        let chain_config = chain_config.unwrap_or(WalletEndPointConfig::default());
        let mnemonic = Mnemonic::generate_in(B39Lang::English, 24)?;
        let seed = mnemonic.to_seed("");
        let path: DerivationPath = "m/44'/118'/0'/0/0"
            .parse()
            .map_err(|e| anyhow!("Invalid derivation path: {}", e))?;

        let xprv = XPrv::derive_from_path(&seed, &path)
            .map_err(|e| anyhow!("Key derivation failed: {}", e))?;

        let private_key_bytes = xprv.private_key().to_bytes();

        let signing_key = SigningKey::from_slice(&private_key_bytes)
            .map_err(|e| anyhow!("Invalid private key: {}", e))?;
        let public_key = signing_key.public_key();
        let account_id = public_key
            .account_id(BECH_PREFIX)
            .map_err(|e| anyhow!("Address generation failed: {}", e))?;
        let (_wif, btc_address) =
            crate::wallet::generate_btc_key::segwit_from_mnemonic(&mnemonic.to_string())?;
        // save_mnemonic(&account_id.to_string(), mnemonic.to_string())?;
        print_secret_to_tty(&mut mnemonic.to_string())?;
        Ok(Wallet {
            private_key: private_key_bytes.to_vec(),
            public_key: public_key.to_bytes().to_vec(),
            twilightaddress: account_id.to_string(),
            balance_nyks: 0,
            balance_sats: 0,
            sequence: 0,
            btc_address,
            btc_address_registered: false,
            account_info: None,
            chain_config,
        })
    }

    pub async fn create_new_with_random_btc_address() -> anyhow::Result<Wallet> {
        let mnemonic = Mnemonic::generate_in(B39Lang::English, 24)?;
        let seed = mnemonic.to_seed("");
        let path: DerivationPath = "m/44'/118'/0'/0/0"
            .parse()
            .map_err(|e| anyhow!("Invalid derivation path: {}", e))?;

        let xprv = XPrv::derive_from_path(&seed, &path)
            .map_err(|e| anyhow!("Key derivation failed: {}", e))?;

        let private_key_bytes = xprv.private_key().to_bytes();

        let signing_key = SigningKey::from_slice(&private_key_bytes)
            .map_err(|e| anyhow!("Invalid private key: {}", e))?;
        let public_key = signing_key.public_key();
        let account_id = public_key
            .account_id(BECH_PREFIX)
            .map_err(|e| anyhow!("Address generation failed: {}", e))?;

        let random_key = SigningKey::random();
        let pubkey_bytes = random_key.public_key().to_bytes();
        let btc_address = format!(
            "bc1q{}",
            hex::encode(&pubkey_bytes[..19])
                .chars()
                .take(38)
                .collect::<String>()
        );

        Ok(Wallet {
            private_key: private_key_bytes.to_vec(),
            public_key: public_key.to_bytes().to_vec(),
            twilightaddress: account_id.to_string(),
            balance_nyks: 0,
            balance_sats: 0,
            sequence: 0,
            btc_address,
            btc_address_registered: false,
            account_info: None,
            chain_config: WalletEndPointConfig::from_env(),
        })
    }

    pub fn from(
        private_key: String,
        public_key: String,
        twilightaddress: String,
        btc_address: String,
        chain_config: Option<WalletEndPointConfig>,
    ) -> Wallet {
        let chain_config = chain_config.unwrap_or(WalletEndPointConfig::default());
        Wallet {
            private_key: hex::decode(private_key.clone()).unwrap_or_default(),
            public_key: hex::decode(public_key).unwrap_or_default(),
            twilightaddress,
            balance_nyks: 0,
            balance_sats: 0,
            sequence: 0,
            btc_address,
            btc_address_registered: false,
            account_info: None,
            chain_config,
        }
    }

    pub fn from_mnemonic(
        mnemonic: &str,
        chain_config: Option<WalletEndPointConfig>,
    ) -> anyhow::Result<Wallet> {
        let chain_config = chain_config.unwrap_or(WalletEndPointConfig::default());
        let mnemonic = Mnemonic::parse_in(B39Lang::English, mnemonic)?;
        let seed = mnemonic.to_seed("");
        let path: DerivationPath = "m/44'/118'/0'/0/0"
            .parse()
            .map_err(|e| anyhow!("Invalid derivation path: {}", e))?;
        let xprv = XPrv::derive_from_path(&seed, &path)
            .map_err(|e| anyhow!("Key derivation failed: {}", e))?;
        let private_key_bytes = xprv.private_key().to_bytes();
        let signing_key = SigningKey::from_slice(&private_key_bytes)
            .map_err(|e| anyhow!("Invalid private key: {}", e))?;
        let public_key = signing_key.public_key();
        let account_id = public_key
            .account_id(BECH_PREFIX)
            .map_err(|e| anyhow!("Address generation failed: {}", e))?;
        let (_wif, btc_address) =
            crate::wallet::generate_btc_key::segwit_from_mnemonic(&mnemonic.to_string())?;
        Ok(Wallet {
            private_key: private_key_bytes.to_vec(),
            public_key: public_key.to_bytes().to_vec(),
            twilightaddress: account_id.to_string(),
            balance_nyks: 0,
            balance_sats: 0,
            sequence: 0,
            btc_address,
            btc_address_registered: false,
            account_info: None,
            chain_config,
        })
    }

    pub fn from_private_key(
        private_key: &str,
        btc_address: &str,
        chain_config: Option<WalletEndPointConfig>,
    ) -> anyhow::Result<Wallet> {
        let chain_config = chain_config.unwrap_or(WalletEndPointConfig::default());
        let private_key = hex::decode(private_key.to_string())?;
        let signing_key = SigningKey::from_slice(&private_key).map_err(|e| anyhow!("{}", e))?;
        let public_key = signing_key.public_key();
        let account_id = public_key
            .account_id(BECH_PREFIX)
            .map_err(|e| anyhow!("Address generation failed: {}", e))?;

        Ok(Wallet {
            private_key: private_key.to_vec(),
            public_key: public_key.to_bytes().to_vec(),
            twilightaddress: account_id.to_string(),
            balance_nyks: 0,
            balance_sats: 0,
            sequence: 0,
            btc_address: btc_address.to_string(),
            btc_address_registered: false,
            account_info: None,
            chain_config,
        })
    }

    pub fn import_from_json(path: &str) -> anyhow::Result<Wallet> {
        let json_string: String = std::fs::read_to_string(path)?;
        let account_info: Value = serde_json::from_str(&json_string)?;

        let wallet = Wallet {
            private_key: hex::decode(account_info["private_key"].as_str().unwrap().to_string())
                .unwrap_or_default(),
            public_key: hex::decode(account_info["public_key"].as_str().unwrap().to_string())
                .unwrap_or_default(),
            twilightaddress: account_info["twilightaddress"]
                .as_str()
                .unwrap()
                .to_string(),
            balance_nyks: account_info["balance_nyks"].as_u64().unwrap_or_default(),
            balance_sats: account_info["balance_sats"].as_u64().unwrap_or_default(),
            sequence: account_info["sequence"].as_u64().unwrap_or_default(),
            btc_address: account_info["btc_address"].as_str().unwrap().to_string(),
            btc_address_registered: account_info["btc_address_registered"]
                .as_bool()
                .unwrap_or_default(),
            account_info: None,
            chain_config: WalletEndPointConfig::new(
                account_info["lcd_endpoint"]
                    .as_str()
                    .unwrap_or("http://0.0.0.0:1317")
                    .to_string(),
                account_info["faucet_endpoint"]
                    .as_str()
                    .unwrap_or("http://0.0.0.0:6969")
                    .to_string(),
                account_info["rpc_endpoint"]
                    .as_str()
                    .unwrap_or("http://0.0.0.0:26657")
                    .to_string(),
                account_info["chain_id"]
                    .as_str()
                    .unwrap_or("nyks")
                    .to_string(),
            ),
        };
        Ok(wallet)
    }

    pub fn signing_key(&self) -> anyhow::Result<SigningKey> {
        let signing_key =
            SigningKey::from_slice(&self.private_key).map_err(|e| anyhow!("{}", e))?;
        Ok(signing_key)
    }
    pub fn public_key(&self) -> anyhow::Result<PublicKey> {
        Ok(self.signing_key()?.public_key())
    }
    pub async fn update_balance(&mut self) -> anyhow::Result<Balance> {
        let url = format!(
            "{}/cosmos/bank/v1beta1/balances/{}",
            self.chain_config.lcd_endpoint, self.twilightaddress
        );
        let client = Client::new();
        let response = client.get(url).send().await?;
        let balance: Value = response.json().await?;
        let mut balance_nyks = 0;
        let mut balance_sats = 0;
        if let Some(balances) = balance.get("balances").and_then(|b| b.as_array()) {
            for balance in balances {
                if let (Some(amount), Some(denom)) = (
                    balance.get("amount").and_then(|a| a.as_str()),
                    balance.get("denom").and_then(|d| d.as_str()),
                ) {
                    debug!("Updating balance: {} {}", amount, denom);
                    if denom == "nyks" {
                        balance_nyks = amount.parse::<NYKS>().unwrap_or(0);
                    } else if denom == "sats" {
                        balance_sats = amount.parse::<SATS>().unwrap_or(0);
                    }
                }
            }
        }
        self.balance_nyks = balance_nyks;
        self.balance_sats = balance_sats;
        Ok(Balance {
            nyks: balance_nyks,
            sats: balance_sats,
        })
    }
    pub async fn account_info(&self) -> anyhow::Result<AccountResponse> {
        let account_details =
            fetch_account_details(&self.twilightaddress, &self.chain_config.lcd_endpoint).await?;
        Ok(account_details)
    }
    pub async fn update_account_info(&mut self) -> anyhow::Result<()> {
        let account_details =
            fetch_account_details(&self.twilightaddress, &self.chain_config.lcd_endpoint).await?;
        self.account_info = Some(account_details.account);
        Ok(())
    }
    pub fn export_to_json(&self, path: &str) -> anyhow::Result<()> {
        let account_info = serde_json::json!({
            "private_key": hex::encode(self.private_key.clone()),
            "public_key": hex::encode(self.public_key.clone()),
            "twilightaddress": self.twilightaddress,
            "btc_address": self.btc_address,
            "btc_address_registered": self.btc_address_registered,
            "balance_nyks": self.balance_nyks,
            "balance_sats": self.balance_sats,
            "sequence": self.sequence,
            "account_info": self.account_info,
            "lcd_endpoint": self.chain_config.lcd_endpoint,
            "faucet_endpoint": self.chain_config.faucet_endpoint,
            "rpc_endpoint": self.chain_config.rpc_endpoint,
            "chain_id": self.chain_config.chain_id,
        });
        std::fs::write(path, account_info.to_string())?;
        Ok(())
    }

    pub fn from_mnemonic_file(path: &str) -> anyhow::Result<Wallet> {
        let mnemonic = std::fs::read_to_string(path)?;
        let wallet = Wallet::from_mnemonic(&mnemonic, None)?;
        Ok(wallet)
    }

    pub fn get_zk_account_seed(
        &self,
        chain_id: &str,
        derivation_message: &str,
    ) -> Result<SecretString, String> {
        Ok(SecretString::new(
            generate_seed(
                &self.private_key,
                &self.twilightaddress,
                derivation_message,
                chain_id,
            )
            .map_err(|e| format!("Failed to generate seed: {}", e))?
            .get_signature(),
        ))
    }
}
// #[cfg(feature = "testnet")]
pub async fn get_test_tokens(wallet: &mut Wallet) -> anyhow::Result<()> {
    let balance = wallet.update_balance().await?;
    debug!("Checking balance values if nyks is less than 50000");
    debug!("nyks: {}", balance.nyks);
    if balance.nyks < 50000 {
        debug!("Getting tokens from faucet");
        get_nyks(
            &wallet.twilightaddress,
            &wallet.chain_config.faucet_endpoint,
        )
        .await
        .unwrap_or_else(|e| {
            error!("Failed to get tokens from faucet: {}", e);
            info!("💡 You may need to wait or try again later");
        });
    } else {
        info!("Skipping get tokens from faucet because nyks is greater than 50000");
    }
    info!("waiting for updated nyks balance to appear on-chain");
    sleep(Duration::from_secs(10)).await;
    debug!("Checking balance values if sats is 0 or less than 50000");
    debug!("sats: {}", balance.sats);
    if balance.sats == 0 && !wallet.btc_address_registered {
        info!("Registering random BTC deposit address");
        match sign_and_send_reg_deposit_tx(
            wallet.signing_key()?,
            wallet.public_key()?,
            wallet.twilightaddress.to_string(),
            wallet.btc_address.to_string(),
            &wallet.chain_config.lcd_endpoint,
        )
        .await
        {
            Ok(_) => {
                info!("Successfully registered BTC deposit address!");
                debug!("   BTC Address: {}", &wallet.btc_address);
                debug!("   Amount: 50,000 satoshis");
                info!("   waiting for registered BTC deposit address to appear on-chain");
                sleep(Duration::from_secs(10)).await;
                info!("Minting test BTC...");
                mint_sats(
                    &wallet.twilightaddress,
                    &wallet.chain_config.faucet_endpoint,
                )
                .await
                .unwrap_or_else(|e| {
                    error!("Failed to mint satoshis: {}", e);
                    info!("    You may need to restart the process again or try again later");
                });
                wallet.btc_address_registered = true;
            }
            Err(e) => {
                error!("Failed to register BTC deposit address: {}", e);
                debug!("   BTC Address: {}", &wallet.btc_address);

                info!("    You may need to restart the process again or try again later");
            }
        };
    } else if balance.sats < 50000 {
        debug!("    Skipping register BTC deposit address because sats is less than 50000");
        info!("Minting test BTC...");
        mint_sats(
            &wallet.twilightaddress,
            &wallet.chain_config.faucet_endpoint,
        )
        .await
        .unwrap_or_else(|e| {
            error!("Failed to mint satoshis: {}", e);
            info!("    You may need to restart the process again or try again later");
        });
    } else {
        info!("Skipping minting test BTC because sats is greater than 50000");
    }
    info!("    waiting for updated sats balance to appear on-chain");
    sleep(Duration::from_secs(10)).await;
    let balance = wallet.update_balance().await?;
    debug!("    new balance: {:?}", balance);

    Ok(())
}
