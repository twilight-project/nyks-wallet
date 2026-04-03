use crate::config::WalletEndPointConfig;
use crate::security::print_secret_to_tty;
use crate::{faucet::*, generate_seed};
use anyhow::anyhow;
use bip32::{DerivationPath, XPrv};
use bip39::{Language as B39Lang, Mnemonic};
use cosmrs::crypto::{secp256k1::SigningKey, PublicKey};
use cosmrs::AccountId;
use log::{debug, error, info};
use reqwest::Client;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::time::{sleep, Duration};
use zeroize::ZeroizeOnDrop;
pub const BECH_PREFIX: &str = "twilight";

pub type NYKS = u64;
pub type SATS = u64;

/// Returns the BIP-44 coin type based on `NETWORK_TYPE` env var.
/// Testnet = 1 (SLIP-44), Mainnet = 118 (Cosmos). Must match Keplr's `slip44`.
fn coin_type() -> u32 {
    use crate::config::NETWORK_TYPE;
    match NETWORK_TYPE.as_str() {
        "mainnet" => 118,
        _ => 118,
    }
}

/// Returns the BIP-44 derivation path using the configured coin type.
fn derivation_path() -> DerivationPath {
    let ct = coin_type();
    format!("m/44'/{ct}'/0'/0/0")
        .parse()
        .expect("valid derivation path")
}

/// Derived key material from a mnemonic phrase.
struct DerivedKeys {
    private_key: Vec<u8>,
    public_key: Vec<u8>,
    account_id: AccountId,
}

/// Shared key derivation pipeline: mnemonic -> seed -> XPrv -> SigningKey -> PublicKey -> AccountId.
/// All wallet creation methods use this to avoid duplicating the derivation logic.
fn derive_keys(mnemonic: &Mnemonic) -> anyhow::Result<DerivedKeys> {
    let seed = mnemonic.to_seed("");
    let path = derivation_path();

    let xprv = XPrv::derive_from_path(&seed, &path)
        .map_err(|e| anyhow!("Key derivation failed: {}", e))?;

    let private_key_bytes = xprv.private_key().to_bytes();

    let signing_key = SigningKey::from_slice(&private_key_bytes)
        .map_err(|e| anyhow!("Invalid private key: {}", e))?;
    let public_key = signing_key.public_key();
    let account_id = public_key
        .account_id(BECH_PREFIX)
        .map_err(|e| anyhow!("Address generation failed: {}", e))?;

    Ok(DerivedKeys {
        private_key: private_key_bytes.to_vec(),
        public_key: public_key.to_bytes().to_vec(),
        account_id,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Balance {
    pub nyks: NYKS,
    pub sats: SATS,
}

pub use crate::wallet::btc_wallet::types::*;

/// Deposit record from the Twilight indexer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerDeposit {
    pub id: u64,
    pub tx_hash: String,
    pub block_height: u64,
    pub reserve_address: String,
    pub deposit_amount: String,
    pub btc_height: String,
    pub btc_hash: String,
    pub votes: u64,
    pub confirmed: bool,
    pub created_at: String,
}

/// Withdrawal record from the Twilight indexer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerWithdrawal {
    pub id: u64,
    pub withdraw_identifier: u32,
    pub withdraw_address: String,
    pub withdraw_reserve_id: String,
    pub withdraw_amount: String,
    pub is_confirmed: bool,
    pub block_height: u64,
    pub created_at: String,
    pub updated_at: String,
}

/// Full account info from the Twilight indexer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerAccountInfo {
    pub address: String,
    pub balance: String,
    pub tx_count: u64,
    pub first_seen: String,
    pub last_seen: String,
    pub balances: Vec<IndexerBalance>,
    pub deposits: Vec<IndexerDeposit>,
    pub withdrawals: Vec<IndexerWithdrawal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerBalance {
    pub denom: String,
    pub amount: String,
}

pub use crate::wallet::btc_wallet::balance::*;

/// Fetch on-chain balance for the given address via LCD endpoint.
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
                debug!("Balance: {} {}", amount, denom);
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

#[derive(Clone, Serialize, Deserialize, ZeroizeOnDrop)]
pub struct Wallet {
    pub(crate) private_key: Vec<u8>,
    pub public_key: Vec<u8>,
    pub twilightaddress: String,
    pub balance_nyks: NYKS,
    pub balance_sats: SATS,
    pub sequence: u64,
    pub btc_address: String,
    pub btc_address_registered: bool,
    /// BTC wallet key material (populated when created from mnemonic)
    #[zeroize(skip)]
    pub btc_wallet: Option<crate::wallet::btc_wallet::BtcWallet>,
    #[zeroize(skip)]
    pub account_info: Option<Account>,
    #[zeroize(skip)]
    pub chain_config: WalletEndPointConfig,
}

impl std::fmt::Display for Wallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Wallet(address={}, nyks={}, sats={}, btc={})",
            self.twilightaddress, self.balance_nyks, self.balance_sats, self.btc_address
        )
    }
}

impl std::fmt::Debug for Wallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Wallet")
            .field("private_key", &"[REDACTED]")
            .field("public_key", &hex::encode(&self.public_key))
            .field("twilightaddress", &self.twilightaddress)
            .field("balance_nyks", &self.balance_nyks)
            .field("balance_sats", &self.balance_sats)
            .field("sequence", &self.sequence)
            .field("btc_address", &self.btc_address)
            .field("btc_address_registered", &self.btc_address_registered)
            .field("btc_wallet", &self.btc_wallet)
            .field("account_info", &self.account_info)
            .field("chain_config", &self.chain_config)
            .finish()
    }
}

impl Wallet {
    /// Controlled access to private key bytes. Prefer `signing_key()` when possible.
    pub fn private_key_bytes(&self) -> &[u8] {
        &self.private_key
    }

    pub fn new(chain_config: Option<WalletEndPointConfig>) -> anyhow::Result<Self> {
        let chain_config = chain_config.unwrap_or_default();
        let mnemonic = Mnemonic::generate_in(B39Lang::English, 24)?;
        let keys = derive_keys(&mnemonic)?;
        let mnemonic_str = mnemonic.to_string();
        let btc_wallet = crate::wallet::btc_wallet::BtcWallet::from_mnemonic(&mnemonic_str)?;
        let btc_address = btc_wallet.address.clone();
        print_secret_to_tty(&mut mnemonic_str.clone())?;
        Ok(Wallet {
            private_key: keys.private_key,
            public_key: keys.public_key,
            twilightaddress: keys.account_id.to_string(),
            balance_nyks: 0,
            balance_sats: 0,
            sequence: 0,
            btc_address,
            btc_address_registered: false,
            btc_wallet: Some(btc_wallet),
            account_info: None,
            chain_config,
        })
    }

    pub async fn create_new_with_random_btc_address() -> anyhow::Result<Wallet> {
        let mnemonic = Mnemonic::generate_in(B39Lang::English, 24)?;
        let keys = derive_keys(&mnemonic)?;
        let btc_wallet = crate::wallet::btc_wallet::BtcWallet::from_mnemonic(&mnemonic.to_string())?;
        let btc_address = btc_wallet.address.clone();

        Ok(Wallet {
            private_key: keys.private_key,
            public_key: keys.public_key,
            twilightaddress: keys.account_id.to_string(),
            balance_nyks: 0,
            balance_sats: 0,
            sequence: 0,
            btc_address,
            btc_address_registered: false,
            btc_wallet: Some(btc_wallet),
            account_info: None,
            chain_config: WalletEndPointConfig::from_env(),
        })
    }

    pub fn from_mnemonic(
        mnemonic: &str,
        chain_config: Option<WalletEndPointConfig>,
    ) -> anyhow::Result<Wallet> {
        let chain_config = chain_config.unwrap_or_default();
        let mnemonic = Mnemonic::parse_in(B39Lang::English, mnemonic)?;
        let keys = derive_keys(&mnemonic)?;
        let mnemonic_str = mnemonic.to_string();
        let btc_wallet = crate::wallet::btc_wallet::BtcWallet::from_mnemonic(&mnemonic_str)?;
        let btc_address = btc_wallet.address.clone();
        Ok(Wallet {
            private_key: keys.private_key,
            public_key: keys.public_key,
            twilightaddress: keys.account_id.to_string(),
            balance_nyks: 0,
            balance_sats: 0,
            sequence: 0,
            btc_address,
            btc_address_registered: false,
            btc_wallet: Some(btc_wallet),
            account_info: None,
            chain_config,
        })
    }

    pub fn from_private_key(
        private_key: &str,
        btc_address: &str,
        chain_config: Option<WalletEndPointConfig>,
    ) -> anyhow::Result<Wallet> {
        let chain_config = chain_config.unwrap_or_default();
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
            btc_wallet: None,
            account_info: None,
            chain_config,
        })
    }

    pub fn import_from_json(path: &str) -> anyhow::Result<Wallet> {
        let json_string: String = std::fs::read_to_string(path)?;
        let account_info: Value = serde_json::from_str(&json_string)?;
        let wallet_config = WalletEndPointConfig::from_env();
        let wallet = Wallet {
            private_key: hex::decode(
                account_info["private_key"]
                    .as_str()
                    .ok_or_else(|| anyhow!("private_key not found"))?
                    .to_string(),
            )
            .unwrap_or_default(),
            public_key: hex::decode(
                account_info["public_key"]
                    .as_str()
                    .ok_or_else(|| anyhow!("public_key not found"))?
                    .to_string(),
            )
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
            btc_wallet: account_info.get("btc_wallet")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            account_info: None,
            chain_config: WalletEndPointConfig::new(
                account_info["lcd_endpoint"]
                    .as_str()
                    .unwrap_or(&wallet_config.lcd_endpoint)
                    .to_string(),
                account_info["faucet_endpoint"]
                    .as_str()
                    .unwrap_or(&wallet_config.faucet_endpoint)
                    .to_string(),
                account_info["rpc_endpoint"]
                    .as_str()
                    .unwrap_or(&wallet_config.rpc_endpoint)
                    .to_string(),
                account_info["chain_id"]
                    .as_str()
                    .unwrap_or(wallet_config.chain_id.as_str())
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

    /// Fetch and update on-chain balance, reusing the shared `check_balance` function.
    pub async fn update_balance(&mut self) -> anyhow::Result<Balance> {
        let balance = check_balance(&self.twilightaddress, &self.chain_config.lcd_endpoint).await?;
        self.balance_nyks = balance.nyks;
        self.balance_sats = balance.sats;
        Ok(balance)
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
            "btc_wallet": self.btc_wallet,
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

    /// Send tokens (nyks or sats) to another Twilight address.
    /// Returns the transaction hash on success.
    pub async fn send_tokens(
        &mut self,
        to_address: &str,
        amount: u64,
        denom: &str,
    ) -> anyhow::Result<String> {
        use crate::nyks_rpc::rpcclient::method::{Method, MethodTypeURL};
        use crate::nyks_rpc::rpcclient::txrequest::{RpcBody, RpcRequest, TxParams};
        use crate::nyks_rpc::rpcclient::txresult::parse_tx_response;

        if denom != "nyks" && denom != "sats" {
            return Err(anyhow!("denom must be 'nyks' or 'sats'"));
        }

        #[derive(prost::Message)]
        struct Coin {
            #[prost(string, tag = "1")]
            denom: String,
            #[prost(string, tag = "2")]
            amount: String,
        }

        #[derive(prost::Message)]
        struct CosmosMsgSend {
            #[prost(string, tag = "1")]
            from_address: String,
            #[prost(string, tag = "2")]
            to_address: String,
            #[prost(message, repeated, tag = "3")]
            amount: Vec<Coin>,
        }

        let msg = CosmosMsgSend {
            from_address: self.twilightaddress.clone(),
            to_address: to_address.to_string(),
            amount: vec![Coin {
                denom: denom.to_string(),
                amount: amount.to_string(),
            }],
        };

        let method_type = MethodTypeURL::MsgSend;
        let any_msg = method_type.type_url(msg);

        let sk = self.signing_key()?;
        let pk = self.public_key()?;

        let account_details = crate::faucet::fetch_account_details(
            &self.twilightaddress,
            &self.chain_config.lcd_endpoint,
        )
        .await?;
        let account_number = account_details.account.account_number;
        let sequence = account_details.account.sequence;

        let signed_tx =
            method_type.sign_msg::<CosmosMsgSend>(any_msg, pk, sequence, account_number, sk)?;

        let method = Method::broadcast_tx_sync;
        let (tx_send, _): (RpcBody<TxParams>, String) =
            RpcRequest::new_with_data(TxParams::new(signed_tx.clone()), method, signed_tx);

        let rpc_endpoint = self.chain_config.rpc_endpoint.clone();
        let response = tokio::task::spawn_blocking(move || tx_send.send(rpc_endpoint))
            .await
            .map_err(|e| anyhow!("RPC send failed: {e}"))?;

        match response {
            Ok(rpc_response) => {
                let result = parse_tx_response(&method, rpc_response)?;
                let tx_hash = result.get_tx_hash();
                let code = result.get_code();
                if code == 0 {
                    Ok(tx_hash)
                } else {
                    Err(anyhow!(
                        "Transaction failed (code {code}), TX Hash: {tx_hash}"
                    ))
                }
            }
            Err(e) => Err(anyhow!("RPC error: {e}")),
        }
    }

    /// Register the wallet's BTC deposit address on-chain (mainnet only).
    /// This tells the chain that the user intends to deposit `btc_satoshi_amount` satoshis.
    /// After registration, the user must send BTC to a reserve address to complete the deposit.
    /// `twilight_staking_amount` is typically 10_000.
    pub async fn register_btc_deposit(
        &mut self,
        btc_satoshi_amount: u64,
        twilight_staking_amount: u64,
    ) -> anyhow::Result<String> {
        if crate::config::NETWORK_TYPE.as_str() != "mainnet" {
            return Err(anyhow!("register_btc_deposit is only available on mainnet. Use get_test_tokens for testnet."));
        }

        use crate::nyks_rpc::rpcclient::method::{Method, MethodTypeURL};
        use crate::nyks_rpc::rpcclient::txrequest::{RpcBody, RpcRequest, TxParams};
        use crate::nyks_rpc::rpcclient::txresult::parse_tx_response;

        let msg = crate::MsgRegisterBtcDepositAddress {
            btc_deposit_address: self.btc_address.clone(),
            btc_satoshi_test_amount: btc_satoshi_amount,
            twilight_staking_amount,
            twilight_address: self.twilightaddress.clone(),
        };

        let method_type = MethodTypeURL::MsgRegisterBtcDepositAddress;
        let any_msg = method_type.type_url(msg);

        let sk = self.signing_key()?;
        let pk = self.public_key()?;

        let account_details = crate::faucet::fetch_account_details(
            &self.twilightaddress,
            &self.chain_config.lcd_endpoint,
        )
        .await?;
        let account_number = account_details.account.account_number;
        let sequence = account_details.account.sequence;

        let signed_tx = method_type
            .sign_msg::<crate::MsgRegisterBtcDepositAddress>(any_msg, pk, sequence, account_number, sk)?;

        let method = Method::broadcast_tx_sync;
        let (tx_send, _): (RpcBody<TxParams>, String) =
            RpcRequest::new_with_data(TxParams::new(signed_tx.clone()), method, signed_tx);

        let rpc_endpoint = self.chain_config.rpc_endpoint.clone();
        let response = tokio::task::spawn_blocking(move || tx_send.send(rpc_endpoint))
            .await
            .map_err(|e| anyhow!("RPC send failed: {e}"))?;

        match response {
            Ok(rpc_response) => {
                let result = parse_tx_response(&method, rpc_response)?;
                let tx_hash = result.get_tx_hash();
                let code = result.get_code();
                if code == 0 {
                    self.btc_address_registered = true;
                    info!("Registered BTC deposit address: {}", self.btc_address);
                    Ok(tx_hash)
                } else {
                    Err(anyhow!(
                        "Register BTC deposit failed (code {code}), TX Hash: {tx_hash}"
                    ))
                }
            }
            Err(e) => Err(anyhow!("RPC error: {e}")),
        }
    }

    /// Submit a BTC withdrawal request on-chain.
    /// `withdraw_address` is the Bitcoin address to receive BTC.
    /// `reserve_id` is the reserve pool to withdraw from (fetch via `fetch_btc_reserves`).
    /// `withdraw_amount` is the amount in satoshis.
    pub async fn withdraw_btc(
        &mut self,
        withdraw_address: &str,
        reserve_id: u64,
        withdraw_amount: u64,
    ) -> anyhow::Result<String> {
        if crate::config::NETWORK_TYPE.as_str() != "mainnet" {
            return Err(anyhow!("withdraw_btc is only available on mainnet."));
        }

        use crate::nyks_rpc::rpcclient::method::{Method, MethodTypeURL};
        use crate::nyks_rpc::rpcclient::txrequest::{RpcBody, RpcRequest, TxParams};
        use crate::nyks_rpc::rpcclient::txresult::parse_tx_response;

        let msg = crate::MsgWithdrawBtcRequest {
            withdraw_address: withdraw_address.to_string(),
            reserve_id,
            withdraw_amount,
            twilight_address: self.twilightaddress.clone(),
        };

        let method_type = MethodTypeURL::MsgWithdrawBtcRequest;
        let any_msg = method_type.type_url(msg);

        let sk = self.signing_key()?;
        let pk = self.public_key()?;

        let account_details = crate::faucet::fetch_account_details(
            &self.twilightaddress,
            &self.chain_config.lcd_endpoint,
        )
        .await?;
        let account_number = account_details.account.account_number;
        let sequence = account_details.account.sequence;

        let signed_tx = method_type
            .sign_msg::<crate::MsgWithdrawBtcRequest>(any_msg, pk, sequence, account_number, sk)?;

        let method = Method::broadcast_tx_sync;
        let (tx_send, _): (RpcBody<TxParams>, String) =
            RpcRequest::new_with_data(TxParams::new(signed_tx.clone()), method, signed_tx);

        let rpc_endpoint = self.chain_config.rpc_endpoint.clone();
        let response = tokio::task::spawn_blocking(move || tx_send.send(rpc_endpoint))
            .await
            .map_err(|e| anyhow!("RPC send failed: {e}"))?;

        match response {
            Ok(rpc_response) => {
                let result = parse_tx_response(&method, rpc_response)?;
                let tx_hash = result.get_tx_hash();
                let code = result.get_code();
                if code == 0 {
                    info!("Withdrawal request submitted: {} sats to {}", withdraw_amount, withdraw_address);
                    Ok(tx_hash)
                } else {
                    Err(anyhow!(
                        "Withdraw BTC request failed (code {code}), TX Hash: {tx_hash}"
                    ))
                }
            }
            Err(e) => Err(anyhow!("RPC error: {e}")),
        }
    }

    /// Fetch all BTC reserve pools from the chain.
    /// Returns a list of reserves with their IDs, addresses, and capacity info.
    /// Users need a reserve ID to submit withdrawal requests and should send BTC
    /// deposits to the reserve address.
    pub async fn fetch_btc_reserves(&self) -> anyhow::Result<Vec<BtcReserve>> {
        let url = format!(
            "{}/twilight-project/nyks/volt/btc_reserve",
            self.chain_config.lcd_endpoint
        );
        let client = Client::new();
        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to fetch BTC reserves ({}): {}", status, body));
        }

        let json: Value = response.json().await?;
        let reserves = json
            .get("BtcReserves")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("Missing BtcReserves field in response"))?;

        let mut result = Vec::new();
        for r in reserves {
            result.push(BtcReserve {
                reserve_id: r.get("ReserveId").and_then(|v| v.as_str()).unwrap_or("0").parse().unwrap_or(0),
                reserve_address: r.get("ReserveAddress").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                judge_address: r.get("JudgeAddress").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                btc_relay_capacity_value: r.get("BtcRelayCapacityValue").and_then(|v| v.as_str()).unwrap_or("0").parse().unwrap_or(0),
                total_value: r.get("TotalValue").and_then(|v| v.as_str()).unwrap_or("0").parse().unwrap_or(0),
                private_pool_value: r.get("PrivatePoolValue").and_then(|v| v.as_str()).unwrap_or("0").parse().unwrap_or(0),
                public_value: r.get("PublicValue").and_then(|v| v.as_str()).unwrap_or("0").parse().unwrap_or(0),
                fee_pool: r.get("FeePool").and_then(|v| v.as_str()).unwrap_or("0").parse().unwrap_or(0),
                unlock_height: r.get("UnlockHeight").and_then(|v| v.as_str()).unwrap_or("0").parse().unwrap_or(0),
                round_id: r.get("RoundId").and_then(|v| v.as_str()).unwrap_or("0").parse().unwrap_or(0),
            });
        }

        Ok(result)
    }

    /// Look up whether a specific BTC address is already registered on-chain.
    /// Returns the twilight address it is mapped to, or None if not registered.
    pub async fn fetch_registered_btc_by_address(
        &self,
        btc_address: &str,
    ) -> anyhow::Result<Option<BtcDepositInfo>> {
        let url = format!(
            "{}/twilight-project/nyks/bridge/registered_btc_deposit_address/{}",
            self.chain_config.lcd_endpoint, btc_address
        );
        let client = Client::new();
        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 404 || status.as_u16() == 400 {
                return Ok(None);
            }
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Failed to query BTC registration ({}): {}",
                status,
                body
            ));
        }

        let json: Value = response.json().await?;
        let deposit_address = json
            .get("depositAddress")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let twilight_address = json
            .get("twilightDepositAddress")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if deposit_address.is_empty() {
            return Ok(None);
        }

        Ok(Some(BtcDepositInfo {
            btc_deposit_address: deposit_address,
            twilight_address,
            is_confirmed: false,
        }))
    }

    /// Query the on-chain deposit registration status for this wallet's twilight address.
    /// Returns the registered BTC deposit address info including confirmation status.
    pub async fn fetch_deposit_status(&self) -> anyhow::Result<Option<BtcDepositInfo>> {
        let url = format!(
            "{}/twilight-project/nyks/bridge/registered_btc_deposit_address_by_twilight_address/{}",
            self.chain_config.lcd_endpoint, self.twilightaddress
        );
        let client = Client::new();
        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 404 {
                return Ok(None);
            }
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to query deposit status ({}): {}", status, body));
        }

        let json: Value = response.json().await?;
        let deposit_address = json
            .get("depositAddress")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let twilight_address = json
            .get("twilightDepositAddress")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if deposit_address.is_empty() {
            return Ok(None);
        }

        Ok(Some(BtcDepositInfo {
            btc_deposit_address: deposit_address,
            twilight_address,
            is_confirmed: false, // this endpoint doesn't return confirmation; use fetch_all_deposits
        }))
    }

    /// Query all registered BTC deposit addresses and find entries matching this wallet.
    /// Returns full deposit info including confirmation status.
    pub async fn fetch_deposit_details(&self) -> anyhow::Result<Vec<BtcDepositDetail>> {
        let url = format!(
            "{}/twilight-project/nyks/bridge/registered_btc_deposit_addresses",
            self.chain_config.lcd_endpoint
        );
        let client = Client::new();
        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to fetch deposit addresses ({}): {}", status, body));
        }

        let json: Value = response.json().await?;
        let addresses = json
            .get("addresses")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("Missing addresses field in response"))?;

        let mut results = Vec::new();
        for a in addresses {
            let twilight_addr = a
                .get("twilightAddress")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if twilight_addr == self.twilightaddress {
                results.push(BtcDepositDetail {
                    btc_deposit_address: a.get("btcDepositAddress").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    btc_satoshi_amount: a.get("btcSatoshiTestAmount").and_then(|v| v.as_str()).unwrap_or("0").parse().unwrap_or(0),
                    twilight_staking_amount: a.get("twilightStakingAmount").and_then(|v| v.as_str()).unwrap_or("0").parse().unwrap_or(0),
                    twilight_address: twilight_addr.to_string(),
                    is_confirmed: a.get("isConfirmed").and_then(|v| v.as_bool()).unwrap_or(false),
                    creation_block_height: a.get("CreationTwilightBlockHeight").and_then(|v| v.as_str()).unwrap_or("0").parse().unwrap_or(0),
                });
            }
        }

        Ok(results)
    }

    /// Query the on-chain withdrawal request status from LCD.
    /// Parameters match the LCD query: reserve_id, btc_address, withdraw_amount.
    pub async fn fetch_withdrawal_status(
        &self,
        reserve_id: u64,
        btc_address: &str,
        withdraw_amount: u64,
    ) -> anyhow::Result<Option<BtcWithdrawStatus>> {
        let url = format!(
            "{}/twilight-project/nyks/volt/btc_withdraw_request/{}?reserveId={}&btcAddress={}&withdrawAmount={}",
            self.chain_config.lcd_endpoint, self.twilightaddress, reserve_id, btc_address, withdraw_amount
        );
        let client = Client::new();
        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 404 || status.as_u16() == 400 {
                return Ok(None);
            }
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Failed to query withdrawal status ({}): {}",
                status,
                body
            ));
        }

        let json: Value = response.json().await?;
        let req = match json.get("BtcWithdrawRequest") {
            Some(r) => r,
            None => return Ok(None),
        };

        Ok(Some(BtcWithdrawStatus {
            withdraw_identifier: req
                .get("withdrawIdentifier")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            withdraw_address: req
                .get("withdrawAddress")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            withdraw_reserve_id: req
                .get("withdrawReserveId")
                .and_then(|v| v.as_str())
                .unwrap_or("0")
                .to_string(),
            withdraw_amount: req
                .get("withdrawAmount")
                .and_then(|v| v.as_str())
                .unwrap_or("0")
                .to_string(),
            twilight_address: req
                .get("twilightAddress")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            is_confirmed: req
                .get("isConfirmed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            creation_twilight_block_height: req
                .get("CreationTwilightBlockHeight")
                .and_then(|v| v.as_str())
                .unwrap_or("0")
                .to_string(),
        }))
    }

    /// Fetch account details from the Twilight indexer, including deposits and withdrawals.
    pub async fn fetch_account_from_indexer(&self) -> anyhow::Result<IndexerAccountInfo> {
        let url = format!(
            "{}/api/accounts/{}",
            crate::config::TWILIGHT_INDEXER_URL.as_str(),
            self.twilightaddress
        );
        let client = Client::new();
        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Indexer request failed ({}): {}", status, body));
        }

        let json: Value = response.json().await?;

        let account = json.get("account").ok_or_else(|| anyhow!("Missing account field"))?;
        let address = account.get("address").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let balance = account.get("balance").and_then(|v| v.as_str()).unwrap_or("0").to_string();
        let tx_count = account.get("txCount").and_then(|v| v.as_u64()).unwrap_or(0);
        let first_seen = account.get("firstSeen").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let last_seen = account.get("lastSeen").and_then(|v| v.as_str()).unwrap_or("").to_string();

        let balances = json
            .get("balances")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|b| IndexerBalance {
                        denom: b.get("denom").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        amount: b.get("amount").and_then(|v| v.as_str()).unwrap_or("0").to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let deposits = json
            .get("deposits")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|d| IndexerDeposit {
                        id: d.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
                        tx_hash: d.get("txHash").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        block_height: d.get("blockHeight").and_then(|v| v.as_u64()).unwrap_or(0),
                        reserve_address: d.get("reserveAddress").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        deposit_amount: d.get("depositAmount").and_then(|v| v.as_str()).unwrap_or("0").to_string(),
                        btc_height: d.get("btcHeight").and_then(|v| v.as_str()).unwrap_or("0").to_string(),
                        btc_hash: d.get("btcHash").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        votes: d.get("votes").and_then(|v| v.as_u64()).unwrap_or(0),
                        confirmed: d.get("confirmed").and_then(|v| v.as_bool()).unwrap_or(false),
                        created_at: d.get("createdAt").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let withdrawals = json
            .get("withdrawals")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|w| IndexerWithdrawal {
                        id: w.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
                        withdraw_identifier: w.get("withdrawIdentifier").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                        withdraw_address: w.get("withdrawAddress").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        withdraw_reserve_id: w.get("withdrawReserveId").and_then(|v| v.as_str()).unwrap_or("0").to_string(),
                        withdraw_amount: w.get("withdrawAmount").and_then(|v| v.as_str()).unwrap_or("0").to_string(),
                        is_confirmed: w.get("isConfirmed").and_then(|v| v.as_bool()).unwrap_or(false),
                        block_height: w.get("blockHeight").and_then(|v| v.as_u64()).unwrap_or(0),
                        created_at: w.get("createdAt").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        updated_at: w.get("updatedAt").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(IndexerAccountInfo {
            address,
            balance,
            tx_count,
            first_seen,
            last_seen,
            balances,
            deposits,
            withdrawals,
        })
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

pub async fn get_test_tokens(wallet: &mut Wallet) -> anyhow::Result<()> {
    if crate::config::NETWORK_TYPE.as_str() == "mainnet" {
        return Err(anyhow!("get_test_tokens is only available on testnet. Use register-btc for mainnet deposits."));
    }

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
            info!("You may need to wait or try again later");
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
                debug!("BTC Address: {}", &wallet.btc_address);
                info!("waiting for registered BTC deposit address to appear on-chain");
                sleep(Duration::from_secs(10)).await;
                info!("Minting test BTC...");
                mint_sats(
                    &wallet.twilightaddress,
                    &wallet.chain_config.faucet_endpoint,
                )
                .await
                .unwrap_or_else(|e| {
                    error!("Failed to mint satoshis: {}", e);
                    info!("You may need to restart the process again or try again later");
                });
                wallet.btc_address_registered = true;
            }
            Err(e) => {
                error!("Failed to register BTC deposit address: {}", e);
                debug!("BTC Address: {}", &wallet.btc_address);
                info!("You may need to restart the process again or try again later");
            }
        };
    } else if balance.sats < 50000 {
        debug!("Skipping register BTC deposit address because sats is less than 50000");
        info!("Minting test BTC...");
        mint_sats(
            &wallet.twilightaddress,
            &wallet.chain_config.faucet_endpoint,
        )
        .await
        .unwrap_or_else(|e| {
            error!("Failed to mint satoshis: {}", e);
            info!("You may need to restart the process again or try again later");
        });
    } else {
        info!("Skipping minting test BTC because sats is greater than 50000");
    }
    info!("waiting for updated sats balance to appear on-chain");
    sleep(Duration::from_secs(10)).await;
    let balance = wallet.update_balance().await?;
    debug!("new balance: {:?}", balance);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_wallet_from_mnemonic() {
        let mnemonic = "test test test test test test test test test test test junk";
        let wallet = Wallet::from_mnemonic(mnemonic, None).expect("Failed to import wallet");
        let ct = coin_type();
        println!(
            "NETWORK_TYPE:       {}",
            std::env::var("NETWORK_TYPE").unwrap_or("testnet".to_string())
        );
        println!("Coin type:          {}", ct);
        println!("Derivation path:    m/44'/{ct}'/0'/0/0");
        println!("Wallet address:     {}", wallet.twilightaddress);
        println!("Wallet BTC address: {}", wallet.btc_address);
        println!("Public key hex:     {}", hex::encode(&wallet.public_key));
    }
}
