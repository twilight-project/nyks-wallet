//! OrderWallet: trading and lending interface for the relayer using ZkOS accounts.
//!
//! Provides high-level operations to:
//! - Fund ZkOS trading accounts from the on-chain wallet
//! - Move balances across ZkOS accounts (single and multiple receivers)
//! - Open/close/cancel trader and lend orders via the relayer
//! - Query order states with retry helpers
//! - Optionally persist wallet, ZK accounts, UTXOs, and request IDs in a database
use std::collections::HashMap;

use crate::{
    config::{EndpointConfig, RelayerEndPointConfig},
    error::{Result as WalletResult, WalletError},
    relayer_module::{
        self, fetch_tx_hash_with_retry, fetch_utxo_details_with_retry,
        relayer_api::RelayerJsonRpcClient,
        relayer_order::{
            cancel_trader_order, close_lend_order, close_trader_order, create_lend_order,
            create_trader_order,
        },
    },
    wallet::Wallet,
    zkos_accounts::{
        encrypted_account::{DERIVATION_MESSAGE, KeyManager},
        zkaccount::ZkAccountDB,
    },
};

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::database::{DatabaseManager, WalletList, connection::run_migrations_once};
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::security::SecurePassword;
use log::{debug, error};
use relayer_module::utils::{TxResult, build_and_sign_msg_mint_burn_trading_btc, send_tx_to_chain};
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use twilight_client_sdk::{
    quisquislib::RistrettoSecretKey,
    relayer::{query_lend_order_zkos, query_trader_order_zkos},
    relayer_rpcclient::method::UtxoDetailResponse,
    relayer_types::{
        LendOrder, OrderStatus, OrderType, PositionType, QueryLendOrderZkos, QueryTraderOrderZkos,
        TraderOrder,
    },
    transaction::{Receiver, Sender},
    transfer::{
        create_private_transfer_transaction_single_source_multiple_recievers,
        create_private_transfer_tx_single,
    },
    zkvm::IOType,
};

/// One-based index of a ZkOS account tracked by `ZkAccountDB`.
pub type AccountIndex = u64;
pub type Balance = u64;
/// Relayer request ID string returned after submitting an order.
pub type RequestId = String;
pub type AccountBalance = (AccountIndex, Balance);
#[derive(Debug, Clone, Serialize)]
/// High-level wallet orchestrator for relayer trading/lending using ZkOS accounts.
pub struct OrderWallet {
    pub wallet: Wallet,
    pub zk_accounts: ZkAccountDB,
    pub chain_id: String,
    #[serde(skip)]
    seed: SecretString,
    pub utxo_details: HashMap<AccountIndex, UtxoDetailResponse>,
    pub request_ids: HashMap<AccountIndex, RequestId>,
    #[serde(skip)]
    pub relayer_api_client: RelayerJsonRpcClient,
    pub relayer_endpoint_config: RelayerEndPointConfig,
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    #[serde(skip)]
    db_manager: Option<DatabaseManager>,
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    #[serde(skip)]
    wallet_password: Option<SecretString>,
}

impl OrderWallet {
    /// Internal constructor helper that wires endpoint configs and the relayer client,
    /// derives the ZkOS seed from the wallet, and initializes runtime caches.
    fn init(
        wallet: Wallet,
        zk_accounts: ZkAccountDB,
        endpoint_config: EndpointConfig,
    ) -> WalletResult<Self> {
        let relayer_endpoint_config = endpoint_config.to_relayer_endpoint_config();
        let relayer_api_client =
            RelayerJsonRpcClient::new(&relayer_endpoint_config.relayer_api_endpoint)
                .map_err(|e| WalletError::RelayerClient(e.to_string()))?;
        let seed = wallet
            .get_zk_account_seed(&endpoint_config.chain_id, DERIVATION_MESSAGE)
            .map_err(|e| WalletError::ZkAccountSeedNotFound(e.to_string()))?;

        Ok(Self {
            wallet,
            zk_accounts,
            chain_id: endpoint_config.chain_id,
            seed: seed,
            utxo_details: HashMap::new(),
            request_ids: HashMap::new(),
            relayer_api_client,
            relayer_endpoint_config,
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            db_manager: None,
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            wallet_password: None,
        })
    }

    /// Create a new `OrderWallet` with a freshly generated base `Wallet`.
    /// If `endpoint_config` is `None`, defaults are used.
    /// The base wallet generates a new mnemonic and prints it once to the TTY.
    pub fn new(endpoint_config: Option<EndpointConfig>) -> WalletResult<Self> {
        let endpoint_config = endpoint_config.unwrap_or(EndpointConfig::default());
        let wallet_endpoint_config = endpoint_config.to_wallet_endpoint_config();
        let wallet = Wallet::new(Some(wallet_endpoint_config))
            .map_err(|e| WalletError::WalletCreation(e.to_string()))?;
        let zk_accounts = ZkAccountDB::new();
        Self::init(wallet, zk_accounts, endpoint_config)
    }

    /// Import an `OrderWallet` from an existing mnemonic, preserving keys and addresses.
    pub fn import_from_mnemonic(
        mnemonic: &str,
        endpoint_config: Option<EndpointConfig>,
    ) -> Result<Self, String> {
        let endpoint_config = endpoint_config.unwrap_or(EndpointConfig::default());
        let relayer_endpoint_config = endpoint_config.to_relayer_endpoint_config();
        let wallet_endpoint_config = endpoint_config.to_wallet_endpoint_config();
        let wallet = Wallet::from_mnemonic(mnemonic, Some(wallet_endpoint_config))
            .map_err(|e| e.to_string())?;
        let zk_accounts = ZkAccountDB::new();
        let utxo_details = HashMap::new();
        let request_ids = HashMap::new();
        let relayer_api_client =
            RelayerJsonRpcClient::new(&relayer_endpoint_config.relayer_api_endpoint)
                .map_err(|e| e.to_string())?;
        let seed = wallet.get_zk_account_seed(&endpoint_config.chain_id, DERIVATION_MESSAGE)?;
        Ok(Self {
            wallet,
            zk_accounts,
            chain_id: endpoint_config.chain_id,
            seed,
            utxo_details,
            request_ids,
            relayer_api_client,
            relayer_endpoint_config,
            db_manager: None,
            wallet_password: None,
        })
    }

    // deafault feature is sqlite, if postgresql is enabled, then use postgresql
    // mnemonic will be securely printed for the first time and then deleted from memory and will not be stored in the database or any other storage
    /// Enable database persistence. Returns a cloned instance with DB enabled.
    /// Password resolution: explicit Some → env NYKS_WALLET_PASSPHRASE → interactive prompt.
    /// If `wallet_id` is None, defaults to the wallet's Twilight address.
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn with_db(
        &mut self,
        wallet_password: Option<SecretString>,
        wallet_id: Option<String>,
    ) -> Result<Self, String> {
        // look for env var NYKS_WALLET_PASSPHRASE for password, if not found then prompt for password

        let password = match wallet_password {
            Some(pwd) => pwd,
            None => {
                SecurePassword::get_passphrase_with_prompt(
                    "Could not find passphrase from environment, \nplease enter wallet encryption password: ",
                )
                .map_err(|e| e.to_string())?
            }
        };
        self.enable_database_persistence(Some(password), wallet_id)?;
        Ok(self.clone())
    }

    /// Load OrderWallet from DB by `wallet_id`. If `password` is None, it will
    /// resolve via env/prompt. Also loads Zk accounts, UTXO details, and request IDs.
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn load_from_db(
        wallet_id: String,
        password: Option<SecretString>,
        db_url: Option<String>,
    ) -> Result<OrderWallet, String> {
        let pool = crate::database::connection::init_pool(db_url)?;
        run_migrations_once(&pool)?;

        let db_manager = DatabaseManager::new(wallet_id, pool);
        let secure_password;
        let wallet = match password {
            Some(pwd) => {
                secure_password = pwd.clone();
                db_manager.load_encrypted_wallet(&pwd)?
            }
            None => {
                let pwd = SecurePassword::get_passphrase_with_prompt(
                    "Could not find passphrase from environment, \nplease enter wallet encryption password: ",
                )
                .map_err(|e| format!("Failed to get password: {}", e))?;
                secure_password = pwd.clone();
                db_manager.load_encrypted_wallet(&pwd)?
            }
        };

        // Load zk accounts
        let zk_accounts = db_manager.load_all_zk_accounts()?;
        let max_account_index = db_manager.get_max_account_index()?;
        let zk_accounts_db = ZkAccountDB {
            accounts: zk_accounts,
            index: max_account_index,
        };

        let mut order_wallet = OrderWallet::init(wallet, zk_accounts_db, EndpointConfig::default())
            .map_err(|e| e.to_string())?;
        order_wallet.wallet_password = Some(secure_password);
        order_wallet.db_manager = Some(db_manager);
        order_wallet.load_all_utxo_details_from_db()?;
        order_wallet.load_all_request_ids_from_db()?;

        Ok(order_wallet)
    }

    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn get_wallet_list_from_db(db_url: Option<String>) -> Result<Vec<WalletList>, String> {
        let pool = crate::database::connection::init_pool(db_url)?;
        run_migrations_once(&pool)?;
        let wallet_list = DatabaseManager::get_wallet_list(&pool)?;
        Ok(wallet_list)
    }

    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn get_wallet_id_from_db(wallet_id: &str, db_url: Option<String>) -> Result<bool, String> {
        let pool = crate::database::connection::init_pool(db_url)?;
        run_migrations_once(&pool)?;
        DatabaseManager::check_wallet_id_exists(&pool, wallet_id)
    }

    /// Derive a child secret key for the given account index from the ZkOS seed.
    pub fn get_secret_key(&self, index: AccountIndex) -> RistrettoSecretKey {
        let key_manager = KeyManager::from_cosmos_signature(self.seed.expose_secret().as_bytes());
        key_manager.derive_child_key(index)
    }
    /// Get last stored request ID for the account; errors if none exists.
    pub fn request_id(&self, index: AccountIndex) -> Result<&str, String> {
        self.request_ids
            .get(&index)
            .map(|s| s.as_str())
            .ok_or(format!("Request ID not found for account index: {}", index))
    }

    /// Ensure the account exists on-chain, has IOType::Coin, and a non-zero balance.
    pub fn ensure_coin_onchain(&self, index: AccountIndex) -> Result<(), String> {
        let a = self
            .zk_accounts
            .get_account(&index)
            .map_err(|e| e.to_string())?;
        if !a.on_chain || a.io_type != IOType::Coin || a.balance == 0 {
            return Err(format!("Account is not on chain or not a coin account"));
        }
        Ok(())
    }

    // -------------------------
    // Funding Operations
    // -------------------------
    // Create a new zk_account and transfer sats from wallet to zk_account
    // Return the tx result (tx_hash, code) and the account index
    pub async fn funding_to_trading(&mut self, amount: u64) -> Result<(TxResult, u64), String> {
        let wallet_balance = self
            .wallet
            .update_balance()
            .await
            .map_err(|e| e.to_string())?;
        if wallet_balance.nyks > 0 && wallet_balance.sats >= amount {
            let account_index = self.zk_accounts.generate_new_account(amount, &self.seed)?;

            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            if let Ok(account) = self.zk_accounts.get_account(&account_index) {
                let _ = self.sync_zk_account_to_db(&account);
            }
            self.wallet
                .update_account_info()
                .await
                .map_err(|e| e.to_string())?;

            let wallet_on_chain_info = match self.wallet.account_info().await {
                Ok(account_info) => account_info,
                Err(e) => {
                    error!("Failed to get wallet account details on chain: {}", e);
                    return Err(e.to_string());
                }
            };
            let sequence = wallet_on_chain_info.account.sequence;
            let account_number = wallet_on_chain_info.account.account_number;
            let signed_tx = build_and_sign_msg_mint_burn_trading_btc(
                &self.wallet,
                &self.zk_accounts,
                account_index,
                sequence,
                account_number,
                amount,
            )?;
            let result =
                send_tx_to_chain(signed_tx, &self.wallet.chain_config.rpc_endpoint).await?;
            if result.code != 0 {
                return Err(format!("Failed to send tx to chain: {}", result.tx_hash));
            } else {
                let account_address = self.zk_accounts.get_account_address(&account_index)?;
                let utxo_detail =
                    fetch_utxo_details_with_retry(account_address, IOType::Coin).await?;

                self.utxo_details.insert(account_index, utxo_detail.clone());

                // Sync to database
                #[cfg(any(feature = "sqlite", feature = "postgresql"))]
                if let Err(e) = self.sync_utxo_detail_to_db(account_index, &utxo_detail) {
                    error!("Failed to sync UTXO detail to database: {}", e);
                }

                self.zk_accounts.update_on_chain(&account_index, true)?;

                #[cfg(any(feature = "sqlite", feature = "postgresql"))]
                if let Ok(account) = self.zk_accounts.get_account(&account_index) {
                    let _ = self.update_zk_account_in_db(&account);
                }
            }
            Ok((result, account_index))
        } else {
            error!("Insufficient balance");
            Err(format!("Insufficient balance"))
        }
    }
    //  -> Result<(TxResult, u64), String>
    pub async fn trading_to_trading(
        &mut self,
        index: AccountIndex,
    ) -> Result<AccountIndex, String> {
        self.ensure_coin_onchain(index)?;
        let sender_account = self.zk_accounts.get_account(&index)?;
        let sender_account_address = sender_account.account.clone();
        let amount = sender_account.balance;
        let new_account_index = self.zk_accounts.generate_new_account(amount, &self.seed)?;

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        if let Ok(account) = self.zk_accounts.get_account(&new_account_index) {
            let _ = self.sync_zk_account_to_db(&account);
        }

        let receiver_input_string = self.zk_accounts.get_account(&new_account_index)?.account;
        let utxo_detail =
            fetch_utxo_details_with_retry(sender_account_address, IOType::Coin).await?;
        let input = utxo_detail.get_input()?;
        let tx_wallet = create_private_transfer_tx_single(
            self.get_secret_key(index),
            input,
            receiver_input_string,
            amount,
            false,
            0,
            1u64,
        );

        let encrypt_scalar = tx_wallet.get_encrypt_scalar_hex();

        let response = tokio::task::spawn_blocking(move || {
            twilight_client_sdk::chain::tx_commit_broadcast_transaction(
                tx_wallet.get_tx().ok_or("Failed to get tx")?,
            )
        })
        .await
        .map_err(|e| format!("Failed to send RPC request: {}", e))?;
        debug!("response: {:?}", response);
        let utxo_detail = fetch_utxo_details_with_retry(
            self.zk_accounts.get_account_address(&new_account_index)?,
            IOType::Coin,
        )
        .await?;

        self.utxo_details
            .insert(new_account_index, utxo_detail.clone());
        self.utxo_details.remove(&index);

        // Sync to database
        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        {
            if let Err(e) = self.sync_utxo_detail_to_db(new_account_index, &utxo_detail) {
                error!("Failed to sync UTXO detail to database: {}", e);
            }
            if let Err(e) = self.remove_utxo_detail_from_db(index) {
                error!("Failed to remove UTXO detail from database: {}", e);
            }
        }
        self.zk_accounts.update_on_chain(&new_account_index, true)?;
        self.zk_accounts.update_on_chain(&index, false)?;
        let account = utxo_detail.output.to_quisquis_account()?;

        self.zk_accounts
            .update_qq_account(&new_account_index, account)?;
        self.zk_accounts
            .update_scalar(&new_account_index, &encrypt_scalar)?;

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        {
            if let Ok(account) = self.zk_accounts.get_account(&new_account_index) {
                let _ = self.update_zk_account_in_db(&account);
            }
            if let Ok(account) = self.zk_accounts.get_account(&index) {
                let _ = self.update_zk_account_in_db(&account);
            }
        }

        Ok(new_account_index)
    }

    /// Split a single Coin account into multiple new Coin accounts as specified by `balances`.
    /// Returns a vector of `(new_account_index, balance)` for each created account.
    /// Requirements:
    /// - Sender must be on-chain in Coin state and have at least sum(balances)
    /// - `balances` must be non-empty
    /// - Recommended to create at most 8 accounts per call due to tx size limits
    pub async fn trading_to_trading_multiple_accounts(
        &mut self,
        sender_account_index: AccountIndex,
        balances: Vec<Balance>,
    ) -> Result<Vec<AccountBalance>, String> {
        self.ensure_coin_onchain(sender_account_index)?;
        let sk = self.get_secret_key(sender_account_index);
        let input_sender = self
            .utxo_details
            .get(&sender_account_index)
            .ok_or("UTXO detail not found")?
            .get_input()?;
        let mut new_account_balances = Vec::new();
        let mut commitment_scalar_vec = Vec::new();
        let mut receiver_vec = Vec::new();
        let mut updated_reciever_balance_vec = Vec::new();
        let num_of_new_accounts = balances.len();
        let sender_transfering_amt = balances.iter().sum::<Balance>();
        let sender_account = self.zk_accounts.get_account(&sender_account_index)?;
        if sender_account.balance < sender_transfering_amt {
            return Err(format!("Insufficient balance"));
        }
        if num_of_new_accounts == 0 || num_of_new_accounts > 9 {
            return Err(format!("No new accounts to create"));
        }
        let updated_sender_balance = sender_account.balance - sender_transfering_amt;
        for balance in balances {
            let new_account_index = self.zk_accounts.generate_new_account(0, &self.seed)?;
            new_account_balances.push((new_account_index, balance));
            commitment_scalar_vec.push(
                self.zk_accounts
                    .get_account(&new_account_index)?
                    .get_scalar()?,
            );
            receiver_vec.push(Receiver::set_receiver(
                balance as i64,
                self.zk_accounts
                    .get_account(&new_account_index)?
                    .get_qq_account()?,
            ));
            updated_reciever_balance_vec.push(balance);
        }
        let sender_array = vec![Sender::set_sender(
            (sender_transfering_amt as i64) * -1,
            sender_account.get_qq_account()?,
            receiver_vec,
        )];
        // debug!("sender_array: {:?}", sender_array);
        // debug!("input_sender: {:?}", input_sender);
        // debug!("updated_sender_balance: {:?}", updated_sender_balance);
        // debug!(
        //     "updated_reciever_balance_vec: {:?}",
        //     updated_reciever_balance_vec
        // );
        // debug!("commitment_scalar_vec: {:?}", commitment_scalar_vec);
        // debug!("sender_transfering_amt: {:?}", sender_transfering_amt);
        let tx_wallet = create_private_transfer_transaction_single_source_multiple_recievers(
            sender_array,
            input_sender,
            sk,
            vec![updated_sender_balance],
            updated_reciever_balance_vec,
            Some(&commitment_scalar_vec),
            1u64,
        )?;
        let tx = tx_wallet.get_tx().ok_or("Failed to get tx")?;
        let outputs = tx.get_tx_outputs();
        let encrypt_scalar = tx_wallet.get_encrypt_scalar();

        let response = tokio::task::spawn_blocking(move || {
            twilight_client_sdk::chain::tx_commit_broadcast_transaction(
                tx_wallet.get_tx().ok_or("Failed to get tx")?,
            )
        })
        .await
        .map_err(|e| format!("Failed to send RPC request: {}", e))?;

        debug!("response: {:?}", response);
        let mut i = 0;
        for (new_account_index, balance) in new_account_balances.iter() {
            let utxo_detail = fetch_utxo_details_with_retry(
                self.zk_accounts.get_account_address(new_account_index)?,
                IOType::Coin,
            )
            .await?;
            self.utxo_details
                .insert(*new_account_index, utxo_detail.clone());
            self.zk_accounts.update_on_chain(new_account_index, true)?;
            self.zk_accounts
                .update_balance(new_account_index, *balance)?;
            let account = utxo_detail.output.to_quisquis_account()?;
            self.zk_accounts
                .update_qq_account(new_account_index, account)?;
            self.zk_accounts
                .update_scalar(new_account_index, &encrypt_scalar[i])?;
            self.zk_accounts.update_account_key(
                new_account_index,
                &outputs[i + 1]
                    .as_output_data()
                    .get_owner_address()
                    .ok_or("Failed to get owner address")?,
            )?;
            i += 1;
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            if let Ok(account) = self.zk_accounts.get_account(new_account_index) {
                let _ = self.update_zk_account_in_db(&account);
            }
        }

        if updated_sender_balance > 0 {
            self.zk_accounts
                .update_balance(&sender_account_index, updated_sender_balance)?;
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            if let Ok(account) = self.zk_accounts.get_account(&sender_account_index) {
                let _ = self.update_zk_account_in_db(&account);
            }
            let utxo_detail = fetch_utxo_details_with_retry(
                self.zk_accounts
                    .get_account_address(&sender_account_index)?,
                IOType::Coin,
            )
            .await?;
            self.utxo_details
                .insert(sender_account_index, utxo_detail.clone());
            let account = utxo_detail.output.to_quisquis_account()?;
            self.zk_accounts
                .update_qq_account(&sender_account_index, account)?;
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            if let Err(e) = self.sync_utxo_detail_to_db(sender_account_index, &utxo_detail) {
                error!("Failed to sync UTXO detail to database: {}", e);
            }
        } else {
            self.zk_accounts.update_balance(&sender_account_index, 0)?;
            self.zk_accounts
                .update_on_chain(&sender_account_index, false)?;
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            if let Ok(account) = self.zk_accounts.get_account(&sender_account_index) {
                let _ = self.update_zk_account_in_db(&account);
            }
            self.utxo_details.remove(&sender_account_index);
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            if let Err(e) = self.remove_utxo_detail_from_db(sender_account_index) {
                error!("Failed to remove UTXO detail from database: {}", e);
            }
        }

        Ok(new_account_balances)
    }
    // -------------------------
    // Trader Order Operations
    // -------------------------

    pub async fn open_trader_order(
        &mut self,
        index: AccountIndex,
        order_type: OrderType,
        order_side: PositionType,
        entry_price: u64,
        leverage: u64,
    ) -> Result<String, String> {
        self.ensure_coin_onchain(index)?;
        if leverage == 0 || leverage > 50 {
            return Err("Leverage must be greater than 0 and less than 50".to_string());
        }
        if entry_price == 0 {
            return Err("Entry price must be greater than 0".to_string());
        }
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let _utxo_detail =
            fetch_utxo_details_with_retry(account_address.clone(), IOType::Coin).await?;

        let secret_key = self.get_secret_key(index);
        let r_scalar = self.zk_accounts.get_account(&index)?.get_scalar()?;
        let initial_margin = self.zk_accounts.get_account(&index)?.balance;
        let position_value = initial_margin
            .checked_mul(leverage)
            .ok_or_else(|| "position_value overflow".to_string())?;
        let position_size = position_value
            .checked_mul(entry_price)
            .ok_or_else(|| "position_size overflow".to_string())?;
        let request_id = create_trader_order(
            secret_key,
            r_scalar,
            initial_margin,
            order_side,
            order_type.clone(),
            leverage,
            entry_price,
            position_value,
            position_size,
            &self.relayer_endpoint_config.relayer_program_json_path,
            account_address.clone(),
            &self.relayer_api_client,
        )
        .await?;
        debug!(
            "inserting request_id: {:?} for account index: {:?}",
            request_id, index
        );
        self.request_ids.insert(index, request_id.clone());

        // Sync to database
        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        if let Err(e) = self.sync_request_id_to_db(index, &request_id) {
            error!("Failed to sync request ID to database: {}", e);
        }

        if order_type == OrderType::LIMIT {
        } else {
            let utxo_detail = fetch_utxo_details_with_retry(account_address, IOType::Memo).await?;

            self.utxo_details.insert(index, utxo_detail.clone());

            // Sync to database
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            if let Err(e) = self.sync_utxo_detail_to_db(index, &utxo_detail) {
                error!("Failed to sync UTXO detail to database: {}", e);
            }
        }

        self.zk_accounts.update_io_type(&index, IOType::Memo)?;

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        if let Ok(account) = self.zk_accounts.get_account(&index) {
            let _ = self.update_zk_account_in_db(&account);
        }

        Ok(request_id)
    }

    pub async fn close_trader_order(
        &mut self,
        index: AccountIndex,
        order_type: OrderType,
        execution_price: f64,
    ) -> Result<String, String> {
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let secret_key = self.get_secret_key(index);
        let request_id = self.request_id(index)?;
        let tx_hash = fetch_tx_hash_with_retry(request_id, &self.relayer_api_client).await?;
        if tx_hash.order_status != OrderStatus::FILLED {
            return Err(format!(
                "Order is not filled, status: {}",
                tx_hash.order_status.to_str()
            ));
        }
        let output = tx_hash.get_output()?;
        let request_id = close_trader_order(
            output,
            &secret_key,
            account_address.clone(),
            tx_hash.order_id,
            order_type,
            execution_price,
            &self.relayer_api_client,
        )
        .await?;
        let tx_hash = fetch_tx_hash_with_retry(&request_id, &self.relayer_api_client).await?;
        if tx_hash.order_status != OrderStatus::SETTLED {
            return Err(format!(
                "Order is not settled, status: {}",
                tx_hash.order_status.to_str()
            ));
        }
        let utxo_detail = fetch_utxo_details_with_retry(account_address, IOType::Coin).await?;
        self.utxo_details.insert(index, utxo_detail.clone());

        // Sync to database
        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        if let Err(e) = self.sync_utxo_detail_to_db(index, &utxo_detail) {
            error!("Failed to sync UTXO detail to database: {}", e);
        }

        let trader_order = self.query_trader_order(index).await?;
        self.zk_accounts
            .update_balance(&index, trader_order.available_margin as u64)?;
        debug!(
            "trader_order available_margin: {:?}",
            trader_order.available_margin as u64
        );
        self.zk_accounts.update_io_type(&index, IOType::Coin)?;
        let account = utxo_detail.output.to_quisquis_account()?;
        self.zk_accounts.update_qq_account(&index, account)?;
        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        if let Ok(account) = self.zk_accounts.get_account(&index) {
            let _ = self.update_zk_account_in_db(&account);
        }
        Ok(request_id)
    }

    pub async fn query_trader_order(&mut self, index: AccountIndex) -> Result<TraderOrder, String> {
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let secret_key = self.get_secret_key(index);
        debug!(
            "query_trader_order: account_address: {:?} for account index: {:?}",
            account_address, index
        );
        let query_order = query_trader_order_zkos(
            account_address.clone(),
            &secret_key,
            account_address.clone(),
            "PENDING".to_string(),
        );
        let query_order_zkos = QueryTraderOrderZkos::decode_from_hex_string(query_order)?;
        let response = self
            .relayer_api_client
            .trader_order_info(query_order_zkos)
            .await
            .map_err(|e| e.to_string())?;

        Ok(response)
    }

    pub async fn cancel_trader_order(&mut self, index: AccountIndex) -> Result<String, String> {
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let secret_key = self.get_secret_key(index);
        let request_id = self.request_id(index)?;
        let tx_hash = fetch_tx_hash_with_retry(request_id, &self.relayer_api_client).await?;
        if tx_hash.order_status != OrderStatus::PENDING {
            return Err(format!(
                "Order is not pending, status: {}",
                tx_hash.order_status.to_str()
            ));
        }
        let request_id = cancel_trader_order(
            account_address.clone(),
            &secret_key,
            account_address.clone(),
            tx_hash.order_id,
            &self.relayer_api_client,
        )
        .await?;
        let tx_hash = fetch_tx_hash_with_retry(&request_id, &self.relayer_api_client).await?;
        if tx_hash.order_status != OrderStatus::CANCELLED {
            return Err(format!(
                "Order is not cancelled, status: {}",
                tx_hash.order_status.to_str()
            ));
        }

        self.zk_accounts.update_io_type(&index, IOType::Coin)?;

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        if let Ok(account) = self.zk_accounts.get_account(&index) {
            let _ = self.update_zk_account_in_db(&account);
        }

        Ok(request_id)
    }

    // -------------------------
    // Lend Order Operations
    // -------------------------

    pub async fn open_lend_order(&mut self, index: AccountIndex) -> Result<String, String> {
        self.ensure_coin_onchain(index)?;
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let _utxo_detail =
            fetch_utxo_details_with_retry(account_address.clone(), IOType::Coin).await?;

        let secret_key = self.get_secret_key(index);
        let scalar_hex: String = self.zk_accounts.get_account(&index)?.scalar.clone();
        let amount = self.zk_accounts.get_account(&index)?.balance;

        let request_id = create_lend_order(
            account_address.clone(),
            secret_key,
            amount,
            &self.relayer_endpoint_config.relayer_program_json_path,
            scalar_hex,
            &self.relayer_api_client,
        )
        .await?;
        self.request_ids.insert(index, request_id.clone());

        // Sync to database
        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        if let Err(e) = self.sync_request_id_to_db(index, &request_id) {
            error!("Failed to sync request ID to database: {}", e);
        }

        let utxo_detail = fetch_utxo_details_with_retry(account_address, IOType::Memo).await?;

        self.utxo_details.insert(index, utxo_detail.clone());

        // Sync to database
        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        if let Err(e) = self.sync_utxo_detail_to_db(index, &utxo_detail) {
            error!("Failed to sync UTXO detail to database: {}", e);
        }

        self.zk_accounts.update_io_type(&index, IOType::Memo)?;

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        if let Ok(account) = self.zk_accounts.get_account(&index) {
            let _ = self.update_zk_account_in_db(&account);
        }

        Ok(request_id)
    }

    pub async fn query_lend_order(&mut self, index: AccountIndex) -> Result<LendOrder, String> {
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let secret_key = self.get_secret_key(index);
        let query_order = query_lend_order_zkos(
            account_address.clone(),
            &secret_key,
            account_address.clone(),
            OrderStatus::LENDED.to_str(),
        );
        let query_order_zkos = QueryLendOrderZkos::decode_from_hex_string(query_order)?;
        let response = self
            .relayer_api_client
            .lend_order_info(query_order_zkos)
            .await
            .map_err(|e| e.to_string())?;

        Ok(response)
    }

    pub async fn close_lend_order(&mut self, index: AccountIndex) -> Result<String, String> {
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let secret_key = self.get_secret_key(index);
        let request_id = self.request_id(index)?;
        let tx_hash = fetch_tx_hash_with_retry(request_id, &self.relayer_api_client).await?;
        if tx_hash.order_status != OrderStatus::FILLED {
            return Err(format!(
                "Order is not filled, status: {}",
                tx_hash.order_status.to_str()
            ));
        }
        let output = tx_hash.get_output()?;
        let request_id = close_lend_order(
            output,
            &secret_key,
            account_address.clone(),
            tx_hash.order_id,
            OrderType::LEND,
            &self.relayer_api_client,
        )
        .await?;
        let tx_hash = fetch_tx_hash_with_retry(&request_id, &self.relayer_api_client).await?;
        if tx_hash.order_status != OrderStatus::SETTLED {
            return Err(format!(
                "Order is not settled, status: {}",
                tx_hash.order_status.to_str()
            ));
        }
        let utxo_detail = fetch_utxo_details_with_retry(account_address, IOType::Coin).await?;
        self.utxo_details.insert(index, utxo_detail.clone());

        // Sync to database
        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        if let Err(e) = self.sync_utxo_detail_to_db(index, &utxo_detail) {
            error!("Failed to sync UTXO detail to database: {}", e);
        }

        let lend_order = self.query_lend_order(index).await?;
        let balance = lend_order.new_lend_state_amount as u64;
        self.zk_accounts.update_balance(&index, balance)?;
        debug!("lend_order balance: {:?}", balance);
        self.zk_accounts.update_io_type(&index, IOType::Coin)?;

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        if let Ok(account) = self.zk_accounts.get_account(&index) {
            let _ = self.update_zk_account_in_db(&account);
        }

        Ok(request_id)
    }

    // -------------------------
    // Database Operations
    // -------------------------
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn sync_zk_account_to_db(
        &self,
        account: &crate::zkos_accounts::zkaccount::ZkAccount,
    ) -> Result<(), String> {
        if let Some(ref db_manager) = self.db_manager {
            db_manager.save_zk_account(account)?;
        }
        Ok(())
    }

    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn update_zk_account_in_db(
        &self,
        account: &crate::zkos_accounts::zkaccount::ZkAccount,
    ) -> Result<(), String> {
        if let Some(ref db_manager) = self.db_manager {
            db_manager.update_zk_account(account)?;
        }
        Ok(())
    }

    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn remove_zk_account_from_db(&self, account_index: u64) -> Result<(), String> {
        if let Some(ref db_manager) = self.db_manager {
            db_manager.remove_zk_account(account_index)?;
        }
        Ok(())
    }

    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn get_all_zk_accounts_from_db(
        &self,
    ) -> Result<HashMap<u64, crate::zkos_accounts::zkaccount::ZkAccount>, String> {
        if let Some(ref db_manager) = self.db_manager {
            db_manager.load_all_zk_accounts()
        } else {
            Err("Database manager not initialized".to_string())
        }
    }

    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    /// Enable database persistence with automatic password prompt
    pub fn enable_database_persistence(
        &mut self,
        wallet_password: Option<SecretString>,
        wallet_id: Option<String>,
    ) -> Result<(), String> {
        let wallet_password = match wallet_password {
            Some(password) => password,
            None => SecurePassword::get_passphrase_with_prompt(
                "Could not find passphrase from environment, \nplease enter wallet encryption password: ",
            )
            .map_err(|e| format!("Failed to get password: {}", e))?,
        };

        // Generate wallet ID from wallet address
        let wallet_id = match wallet_id {
            Some(id) => id,
            None => self.wallet.twilightaddress.clone(),
        };

        // Initialize database connection and run migrations
        let pool = crate::database::connection::init_pool(None)?;
        run_migrations_once(&pool)?;

        // Create database manager
        // let wallet_list = DatabaseManager::get_wallet_list(&pool)?;
        // if wallet_list.iter().any(|w| w.wallet_id == wallet_id) {
        //     return Err(format!("Wallet ID already exists: {}", wallet_id));
        // }
        if DatabaseManager::check_wallet_id_exists(&pool, &wallet_id)? {
            return Err(format!("Wallet ID already exists: {}", wallet_id));
        }
        let db_manager = DatabaseManager::new(wallet_id, pool);
        // Save encrypted wallet if password is provided

        db_manager.save_encrypted_wallet(&self.wallet, &wallet_password)?;

        // Save existing zk accounts
        for account in self.zk_accounts.get_all_accounts() {
            db_manager.save_zk_account(account)?;
        }

        self.db_manager = Some(db_manager);
        self.wallet_password = Some(wallet_password);
        Ok(())
    }

    /// Save the OrderWallet configuration to database
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn save_order_wallet_to_db(&self) -> Result<(), String> {
        if let Some(ref db_manager) = self.db_manager {
            if let Some(ref password) = self.wallet_password {
                db_manager.save_order_wallet(
                    &self.chain_id,
                    self.seed.expose_secret(),
                    &self.relayer_endpoint_config,
                    password.expose_secret(),
                )?;

                Ok(())
            } else {
                Err("No password available for encryption".to_string())
            }
        } else {
            Ok(()) // No database persistence enabled
        }
    }

    /// Sync UTXO detail to database
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn sync_utxo_detail_to_db(
        &self,
        account_index: u64,
        utxo_detail: &UtxoDetailResponse,
    ) -> Result<(), String> {
        if let Some(ref db_manager) = self.db_manager {
            db_manager.save_utxo_detail(account_index, utxo_detail)?;
        }
        Ok(())
    }

    /// Sync request ID to database
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn sync_request_id_to_db(
        &self,
        account_index: u64,
        request_id: &str,
    ) -> Result<(), String> {
        if let Some(ref db_manager) = self.db_manager {
            db_manager.save_request_id(account_index, request_id)?;
        }
        Ok(())
    }

    /// Load all UTXO details from database
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn load_all_utxo_details_from_db(&mut self) -> Result<(), String> {
        if let Some(ref db_manager) = self.db_manager {
            let utxo_details = db_manager.load_all_utxo_details()?;
            self.utxo_details = utxo_details;
        }
        Ok(())
    }

    /// Load all request IDs from database
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn load_all_request_ids_from_db(&mut self) -> Result<(), String> {
        if let Some(ref db_manager) = self.db_manager {
            let request_ids = db_manager.load_all_request_ids()?;
            self.request_ids = request_ids;
        }
        Ok(())
    }

    /// Remove UTXO detail from database
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn remove_utxo_detail_from_db(&self, account_index: u64) -> Result<(), String> {
        if let Some(ref db_manager) = self.db_manager {
            db_manager.remove_utxo_detail(account_index)?;
        }
        Ok(())
    }
    /// Remove request ID from database
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn remove_request_id_from_db(&self, account_index: u64) -> Result<(), String> {
        if let Some(ref db_manager) = self.db_manager {
            db_manager.remove_request_id(account_index)?;
        }
        Ok(())
    }
}

// -------------------------
// Drop
// -------------------------
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
impl Drop for OrderWallet {
    fn drop(&mut self) {
        if let Some(ref db_manager) = self.db_manager {
            // Save all current zk accounts to database
            for account in self.zk_accounts.get_all_accounts() {
                if let Err(e) = db_manager.save_zk_account(account) {
                    error!(
                        "Failed to persist zk_account {} during drop: {}",
                        account.index, e
                    );
                }
            }

            // Save encrypted wallet if password is available
            if let Some(ref password) = self.wallet_password {
                // if let Err(e) = db_manager.save_encrypted_wallet(&self.wallet, password) {
                //     error!("Failed to persist wallet during drop: {}", e);
                // }

                // Save OrderWallet configuration
                if let Err(e) = db_manager.save_order_wallet(
                    &self.chain_id,
                    self.seed.expose_secret(),
                    &self.relayer_endpoint_config,
                    password.expose_secret(),
                ) {
                    error!(
                        "Failed to persist OrderWallet configuration during drop: {}",
                        e
                    );
                }
            }

            // Save all UTXO details
            for (account_index, utxo_detail) in &self.utxo_details {
                if let Err(e) = db_manager.save_utxo_detail(*account_index, utxo_detail) {
                    error!(
                        "Failed to persist UTXO detail for account {} during drop: {}",
                        account_index, e
                    );
                }
            }

            // Save all request IDs
            for (account_index, request_id) in &self.request_ids {
                if let Err(e) = db_manager.save_request_id(*account_index, request_id) {
                    error!(
                        "Failed to persist request ID for account {} during drop: {}",
                        account_index, e
                    );
                }
            }

            debug!("OrderWallet data persisted to database during drop");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{get_test_tokens, relayer_module::fetch_tx_hash_with_retry};
    use log::info;
    use serial_test::serial;
    use std::sync::Once;
    use tokio::time::{Duration, sleep};
    use twilight_client_sdk::relayer_types::PositionType;
    static INIT: Once = Once::new();

    // This function initializes the logger for the tests.
    fn init_logger() {
        INIT.call_once(|| {
            // `is_test(true)` keeps the default filter at `trace`
            // and respects RUST_LOG if you set it.
            env_logger::builder().is_test(true).try_init().ok();
        });
    }
    async fn setup_wallet() -> Result<Wallet, String> {
        info!("Creating new wallet with random BTC address");
        let mut wallet = Wallet::new(None).map_err(|e| e.to_string())?;
        // info!("importing wallet from json");
        // let mut wallet = Wallet::import_from_json("test.json").map_err(|e| e.to_string())?;

        info!("Getting test tokens from faucet");
        match get_test_tokens(&mut wallet).await {
            Ok(_) => info!("Tokens received successfully"),
            Err(e) => return Err(e.to_string()),
        }

        // Give the faucet some time to finalize and the indexer to catch up.
        sleep(Duration::from_secs(5)).await;

        Ok(wallet)
    }
    // cargo test --no-default-features --features postgresql --lib -- relayer_module::order_wallet::tests::test_create_order --exact --show-output
    // cargo test --no-default-features --features sqlite --lib -- relayer_module::order_wallet::tests::test_create_order --exact --show-output
    // cargo test --all-features --lib -- relayer_module::order_wallet::tests::test_create_order --exact --show-output
    // #[cfg(feature = "sqlite")]
    #[tokio::test]
    #[serial]
    async fn test_create_order_complete_cycle() -> Result<(), String> {
        dotenv::dotenv().ok();
        unsafe {
            // std::env::set_var("DATABASE_URL", "./test.db");
            std::env::set_var("NYKS_WALLET_PASSPHRASE", "test1_password");
        }
        init_logger();
        let wallet = setup_wallet().await.map_err(|e| e.to_string())?;
        let zk_accounts = ZkAccountDB::new();

        let mut order_wallet = OrderWallet::init(wallet, zk_accounts, EndpointConfig::default())
            .map_err(|e| e.to_string())?;
        order_wallet.with_db(None, None)?;
        let (tx_result, account_index) = order_wallet.funding_to_trading(6000).await?;
        if tx_result.code != 0 {
            return Err(format!("Failed to send tx to chain: {}", tx_result.tx_hash));
        }

        let btc_price = order_wallet
            .relayer_api_client
            .btc_usd_price()
            .await
            .map_err(|e| e.to_string())?;
        info!("btc_price: {:?}", btc_price);
        let entry_price = btc_price.price as u64;
        let result = order_wallet
            .open_trader_order(
                account_index,
                OrderType::MARKET,
                PositionType::LONG,
                entry_price,
                10,
            )
            .await?;
        info!("result: {:?}", result);
        let tx_hash = fetch_tx_hash_with_retry(&result, &order_wallet.relayer_api_client).await?;
        info!("tx_hash: {:?}", tx_hash);
        assert_eq!(tx_hash.order_status, OrderStatus::FILLED);
        let response = order_wallet.query_trader_order(account_index).await?;
        info!("response: {:?}", response);
        assert_eq!(response.order_status, OrderStatus::FILLED);
        assert_eq!(
            order_wallet
                .zk_accounts
                .get_account(&account_index)?
                .io_type,
            IOType::Memo
        );

        let result = order_wallet
            .close_trader_order(account_index, OrderType::MARKET, 0.0)
            .await?;

        let tx_hash = fetch_tx_hash_with_retry(&result, &order_wallet.relayer_api_client).await?;
        assert_eq!(tx_hash.order_status, OrderStatus::SETTLED);
        let response = order_wallet.query_trader_order(account_index).await?;
        assert_eq!(response.order_status, OrderStatus::SETTLED);
        let zk_account = order_wallet.zk_accounts.get_account(&account_index)?;
        assert_eq!(zk_account.io_type, IOType::Coin);
        assert_eq!(zk_account.balance, response.available_margin as u64);
        let receiver_account_index = order_wallet.trading_to_trading(account_index).await?;
        assert_ne!(account_index, receiver_account_index);
        assert_eq!(
            order_wallet
                .zk_accounts
                .get_account(&account_index)?
                .on_chain,
            false
        );
        info!("receiver_account_index: {:?}", receiver_account_index);
        let result1 = order_wallet
            .open_trader_order(
                receiver_account_index,
                OrderType::MARKET,
                PositionType::LONG,
                entry_price,
                10,
            )
            .await?;
        info!("result1: {:?}", result1);
        let tx_hash1 = fetch_tx_hash_with_retry(&result1, &order_wallet.relayer_api_client).await?;
        info!("tx_hash1: {:?}", tx_hash1);
        assert_eq!(tx_hash1.order_status, OrderStatus::FILLED);
        let response = order_wallet
            .query_trader_order(receiver_account_index)
            .await?;
        info!("response: {:?}", response);
        assert_eq!(response.order_status, OrderStatus::FILLED);
        assert_eq!(
            order_wallet
                .zk_accounts
                .get_account(&receiver_account_index)?
                .io_type,
            IOType::Memo
        );
        let result2 = order_wallet
            .close_trader_order(receiver_account_index, OrderType::MARKET, 0.0)
            .await?;
        let tx_hash2 = fetch_tx_hash_with_retry(&result2, &order_wallet.relayer_api_client).await?;
        assert_eq!(tx_hash2.order_status, OrderStatus::SETTLED);
        let response2 = order_wallet
            .query_trader_order(receiver_account_index)
            .await?;
        assert_eq!(response2.order_status, OrderStatus::SETTLED);
        assert_eq!(
            order_wallet
                .zk_accounts
                .get_account(&receiver_account_index)?
                .io_type,
            IOType::Coin
        );
        let new_recever_index = order_wallet
            .trading_to_trading(receiver_account_index)
            .await?;
        assert_ne!(receiver_account_index, new_recever_index);
        assert_eq!(
            order_wallet
                .zk_accounts
                .get_account(&new_recever_index)?
                .io_type,
            IOType::Coin
        );
        let result3 = order_wallet
            .open_trader_order(
                new_recever_index,
                OrderType::MARKET,
                PositionType::LONG,
                entry_price,
                10,
            )
            .await?;
        let tx_hash3 = fetch_tx_hash_with_retry(&result3, &order_wallet.relayer_api_client).await?;
        info!("tx_hash3: {:?}", tx_hash3);
        assert_eq!(tx_hash3.order_status, OrderStatus::FILLED);
        let response3 = order_wallet.query_trader_order(new_recever_index).await?;
        info!("response3: {:?}", response3);
        assert_eq!(response3.order_status, OrderStatus::FILLED);
        assert_eq!(
            order_wallet
                .zk_accounts
                .get_account(&new_recever_index)?
                .io_type,
            IOType::Memo
        );
        let result4 = order_wallet
            .close_trader_order(new_recever_index, OrderType::MARKET, 0.0)
            .await?;
        debug!("result4: {:?}", result4);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_trading_to_trading() -> Result<(), String> {
        dotenv::dotenv().ok();
        init_logger();
        let wallet = setup_wallet().await.map_err(|e| e.to_string())?;
        let zk_accounts = ZkAccountDB::new();
        let mut order_wallet = OrderWallet::init(wallet, zk_accounts, EndpointConfig::default())
            .map_err(|e| e.to_string())?;
        let (tx_result, sender_account_index) = order_wallet.funding_to_trading(6000).await?;
        if tx_result.code != 0 {
            return Err(format!("Failed to send tx to chain: {}", tx_result.tx_hash));
        }
        let receiver_account_index = order_wallet
            .trading_to_trading(sender_account_index)
            .await?;
        assert_ne!(sender_account_index, receiver_account_index);
        assert_eq!(
            order_wallet
                .zk_accounts
                .get_account(&sender_account_index)?
                .on_chain,
            false
        );
        assert_eq!(
            order_wallet
                .zk_accounts
                .get_account(&receiver_account_index)?
                .balance,
            6000
        );
        assert_eq!(
            order_wallet
                .zk_accounts
                .get_account(&receiver_account_index)?
                .on_chain,
            true
        );
        let btc_price = order_wallet
            .relayer_api_client
            .btc_usd_price()
            .await
            .map_err(|e| e.to_string())?;
        info!("btc_price: {:?}", btc_price);
        let entry_price = btc_price.price as u64;
        let result = order_wallet
            .open_trader_order(
                receiver_account_index,
                OrderType::MARKET,
                PositionType::LONG,
                entry_price,
                10,
            )
            .await?;
        info!("result: {:?}", result);
        let tx_hash = fetch_tx_hash_with_retry(&result, &order_wallet.relayer_api_client).await?;
        info!("tx_hash: {:?}", tx_hash);
        assert_eq!(tx_hash.order_status, OrderStatus::FILLED);
        let response = order_wallet
            .query_trader_order(receiver_account_index)
            .await?;
        info!("response: {:?}", response);
        assert_eq!(response.order_status, OrderStatus::FILLED);
        assert_eq!(
            order_wallet
                .zk_accounts
                .get_account(&receiver_account_index)?
                .io_type,
            IOType::Memo
        );
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_open_lend_order() -> Result<(), String> {
        dotenv::dotenv().ok();
        init_logger();
        let wallet = setup_wallet().await.map_err(|e| e.to_string())?;
        let zk_accounts = ZkAccountDB::new();
        let mut order_wallet = OrderWallet::init(wallet, zk_accounts, EndpointConfig::default())
            .map_err(|e| e.to_string())?;
        let (tx_result, account_index) = order_wallet.funding_to_trading(6000).await?;
        if tx_result.code != 0 {
            return Err(format!("Failed to send tx to chain: {}", tx_result.tx_hash));
        }
        let result = order_wallet.open_lend_order(account_index).await?;
        let tx_hash = fetch_tx_hash_with_retry(&result, &order_wallet.relayer_api_client).await?;
        assert_eq!(tx_hash.order_status, OrderStatus::FILLED);
        let response = order_wallet.query_lend_order(account_index).await?;
        assert_eq!(response.order_status, OrderStatus::FILLED);
        assert_eq!(
            order_wallet
                .zk_accounts
                .get_account(&account_index)?
                .io_type,
            IOType::Memo
        );
        let result = order_wallet.close_lend_order(account_index).await?;
        let tx_hash = fetch_tx_hash_with_retry(&result, &order_wallet.relayer_api_client).await?;
        assert_eq!(tx_hash.order_status, OrderStatus::SETTLED);
        let response = order_wallet.query_lend_order(account_index).await?;
        assert_eq!(response.order_status, OrderStatus::SETTLED);
        let zk_account = order_wallet.zk_accounts.get_account(&account_index)?;
        assert_eq!(zk_account.io_type, IOType::Coin);
        assert_eq!(zk_account.balance, response.new_lend_state_amount as u64);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_create_order_with_cancel() -> Result<(), String> {
        dotenv::dotenv().ok();
        init_logger();
        let wallet = setup_wallet().await.map_err(|e| e.to_string())?;
        let zk_accounts = ZkAccountDB::new();

        let mut order_wallet = OrderWallet::init(wallet, zk_accounts, EndpointConfig::default())
            .map_err(|e| e.to_string())?;
        let (tx_result, account_index) = order_wallet.funding_to_trading(6000).await?;
        if tx_result.code != 0 {
            return Err(format!("Failed to send tx to chain: {}", tx_result.tx_hash));
        }

        let btc_price = order_wallet
            .relayer_api_client
            .btc_usd_price()
            .await
            .map_err(|e| e.to_string())?;
        info!("btc_price: {:?}", btc_price);
        let entry_price = btc_price.price as u64;
        let result = order_wallet
            .open_trader_order(
                account_index,
                OrderType::LIMIT,
                PositionType::LONG,
                entry_price - 1000,
                10,
            )
            .await?;
        let tx_hash = fetch_tx_hash_with_retry(&result, &order_wallet.relayer_api_client).await?;
        assert_eq!(tx_hash.order_status, OrderStatus::PENDING);
        let response = order_wallet.query_trader_order(account_index).await?;
        assert_eq!(response.order_status, OrderStatus::PENDING);
        assert_eq!(
            order_wallet
                .zk_accounts
                .get_account(&account_index)?
                .io_type,
            IOType::Memo
        );

        let result = order_wallet.cancel_trader_order(account_index).await?;

        let tx_hash = fetch_tx_hash_with_retry(&result, &order_wallet.relayer_api_client).await?;
        assert_eq!(tx_hash.order_status, OrderStatus::CANCELLED);
        let response = order_wallet.query_trader_order(account_index).await?;
        assert_eq!(response.order_status, OrderStatus::CANCELLED);
        let zk_account = order_wallet.zk_accounts.get_account(&account_index)?;
        assert_eq!(zk_account.io_type, IOType::Coin);
        assert_eq!(zk_account.balance, response.available_margin as u64);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_get_wallet_list_from_db() -> Result<(), String> {
        dotenv::dotenv().ok();
        init_logger();
        let wallet_list = OrderWallet::get_wallet_list_from_db(None).map_err(|e| e.to_string())?;
        println!("wallet_list: {:?}", wallet_list);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_get_wallet_list_from_db_with_db_url() -> Result<(), String> {
        dotenv::dotenv().ok();
        init_logger();
        let mut order_wallet = OrderWallet::new(None).map_err(|e| e.to_string())?;
        order_wallet.with_db(None, None)?;
        drop(order_wallet);
        let wallet_list = OrderWallet::get_wallet_list_from_db(None).map_err(|e| e.to_string())?;
        println!("wallet_list: {:?}", wallet_list);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_with_db() -> Result<(), String> {
        dotenv::dotenv().ok();
        init_logger();
        let mut order_wallet = OrderWallet::new(None).map_err(|e| e.to_string())?;
        order_wallet.with_db(None, None)?;
        let wallet_list = OrderWallet::get_wallet_list_from_db(None).map_err(|e| e.to_string())?;
        println!("wallet_list: {:?}", wallet_list);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_trading_to_trading_multiple_accounts() -> Result<(), String> {
        dotenv::dotenv().ok();
        init_logger();
        let wallet = setup_wallet().await.map_err(|e| e.to_string())?;
        let zk_accounts = ZkAccountDB::new();
        let mut order_wallet = OrderWallet::init(wallet, zk_accounts, EndpointConfig::default())
            .map_err(|e| e.to_string())?;
        // order_wallet.with_db(None, Some("test_wallet_multiple_accounts10".to_string()))?;
        let (tx_result, sender_account_index) = order_wallet.funding_to_trading(40000).await?;
        if tx_result.code != 0 {
            return Err(format!("Failed to send tx to chain: {}", tx_result.tx_hash));
        }
        let balances = vec![5000, 1000, 8000, 600];
        let new_account_balances = order_wallet
            .trading_to_trading_multiple_accounts(sender_account_index, balances)
            .await?;
        println!("new_account_balances: {:?}", new_account_balances);
        println!("zk_accounts: {:?}", order_wallet.zk_accounts);

        let btc_price = order_wallet
            .relayer_api_client
            .btc_usd_price()
            .await
            .map_err(|e| e.to_string())?;
        info!("btc_price: {:?}", btc_price);
        let entry_price = btc_price.price as u64;
        info!(
            "zk_accounts: {:?}",
            order_wallet
                .zk_accounts
                .get_account(&new_account_balances[0].0)
        );
        info!("waiting 10 seconds");
        sleep(Duration::from_secs(10)).await;
        let result = order_wallet
            .open_trader_order(
                new_account_balances[0].0,
                OrderType::MARKET,
                PositionType::LONG,
                entry_price,
                10,
            )
            .await?;
        let tx_hash = fetch_tx_hash_with_retry(&result, &order_wallet.relayer_api_client).await?;
        assert_eq!(tx_hash.order_status, OrderStatus::FILLED);
        let response = order_wallet
            .query_trader_order(new_account_balances[0].0)
            .await?;
        assert_eq!(response.order_status, OrderStatus::FILLED);
        assert_eq!(
            order_wallet
                .zk_accounts
                .get_account(&new_account_balances[0].0)?
                .io_type,
            IOType::Memo
        );

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_load_encrypted_wallet() -> Result<(), String> {
        dotenv::dotenv().ok();
        init_logger();
        let wallet_id = "load_encrypted_wallet_test".to_string();
        let mut order_wallet;
        let wallet_exists = OrderWallet::get_wallet_id_from_db(&wallet_id, None)?;
        if wallet_exists {
            order_wallet =
                OrderWallet::load_from_db(wallet_id, None, None).map_err(|e| e.to_string())?;
        } else {
            order_wallet = OrderWallet::new(None).map_err(|e| e.to_string())?;
            order_wallet
                .with_db(None, Some(wallet_id))
                .map_err(|e| e.to_string())?;
        }
        get_test_tokens(&mut order_wallet.wallet)
            .await
            .map_err(|e| e.to_string())?;
        drop(order_wallet);
        Ok(())
    }
}
