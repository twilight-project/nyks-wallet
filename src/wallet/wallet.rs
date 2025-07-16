use crate::faucet::*;
use anyhow::anyhow;
use bip32::{DerivationPath, XPrv};
use bip39::{Error as Bip39Error, Language as B39Lang, Mnemonic};
use cosmrs::AccountId;
use cosmrs::crypto::{PublicKey, secp256k1::SigningKey};
use reqwest::Client;
use ripemd::Ripemd160;
use rpassword::prompt_password;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use tokio::time::{Duration, sleep};

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

pub async fn check_code() -> anyhow::Result<()> {
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
    match get_nyks(&account_id.to_string()).await {
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
    println!("   Generated BTC Address: {}", btc_address);

    match sign_and_send_reg_deposit_tx(
        signing_key,
        public_key,
        account_id.to_string(),
        btc_address.to_string(),
    )
    .await
    {
        Ok(_) => {
            println!("✅ Successfully registered BTC deposit address!");
            println!("   BTC Address: {}", btc_address);
            println!("   Test Amount: 50,000 satoshis");
            println!("   Staking Amount: 10,000 twilight tokens");
        }
        Err(e) => {
            eprintln!("❌ Failed to register BTC deposit address: {}", e);
        }
    };

    // Wait a bit before next operation
    sleep(Duration::from_secs(5)).await;

    // Step 3: Mint test satoshis
    println!("\n🪙 Minting test satoshis...");
    match mint_sats(&account_id.to_string()).await {
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

pub async fn check_balance(address: &str) -> anyhow::Result<Balance> {
    let baseurl = std::env::var("LCD_BASE_URL").unwrap_or("https://lcd.twilight.rest".to_string());
    let url = format!("{}/cosmos/bank/v1beta1/balances/{}", baseurl, address);
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
        "address": account_id.to_string(),
        "btc_address": btc_address.to_string(),
    });

    let json_string = serde_json::to_string_pretty(&account_info)?;
    std::fs::write(write_path.clone(), json_string)?;

    println!("✅ Account information saved to {}", write_path);

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub private_key: Vec<u8>,
    pub public_key: Vec<u8>,
    pub twilightaddress: String,
    pub balance_nyks: NYKS,
    pub balance_sats: SATS,
    pub sequence: u64,
    pub btc_address: String,
}
impl Wallet {
    pub fn new(
        private_key: String,
        public_key: String,
        twilightaddress: String,
        btc_address: String,
    ) -> Wallet {
        Wallet {
            private_key: hex::decode(private_key.clone()).unwrap_or_default(),
            public_key: hex::decode(public_key).unwrap_or_default(),
            twilightaddress,
            balance_nyks: 0,
            balance_sats: 0,
            sequence: 0,
            btc_address,
        }
    }

    pub fn from_mnemonic(mnemonic: &str) -> anyhow::Result<Wallet> {
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
        })
    }

    pub fn from_private_key(private_key: &str) -> anyhow::Result<Wallet> {
        let private_key = hex::decode(private_key.to_string())?;
        let signing_key = SigningKey::from_slice(&private_key).map_err(|e| anyhow!("{}", e))?;
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
            private_key: private_key.to_vec(),
            public_key: public_key.to_bytes().to_vec(),
            twilightaddress: account_id.to_string(),
            balance_nyks: 0,
            balance_sats: 0,
            sequence: 0,
            btc_address,
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
            twilightaddress: account_info["address"].as_str().unwrap().to_string(),
            balance_nyks: 0,
            balance_sats: 0,
            sequence: 0,
            btc_address: account_info["btc_address"].as_str().unwrap().to_string(),
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
        let baseurl = std::env::var("LCD_BASE_URL").unwrap_or("http://0.0.0.0:1317".to_string());
        let url = format!(
            "{}/cosmos/bank/v1beta1/balances/{}",
            baseurl, self.twilightaddress
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
                    println!("Balance: {} {}", amount, denom);
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
        let account_details = fetch_account_details(&self.twilightaddress).await?;
        Ok(account_details)
    }
}

pub async fn get_test_tokens(wallet: &mut Wallet) -> anyhow::Result<()> {
    let balance = wallet.update_balance().await?;
    println!("Checking balance values if nyks is less than 1000000");
    println!("nyks: {}", balance.nyks);
    if balance.nyks < 1000000 {
        println!("Getting tokens from faucet");
        get_nyks(&wallet.twilightaddress).await.unwrap_or_else(|e| {
            eprintln!("Failed to get tokens from faucet: {}", e);
            println!("💡 You may need to wait or try again later");
        });
    }
    println!("waiting 10 seconds to update nyks balance");
    sleep(Duration::from_secs(10)).await;
    println!("Checking balance values if sats is 0 or less than 1000000");
    println!("sats: {}", balance.sats);
    if balance.sats == 0 {
        println!("Registering BTC deposit address");
        match sign_and_send_reg_deposit_tx(
            wallet.signing_key()?,
            wallet.public_key()?,
            wallet.twilightaddress.to_string(),
            wallet.btc_address.to_string(),
        )
        .await
        {
            Ok(_) => {
                println!("✅ Successfully registered BTC deposit address!");
                println!("   BTC Address: {}", &wallet.btc_address);
                println!("   Test Amount: 50,000 satoshis");
                println!("waiting 10 seconds to update sats balance");
                sleep(Duration::from_secs(10)).await;
                println!("Minting satoshis");
                mint_sats(&wallet.twilightaddress)
                    .await
                    .unwrap_or_else(|e| {
                        eprintln!("Failed to mint satoshis: {}", e);
                        println!("💡 You may need to wait or try again later");
                    });
            }
            Err(e) => {
                eprintln!("❌ Failed to register BTC deposit address: {}", e);
            }
        };
    } else if balance.sats < 1000000 {
        println!("Skipping register BTC deposit address because sats is less than 1000000");
        println!("Minting satoshis");
        mint_sats_5btc(&wallet.twilightaddress)
            .await
            .unwrap_or_else(|e| {
                eprintln!("Failed to mint satoshis: {}", e);
                println!("💡 You may need to wait or try again later");
            });
    }
    sleep(Duration::from_secs(5)).await;
    let balance = wallet.update_balance().await?;
    println!("new balance: {:?}", balance);

    Ok(())
}
