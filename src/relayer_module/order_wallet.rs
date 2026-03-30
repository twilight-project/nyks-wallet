//! OrderWallet: trading and lending interface for the relayer using ZkOS accounts.
//!
//! Provides high-level operations to:
//! - Fund ZkOS trading accounts from the on-chain wallet
//! - Move balances across ZkOS accounts (single and multiple receivers)
//! - Open/close/cancel trader and lend orders via the relayer
//! - Query order states with retry helpers
//! - Optionally persist wallet, ZK accounts, UTXOs, and request IDs in a database
use std::{collections::HashMap, time::Duration};

use std::sync::Arc;

use crate::{
    config::{EndpointConfig, RelayerEndPointConfig},
    error::{Result as WalletResult, WalletError},
    relayer_module::{
        self, fetch_removed_utxo_details_with_retry, fetch_tx_hash_with_account_address_retry,
        fetch_tx_hash_with_retry, fetch_utxo_details_with_retry,
        nonce_manager::NonceManager,
        relayer_api::RelayerJsonRpcClient,
        relayer_order::{
            cancel_trader_order, close_lend_order, close_trader_order_internal,
            close_trader_order_sltp_internal, create_lend_order, create_trader_order,
        },
    },
    wallet::Wallet,
    zkos_accounts::{
        encrypted_account::{KeyManager, DERIVATION_MESSAGE},
        zkaccount::ZkAccountDB,
    },
};

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::database::{connection::run_migrations_once, DatabaseManager, WalletList};
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::security::SecurePassword;
use log::{debug, error, info};
use relayer_module::utils::{build_and_sign_msg_mint_burn_trading_btc, send_tx_to_chain, TxResult};
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
        create_burn_message_transaction,
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
    #[serde(skip)]
    pub nonce_manager: Arc<NonceManager>,
    /// Accounts that were modified but not yet synced with on-chain UTXO state.
    #[serde(skip)]
    pub pending_sync: std::collections::HashSet<AccountIndex>,
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
            nonce_manager: Arc::new(NonceManager::new()),
            pending_sync: std::collections::HashSet::new(),
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
        let endpoint_config = endpoint_config.unwrap_or_default();
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
        let endpoint_config = endpoint_config.unwrap_or_default();
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
            nonce_manager: Arc::new(NonceManager::new()),
            pending_sync: std::collections::HashSet::new(),
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
        // index is the *next* account index to use, so it must be max + 1
        let next_index = if zk_accounts.is_empty() {
            0
        } else {
            max_account_index + 1
        };
        let zk_accounts_db = ZkAccountDB {
            accounts: zk_accounts,
            index: next_index,
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

    /// Get a reference to the database manager, if DB persistence is enabled.
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn get_db_manager(&self) -> Option<&DatabaseManager> {
        self.db_manager.as_ref()
    }

    /// Get a reference to the wallet password, if available.
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn get_wallet_password(&self) -> Option<&SecretString> {
        self.wallet_password.as_ref()
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

    /// Sync an account's on-chain UTXO state. Call this to complete a deferred
    /// sync after a `--no-wait` open or close operation.
    pub async fn sync_account_state(&mut self, index: AccountIndex) -> Result<(), String> {
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let io_type = self.zk_accounts.get_account(&index)?.io_type;
        let utxo_detail =
            fetch_utxo_details_with_retry(account_address, io_type).await?;
        self.utxo_details.insert(index, utxo_detail.clone());

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        if let Err(e) = self.sync_utxo_detail_to_db(index, &utxo_detail) {
            error!("Failed to sync UTXO detail to database: {}", e);
        }

        let account = utxo_detail.output.to_quisquis_account()?;
        self.zk_accounts.update_qq_account(&index, account)?;

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        {
            if let Ok(account) = self.zk_accounts.get_account(&index) {
                let _ = self.update_zk_account_in_db(&account);
            }
        }

        self.pending_sync.remove(&index);
        info!("Account {} synced with on-chain state", index);
        Ok(())
    }

    /// Sync all accounts that have pending state changes.
    pub async fn sync_all_pending(&mut self) -> Result<(), String> {
        let pending: Vec<AccountIndex> = self.pending_sync.iter().cloned().collect();
        for index in pending {
            if let Err(e) = self.sync_account_state(index).await {
                error!("Failed to sync account {}: {}", index, e);
            }
        }
        Ok(())
    }

    /// Sync the nonce manager from the on-chain account state.
    /// Call this before a batch of transactions, or periodically to
    /// re-anchor the local sequence counter.
    pub async fn sync_nonce(&self) -> Result<(), String> {
        self.nonce_manager
            .sync_from_chain(
                &self.wallet.chain_config.lcd_endpoint,
                &self.wallet.twilightaddress,
            )
            .await
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

            // Sync nonce manager and acquire a sequence number
            self.nonce_manager
                .sync_from_chain(
                    &self.wallet.chain_config.lcd_endpoint,
                    &self.wallet.twilightaddress,
                )
                .await?;
            let (sequence, account_number) = self.nonce_manager.acquire_next()?;

            let signed_tx = build_and_sign_msg_mint_burn_trading_btc(
                &self.wallet,
                &self.zk_accounts,
                account_index,
                sequence,
                account_number,
                amount,
                true,
            )?;
            let result =
                send_tx_to_chain(signed_tx, &self.wallet.chain_config.rpc_endpoint).await?;
            if result.code != 0 {
                self.nonce_manager.release(sequence);
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

                #[cfg(any(feature = "sqlite", feature = "postgresql"))]
                self.log_transfer_history(
                    "fund_to_trade",
                    None,
                    Some(account_index),
                    amount,
                    Some(&result.tx_hash),
                );
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
        self.zk_accounts.update_balance(&index, 0u64)?;
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
            let tx_hash = match &response {
                Ok(hash) => Some(hash.as_str()),
                Err(_) => None,
            };
            self.log_transfer_history(
                "trade_to_trade",
                Some(index),
                Some(new_account_index),
                sender_account.balance,
                tx_hash,
            );
        }

        Ok(new_account_index)
    }

    pub async fn trading_to_funding(&mut self, old_index: AccountIndex) -> Result<(), String> {
        self.ensure_coin_onchain(old_index)?;
        let index = self.trading_to_trading(old_index).await?;
        let sender_account = self.zk_accounts.get_account(&index)?;
        let amount = sender_account.balance;

        let sk = self.get_secret_key(index);
        let input = self
            .utxo_details
            .get(&index)
            .ok_or("UTXO detail not found")?
            .get_input()?;
        let encrypt_scalar = sender_account.scalar.clone();
        let tx_hex = create_burn_message_transaction(
            input,
            amount,
            encrypt_scalar,
            sk,
            sender_account.account.clone(),
        );
        let transaction = bincode::deserialize(&hex::decode(tx_hex).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let _tx_hash = tokio::task::spawn_blocking(move || {
            twilight_client_sdk::chain::tx_commit_broadcast_transaction(transaction)
        })
        .await
        .map_err(|e| format!("Failed to send RPC request: {}", e))?
        .map_err(|e| format!("Failed to get tx hash: {}", e))?;
        //waiting for the utxo to be removed
        let _ = fetch_removed_utxo_details_with_retry(
            self.zk_accounts.get_account_address(&index)?,
            IOType::Coin,
        )
        .await?;

        self.wallet
            .update_account_info()
            .await
            .map_err(|e| e.to_string())?;

        // Sync nonce manager and acquire a sequence number
        self.nonce_manager
            .sync_from_chain(
                &self.wallet.chain_config.lcd_endpoint,
                &self.wallet.twilightaddress,
            )
            .await?;
        let (sequence, account_number) = self.nonce_manager.acquire_next()?;

        let signed_tx = build_and_sign_msg_mint_burn_trading_btc(
            &self.wallet,
            &self.zk_accounts,
            index,
            sequence,
            account_number,
            amount,
            false,
        )?;
        let result = send_tx_to_chain(signed_tx, &self.wallet.chain_config.rpc_endpoint).await?;
        if result.code != 0 {
            self.nonce_manager.release(sequence);
            return Err(format!("Failed to send tx to chain: {}", result.tx_hash));
        }
        self.zk_accounts.update_on_chain(&index, false)?;
        self.zk_accounts.update_balance(&index, 0)?;
        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        {
            if let Ok(account) = self.zk_accounts.get_account(&index) {
                let _ = self.update_zk_account_in_db(&account);
            }
            self.log_transfer_history(
                "trade_to_fund",
                Some(old_index),
                None,
                amount,
                Some(&result.tx_hash),
            );
        }
        Ok(())
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
            if let Ok(account) = self.zk_accounts.get_account(&sender_account_index) {
                let _ = self.update_zk_account_in_db(&account);
            }
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

    /// Validate that the market is not halted.
    ///
    /// Close, cancel, and lend operations are allowed during HEALTHY and CLOSE_ONLY
    /// but rejected during HALT (per risk_engine.md).
    pub async fn validate_market_not_halted(&self) -> Result<(), String> {
        let stats = self
            .relayer_api_client
            .get_market_stats()
            .await
            .map_err(|e| format!("Failed to fetch market stats: {}", e))?;

        if stats.status == "HALT" {
            return Err(format!(
                "Market is halted: {}. All operations are blocked.",
                stats.status_reason.as_deref().unwrap_or("unknown")
            ));
        }
        Ok(())
    }

    /// Pre-validate an open order against the relayer's risk engine via `get_market_stats`.
    ///
    /// This replicates the server-side validation pipeline locally so the user gets
    /// an immediate, descriptive error instead of a generic rejection from the relayer.
    /// Checks performed (matching `risk_engine.md`):
    /// 1. Market status — reject if HALT or CLOSE_ONLY
    /// 2. Max leverage — reject if leverage exceeds `params.max_leverage`
    /// 3. Min position size — reject if entry_value < `params.min_position_btc`
    /// 4. Per-position cap — reject if entry_value > `params.max_position_pct * pool_equity`
    /// 5. Directional headroom — reject if entry_value > `max_long_btc` / `max_short_btc`
    pub async fn validate_open_order(
        &self,
        order_side: &PositionType,
        initial_margin: u64,
        leverage: u64,
    ) -> Result<(), String> {
        let stats = self
            .relayer_api_client
            .get_market_stats()
            .await
            .map_err(|e| format!("Failed to fetch market stats: {}", e))?;

        // 1. Market status
        match stats.status.as_str() {
            "HALT" => {
                return Err(format!(
                    "Market is halted: {}",
                    stats.status_reason.as_deref().unwrap_or("unknown")
                ));
            }
            "CLOSE_ONLY" => {
                return Err(format!(
                    "Market is in close-only mode: {}",
                    stats.status_reason.as_deref().unwrap_or("unknown")
                ));
            }
            _ => {}
        }

        // entry_value = initial_margin * leverage (in BTC / sats)
        let entry_value = initial_margin as f64 * leverage as f64;

        // 2. Max leverage
        if stats.params.max_leverage > 0.0 && leverage as f64 > stats.params.max_leverage {
            return Err(format!(
                "Leverage {} exceeds maximum allowed {}",
                leverage, stats.params.max_leverage
            ));
        }

        // 3. Min position size
        if stats.params.min_position_btc > 0.0 && entry_value < stats.params.min_position_btc {
            return Err(format!(
                "Position size {:.0} sats is below minimum {:.0} sats",
                entry_value, stats.params.min_position_btc
            ));
        }

        // 4. Per-position cap
        let pos_cap = stats.params.max_position_pct * stats.pool_equity_btc;
        if pos_cap > 0.0 && entry_value > pos_cap {
            return Err(format!(
                "Position size {:.0} sats exceeds per-position cap {:.0} sats ({:.1}% of pool equity)",
                entry_value, pos_cap, stats.params.max_position_pct * 100.0
            ));
        }

        // 5. Directional headroom (max_long / max_short already incorporates OI + net limits)
        let (headroom, direction) = match order_side {
            PositionType::LONG => (stats.max_long_btc, "long"),
            PositionType::SHORT => (stats.max_short_btc, "short"),
        };
        if entry_value > headroom {
            return Err(format!(
                "Position size {:.0} sats exceeds max available {} capacity {:.0} sats (utilization: {:.1}%)",
                entry_value, direction, headroom, stats.utilization * 100.0
            ));
        }

        Ok(())
    }

    pub async fn open_trader_order(
        &mut self,
        index: AccountIndex,
        order_type: OrderType,
        order_side: PositionType,
        entry_price: u64,
        leverage: u64,
    ) -> Result<String, String> {
        self.open_trader_order_opts(index, order_type, order_side, entry_price, leverage, false)
            .await
    }

    pub async fn open_trader_order_opts(
        &mut self,
        index: AccountIndex,
        order_type: OrderType,
        order_side: PositionType,
        entry_price: u64,
        leverage: u64,
        no_wait: bool,
    ) -> Result<String, String> {
        self.ensure_coin_onchain(index)?;
        if leverage == 0 {
            return Err("Leverage must be greater than 0".to_string());
        }
        if entry_price == 0 {
            return Err("Entry price must be greater than 0".to_string());
        }

        // Pre-validate against the risk engine before submitting
        let initial_margin = self.zk_accounts.get_account(&index)?.balance;
        self.validate_open_order(&order_side, initial_margin, leverage)
            .await?;
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
        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        let order_side_str = format!("{:?}", order_side);
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

        self.zk_accounts.update_io_type(&index, IOType::Memo)?;

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        {
            self.log_order_history(
                index,
                &request_id,
                "open",
                &format!("{:?}", order_type),
                Some(&order_side_str),
                initial_margin,
                Some(entry_price as f64),
                Some(leverage),
                None,
                "submitted",
                None,
            );
        }

        if no_wait || order_type == OrderType::LIMIT {
            self.pending_sync.insert(index);
        } else {
            self.sync_account_state(index).await?;
        }

        Ok(request_id)
    }

    pub async fn close_trader_order(
        &mut self,
        index: AccountIndex,
        order_type: OrderType,
        execution_price: f64,
    ) -> Result<String, String> {
        self.close_trader_order_opts(index, order_type, execution_price, false).await
    }

    pub async fn close_trader_order_opts(
        &mut self,
        index: AccountIndex,
        order_type: OrderType,
        execution_price: f64,
        no_wait: bool,
    ) -> Result<String, String> {
        self.validate_market_not_halted().await?;
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
        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        let order_type_str = format!("{:?}", order_type);
        let request_id = close_trader_order_internal(
            output,
            &secret_key,
            account_address.clone(),
            tx_hash.order_id,
            order_type.clone(),
            execution_price,
            &self.relayer_api_client,
        )
        .await?;
        let tx_hash = fetch_tx_hash_with_retry(&request_id, &self.relayer_api_client).await?;
        if tx_hash.order_status != OrderStatus::SETTLED {
            // order_type is LIMIT , so we need to wait for the order to be settled
            info!(
                "Limit order is not settled, status: {}",
                tx_hash.order_status.to_str()
            );
            return Ok(request_id);
        }
        if order_type == OrderType::LIMIT {
            info!("Limit order is settled on Market Price due to mark price hit limit price");
        }

        // Query PnL immediately (fast — hits relayer API, no chain wait)
        let trader_order = self.query_trader_order(index).await?;
        info!(
            "PnL: {:?}, Net PnL: {:?}",
            trader_order.unrealized_pnl,
            trader_order.available_margin - trader_order.initial_margin
        );
        self.zk_accounts
            .update_balance(&index, trader_order.available_margin as u64)?;
        debug!(
            "trader_order available_margin: {:?}",
            trader_order.available_margin as u64
        );
        self.zk_accounts.update_io_type(&index, IOType::Coin)?;

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        {
            self.log_order_history(
                index,
                &request_id,
                "close",
                &order_type_str,
                Some(&format!("{:?}", trader_order.position_type)),
                trader_order.available_margin as u64,
                Some(execution_price),
                Some(trader_order.leverage as u64),
                Some(trader_order.unrealized_pnl),
                "settled",
                None,
            );
        }

        if no_wait {
            self.pending_sync.insert(index);
        } else {
            // UTXO fetch waits for chain indexing (~5-6s)
            self.sync_account_state(index).await?;
        }

        Ok(request_id)
    }

    pub async fn close_trader_order_sltp(
        &mut self,
        index: AccountIndex,
        order_type: OrderType,
        execution_price: f64,
        stop_loss_price: Option<f64>,
        take_profit_price: Option<f64>,
    ) -> Result<String, String> {
        self.validate_market_not_halted().await?;
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
        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        let order_type_str = format!("{:?}", order_type);
        let request_id = close_trader_order_sltp_internal(
            output,
            &secret_key,
            account_address.clone(),
            tx_hash.order_id,
            order_type,
            execution_price,
            stop_loss_price,
            take_profit_price,
            &self.relayer_api_client,
        )
        .await?;
        let tx_hash = fetch_tx_hash_with_retry(&request_id, &self.relayer_api_client).await?;
        tokio::time::sleep(Duration::from_millis(200)).await;
        let trader_order = self.query_trader_order(index).await?;
        if trader_order.order_status != OrderStatus::SETTLED {
            // SLTP orders are conditional — they stay pending until the market
            // price hits the stop-loss or take-profit level.  We only need to
            // confirm the order was accepted (tx hash exists), not that it has
            // already settled.
            info!(
                "SLTP close order submitted, status: {}",
                tx_hash.order_status.to_str()
            );
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            {
                // let trader_order = self.query_trader_order(index).await.ok();
                self.log_order_history(
                    index,
                    &request_id,
                    "close_sltp",
                    &order_type_str,
                    Some(&format!("{:?}", trader_order.position_type)),
                    trader_order.available_margin as u64,
                    Some(execution_price),
                    Some(trader_order.leverage as u64),
                    Some(trader_order.unrealized_pnl),
                    &tx_hash.order_status.to_str(),
                    None,
                );
            }
        } else {
            info!("Order settled on Market Price due to mark price hit SLTP price");
            info!(
                "PnL: {:?}, Net PnL: {:?}",
                trader_order.unrealized_pnl,
                trader_order.available_margin - trader_order.initial_margin
            );
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
            {
                if let Ok(account) = self.zk_accounts.get_account(&index) {
                    let _ = self.update_zk_account_in_db(&account);
                }
                self.log_order_history(
                    index,
                    &request_id,
                    "close_sltp",
                    &order_type_str,
                    Some(&format!("{:?}", trader_order.position_type)),
                    trader_order.available_margin as u64,
                    Some(execution_price),
                    Some(trader_order.leverage as u64),
                    Some(trader_order.unrealized_pnl),
                    "settled",
                    Some(&tx_hash.tx_hash),
                );
            }
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

    /// Query enhanced trader order info (v1) with settle_limit, take_profit, stop_loss, funding_applied.
    pub async fn query_trader_order_v1(
        &mut self,
        index: AccountIndex,
    ) -> Result<super::relayer_types::TraderOrderV1, String> {
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let secret_key = self.get_secret_key(index);
        let query_order = query_trader_order_zkos(
            account_address.clone(),
            &secret_key,
            account_address.clone(),
            "PENDING".to_string(),
        );
        let query_order_zkos = QueryTraderOrderZkos::decode_from_hex_string(query_order)?;
        let response = self
            .relayer_api_client
            .trader_order_info_v1(query_order_zkos)
            .await
            .map_err(|e| e.to_string())?;
        Ok(response)
    }

    /// Query enhanced lend order info (v1) with unrealised profit and APR.
    pub async fn query_lend_order_v1(
        &mut self,
        index: AccountIndex,
    ) -> Result<super::relayer_types::LendOrderV1, String> {
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
            .lend_order_info_v1(query_order_zkos)
            .await
            .map_err(|e| e.to_string())?;
        Ok(response)
    }

    /// Query historical trader orders for an account.
    pub async fn historical_trader_order(
        &mut self,
        index: AccountIndex,
    ) -> Result<Vec<TraderOrder>, String> {
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let secret_key = self.get_secret_key(index);
        let query_order = query_trader_order_zkos(
            account_address.clone(),
            &secret_key,
            account_address.clone(),
            "PENDING".to_string(),
        );
        let query_order_zkos = QueryTraderOrderZkos::decode_from_hex_string(query_order)?;
        self.relayer_api_client
            .historical_trader_order_info(query_order_zkos)
            .await
            .map_err(|e| e.to_string())
    }

    /// Query historical lend orders for an account.
    pub async fn historical_lend_order(
        &mut self,
        index: AccountIndex,
    ) -> Result<Vec<LendOrder>, String> {
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let secret_key = self.get_secret_key(index);
        let query_order = query_lend_order_zkos(
            account_address.clone(),
            &secret_key,
            account_address.clone(),
            OrderStatus::LENDED.to_str(),
        );
        let query_order_zkos = QueryLendOrderZkos::decode_from_hex_string(query_order)?;
        self.relayer_api_client
            .historical_lend_order_info(query_order_zkos)
            .await
            .map_err(|e| e.to_string())
    }

    /// Query funding payment history for a trader order on an account.
    pub async fn order_funding_history(
        &mut self,
        index: AccountIndex,
    ) -> Result<Vec<super::relayer_types::FundingHistoryEntry>, String> {
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let secret_key = self.get_secret_key(index);
        let query_order = query_trader_order_zkos(
            account_address.clone(),
            &secret_key,
            account_address.clone(),
            "PENDING".to_string(),
        );
        let query_order_zkos = QueryTraderOrderZkos::decode_from_hex_string(query_order)?;
        self.relayer_api_client
            .order_funding_history(query_order_zkos)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn cancel_trader_order(&mut self, index: AccountIndex) -> Result<String, String> {
        self.validate_market_not_halted().await?;
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
        {
            if let Ok(account) = self.zk_accounts.get_account(&index) {
                let _ = self.update_zk_account_in_db(&account);
            }
            let balance = self.zk_accounts.get_balance(&index).unwrap_or(0);
            self.log_order_history(
                index,
                &request_id,
                "cancel",
                "TRADER",
                None,
                balance,
                None,
                None,
                None,
                "cancelled",
                None,
            );
        }

        Ok(request_id)
    }

    /// Check if a previously closed (e.g. SLTP) trader order has settled and, if so,
    /// unlock the account by refreshing its UTXO, balance, and IO type back to `Coin`.
    ///
    /// Returns the current `OrderStatus` so the caller can decide what to do next.
    pub async fn unlock_settled_order(
        &mut self,
        index: AccountIndex,
    ) -> Result<OrderStatus, String> {
        let trader_order = self.query_trader_order(index).await?;

        if trader_order.order_status != OrderStatus::SETTLED
            && trader_order.order_status != OrderStatus::LIQUIDATE
        {
            return Ok(trader_order.order_status);
        }

        let account_address = self.zk_accounts.get_account_address(&index)?.to_string();
        let tx_hash = fetch_tx_hash_with_account_address_retry(
            &account_address,
            Some(trader_order.order_status.clone()),
            &self.relayer_api_client,
        )
        .await?;
        let request_id = tx_hash.request_id.unwrap_or_default();
        let utxo_detail = fetch_utxo_details_with_retry(account_address, IOType::Coin).await?;
        self.utxo_details.insert(index, utxo_detail.clone());

        // Sync to database
        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        if let Err(e) = self.sync_utxo_detail_to_db(index, &utxo_detail) {
            error!("Failed to sync UTXO detail to database: {}", e);
        }

        // let trader_order = self.query_trader_order(index).await?;
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
        {
            if let Ok(account) = self.zk_accounts.get_account(&index) {
                let _ = self.update_zk_account_in_db(&account);
            }
            self.log_order_history(
                index,
                &request_id.to_string(),
                "close",
                "MARKET",
                Some(&format!("{:?}", trader_order.position_type)),
                trader_order.available_margin as u64,
                Some(0.0),
                Some(trader_order.leverage as u64),
                Some(trader_order.unrealized_pnl),
                &format!("{}", trader_order.order_status.to_str()),
                None,
            );
        }

        Ok(trader_order.order_status)
    }

    // -------------------------
    // Lend Order Operations
    // -------------------------

    pub async fn open_lend_order(&mut self, index: AccountIndex) -> Result<String, String> {
        self.validate_market_not_halted().await?;
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
        {
            if let Ok(account) = self.zk_accounts.get_account(&index) {
                let _ = self.update_zk_account_in_db(&account);
            }
            self.log_order_history(
                index,
                &request_id,
                "open",
                "LEND",
                None,
                amount,
                None,
                None,
                None,
                "submitted",
                None,
            );
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
        self.validate_market_not_halted().await?;
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
        let account = utxo_detail.output.to_quisquis_account()?;
        self.zk_accounts.update_qq_account(&index, account)?;

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        {
            if let Ok(account) = self.zk_accounts.get_account(&index) {
                let _ = self.update_zk_account_in_db(&account);
            }
            let pnl = lend_order.new_lend_state_amount - lend_order.deposit;
            self.log_order_history(
                index,
                &request_id,
                "close",
                "LEND",
                None,
                balance,
                None,
                None,
                Some(pnl),
                "settled",
                None,
            );
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

    // -------------------------
    // Transaction History
    // -------------------------

    /// Log an order action (open/close/cancel) to the history table.
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    fn log_order_history(
        &self,
        account_index: AccountIndex,
        request_id: &str,
        action: &str,
        order_type: &str,
        position_type: Option<&str>,
        amount: u64,
        price: Option<f64>,
        leverage: Option<u64>,
        pnl: Option<f64>,
        status: &str,
        tx_hash: Option<&str>,
    ) {
        if let Some(ref db_manager) = self.db_manager {
            let entry = crate::database::models::NewDbOrderHistory {
                wallet_id: db_manager.get_wallet_id().to_string(),
                account_index: account_index as i64,
                request_id: request_id.to_string(),
                action: action.to_string(),
                order_type: order_type.to_string(),
                position_type: position_type.map(|s| s.to_string()),
                amount: amount as i64,
                price,
                leverage: leverage.map(|l| l as i64),
                pnl,
                status: status.to_string(),
                tx_hash: tx_hash.map(|s| s.to_string()),
                created_at: chrono::Utc::now().naive_utc(),
            };
            if let Err(e) = db_manager.save_order_history(entry) {
                error!("Failed to log order history: {}", e);
            }
        }
    }

    /// Log a transfer action (fund_to_trade/trade_to_fund/trade_to_trade) to the history table.
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    fn log_transfer_history(
        &self,
        direction: &str,
        from_index: Option<AccountIndex>,
        to_index: Option<AccountIndex>,
        amount: u64,
        tx_hash: Option<&str>,
    ) {
        if let Some(ref db_manager) = self.db_manager {
            let entry = crate::database::models::NewDbTransferHistory {
                wallet_id: db_manager.get_wallet_id().to_string(),
                direction: direction.to_string(),
                from_index: from_index.map(|i| i as i64),
                to_index: to_index.map(|i| i as i64),
                amount: amount as i64,
                tx_hash: tx_hash.map(|s| s.to_string()),
                created_at: chrono::Utc::now().naive_utc(),
            };
            if let Err(e) = db_manager.save_transfer_history(entry) {
                error!("Failed to log transfer history: {}", e);
            }
        }
    }

    /// Query order history with optional filters.
    /// Requires database persistence to be enabled.
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn get_order_history(
        &self,
        filter: super::transaction_history::OrderHistoryFilter,
    ) -> Result<Vec<super::transaction_history::OrderHistoryEntry>, String> {
        let db_manager = self.db_manager.as_ref().ok_or("Database not enabled")?;

        let limit = filter.limit.unwrap_or(100);
        let offset = filter.offset.unwrap_or(0);

        let rows = if let Some(account_index) = filter.account_index {
            db_manager.load_order_history_by_account(account_index, limit, offset)?
        } else {
            db_manager.load_order_history(limit, offset)?
        };

        Ok(rows
            .iter()
            .map(super::transaction_history::OrderHistoryEntry::from_db)
            .collect())
    }

    /// Query transfer history with optional filters.
    /// Requires database persistence to be enabled.
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn get_transfer_history(
        &self,
        filter: super::transaction_history::TransferHistoryFilter,
    ) -> Result<Vec<super::transaction_history::TransferHistoryEntry>, String> {
        let db_manager = self.db_manager.as_ref().ok_or("Database not enabled")?;

        let limit = filter.limit.unwrap_or(100);
        let offset = filter.offset.unwrap_or(0);

        let rows = db_manager.load_transfer_history(limit, offset)?;

        Ok(rows
            .iter()
            .map(super::transaction_history::TransferHistoryEntry::from_db)
            .collect())
    }

    // -------------------------
    // Portfolio / Position Tracking
    // -------------------------

    /// Get a snapshot of all ZkOS account balances and their states.
    pub fn get_account_balances(&self) -> Vec<super::portfolio::AccountBalanceInfo> {
        self.zk_accounts
            .get_all_accounts()
            .iter()
            .map(|a| super::portfolio::AccountBalanceInfo {
                account_index: a.index,
                balance: a.balance,
                io_type: a.io_type.clone(),
                on_chain: a.on_chain,
            })
            .collect()
    }

    /// Query a single trader position and return a structured summary with PnL.
    /// The position must be in Memo state (i.e. an open order exists).
    pub async fn get_position_pnl(
        &mut self,
        index: AccountIndex,
    ) -> Result<super::portfolio::PositionSummary, String> {
        let account = self.zk_accounts.get_account(&index)?;
        if account.io_type != IOType::Memo {
            return Err(format!(
                "Account {} is not in Memo state (no open position)",
                index
            ));
        }
        let order_v1 = self.query_trader_order_v1(index).await?;
        let current_price = self
            .relayer_api_client
            .btc_usd_price()
            .await
            .map(|p| p.price)
            .unwrap_or(order_v1.order.entryprice);
        Ok(super::portfolio::PositionSummary::from_trader_order_v1(
            index,
            &order_v1,
            current_price,
        ))
    }

    /// Query a single lend position and return a structured summary.
    /// The position must be in Memo state (i.e. an active lend order exists).
    pub async fn get_lend_position_pnl(
        &mut self,
        index: AccountIndex,
    ) -> Result<super::portfolio::LendPositionSummary, String> {
        let account = self.zk_accounts.get_account(&index)?;
        if account.io_type != IOType::Memo {
            return Err(format!(
                "Account {} is not in Memo state (no active lend position)",
                index
            ));
        }
        let order_v1 = self.query_lend_order_v1(index).await?;
        Ok(super::portfolio::LendPositionSummary::from_lend_order_v1(
            index, &order_v1,
        ))
    }

    /// Build a full portfolio summary across all accounts.
    ///
    /// This queries the relayer for each open trader/lend position to get live PnL data.
    /// Accounts in Coin state contribute to `total_trading_balance`.
    /// Accounts in Memo state are queried as trader positions first; if that fails,
    /// they are tried as lend positions.
    pub async fn get_portfolio_summary(&mut self) -> Result<super::portfolio::Portfolio, String> {
        let current_price = self
            .relayer_api_client
            .btc_usd_price()
            .await
            .map(|p| p.price)
            .unwrap_or(0.0);

        let mut total_trading_balance: u64 = 0;
        let mut trader_positions = Vec::new();
        let mut closed_trader_positions = Vec::new();
        let mut liquidated_trader_positions = Vec::new();
        let mut lend_positions = Vec::new();
        let mut on_chain_count = 0;

        let accounts: Vec<_> = self
            .zk_accounts
            .get_all_accounts()
            .into_iter()
            .cloned()
            .collect();
        let total_accounts = accounts.len();

        for account in &accounts {
            if account.on_chain {
                on_chain_count += 1;
            }

            match account.io_type {
                IOType::Coin => {
                    if account.on_chain && account.balance > 0 {
                        total_trading_balance += account.balance;
                    }
                }
                IOType::Memo => {
                    // Try trader order first, fall back to lend order
                    match self.query_trader_order_v1(account.index).await {
                        Ok(order_v1) => {
                            if order_v1.order.order_status == OrderStatus::SETTLED {
                                // Build summary using the relayer's PnL (realised)
                                let mut summary =
                                    super::portfolio::PositionSummary::from_trader_order_v1(
                                        account.index,
                                        &order_v1,
                                        current_price,
                                    );
                                // Use the relayer-reported unrealized_pnl as realised PnL
                                summary.unrealized_pnl = order_v1.order.unrealized_pnl;

                                // Unlock the settled account (refresh UTXO, restore to Coin)
                                match self.unlock_settled_order(account.index).await {
                                    Ok(_) => {
                                        info!(
                                            "Unlocked settled account {} during portfolio scan",
                                            account.index
                                        );
                                    }
                                    Err(e) => {
                                        error!(
                                            "Failed to unlock settled account {}: {}",
                                            account.index, e
                                        );
                                    }
                                }

                                closed_trader_positions.push(summary);
                            } else if order_v1.order.order_status == OrderStatus::LIQUIDATE {
                                let summary =
                                    super::portfolio::PositionSummary::from_trader_order_v1(
                                        account.index,
                                        &order_v1,
                                        current_price,
                                    );

                                // Unlock the liquidated account (refresh UTXO, restore to Coin)
                                match self.unlock_settled_order(account.index).await {
                                    Ok(_) => {
                                        info!(
                                            "Unlocked liquidated account {} during portfolio scan",
                                            account.index
                                        );
                                    }
                                    Err(e) => {
                                        error!(
                                            "Failed to unlock liquidated account {}: {}",
                                            account.index, e
                                        );
                                    }
                                }

                                liquidated_trader_positions.push(summary);
                            } else {
                                trader_positions.push(
                                    super::portfolio::PositionSummary::from_trader_order_v1(
                                        account.index,
                                        &order_v1,
                                        current_price,
                                    ),
                                );
                            }
                        }
                        Err(_) => {
                            // Not a trader order — try lend
                            if let Ok(order_v1) = self.query_lend_order_v1(account.index).await {
                                lend_positions.push(
                                    super::portfolio::LendPositionSummary::from_lend_order_v1(
                                        account.index,
                                        &order_v1,
                                    ),
                                );
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let wallet_balance_sats = self
            .wallet
            .update_balance()
            .await
            .map(|b| b.sats)
            .unwrap_or(0);

        Ok(super::portfolio::Portfolio::build(
            wallet_balance_sats,
            total_trading_balance,
            trader_positions,
            closed_trader_positions,
            liquidated_trader_positions,
            lend_positions,
            total_accounts,
            on_chain_count,
        ))
    }

    /// Get liquidation risk info for all open trader positions, sorted by distance
    /// to liquidation (most at-risk first).
    pub async fn get_liquidation_risks(
        &mut self,
    ) -> Result<Vec<super::portfolio::LiquidationRisk>, String> {
        let current_price = self
            .relayer_api_client
            .btc_usd_price()
            .await
            .map(|p| p.price)
            .unwrap_or(0.0);

        if current_price == 0.0 {
            return Err("Could not fetch current BTC/USD price".to_string());
        }

        let accounts: Vec<_> = self
            .zk_accounts
            .get_all_accounts()
            .into_iter()
            .cloned()
            .collect();
        let mut risks = Vec::new();

        for account in &accounts {
            if account.io_type != IOType::Memo || !account.on_chain {
                continue;
            }

            if let Ok(order) = self.query_trader_order(account.index).await {
                if order.liquidation_price > 0.0 {
                    let distance_pct = match order.position_type {
                        PositionType::LONG => {
                            (current_price - order.liquidation_price) / current_price * 100.0
                        }
                        PositionType::SHORT => {
                            (order.liquidation_price - current_price) / current_price * 100.0
                        }
                    };

                    let margin_ratio = if order.initial_margin > 0.0 {
                        order.available_margin / order.initial_margin
                    } else {
                        0.0
                    };

                    risks.push(super::portfolio::LiquidationRisk {
                        account_index: account.index,
                        position_type: order.position_type,
                        liquidation_price: order.liquidation_price,
                        current_price,
                        distance_pct,
                        margin_ratio,
                    });
                }
            }
        }

        // Sort by distance — most at-risk (lowest distance) first
        risks.sort_by(|a, b| {
            a.distance_pct
                .partial_cmp(&b.distance_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(risks)
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
    use tokio::time::{sleep, Duration};
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
        // let mut wallet = Wallet::new(None).map_err(|e| e.to_string())?;
        // info!("importing wallet from json");
        let mut wallet = Wallet::import_from_json("test.json").map_err(|e| e.to_string())?;

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
    // #[tokio::test]
    // #[serial]
    // async fn test_create_order_complete_cycle() -> Result<(), String> {
    //     dotenv::dotenv().ok();
    //     unsafe {
    //         // std::env::set_var("DATABASE_URL", "./test.db");
    //         std::env::set_var("NYKS_WALLET_PASSPHRASE", "test1_password");
    //     }
    //     init_logger();
    //     let wallet = setup_wallet().await.map_err(|e| e.to_string())?;
    //     let zk_accounts = ZkAccountDB::new();

    //     let mut order_wallet = OrderWallet::init(wallet, zk_accounts, EndpointConfig::default())
    //         .map_err(|e| e.to_string())?;
    //     order_wallet.with_db(None, None)?;
    //     let (tx_result, account_index) = order_wallet.funding_to_trading(6000).await?;
    //     if tx_result.code != 0 {
    //         return Err(format!("Failed to send tx to chain: {}", tx_result.tx_hash));
    //     }

    //     let btc_price = order_wallet
    //         .relayer_api_client
    //         .btc_usd_price()
    //         .await
    //         .map_err(|e| e.to_string())?;
    //     info!("btc_price: {:?}", btc_price);
    //     let entry_price = btc_price.price as u64;
    //     let result = order_wallet
    //         .open_trader_order(
    //             account_index,
    //             OrderType::MARKET,
    //             PositionType::LONG,
    //             entry_price,
    //             10,
    //         )
    //         .await?;
    //     info!("result: {:?}", result);
    //     let tx_hash = fetch_tx_hash_with_retry(&result, &order_wallet.relayer_api_client).await?;
    //     info!("tx_hash: {:?}", tx_hash);
    //     assert_eq!(tx_hash.order_status, OrderStatus::FILLED);
    //     let response = order_wallet.query_trader_order(account_index).await?;
    //     info!("response: {:?}", response);
    //     assert_eq!(response.order_status, OrderStatus::FILLED);
    //     assert_eq!(
    //         order_wallet
    //             .zk_accounts
    //             .get_account(&account_index)?
    //             .io_type,
    //         IOType::Memo
    //     );

    //     let result = order_wallet
    //         .close_trader_order(account_index, OrderType::MARKET, 0.0)
    //         .await?;

    //     let tx_hash = fetch_tx_hash_with_retry(&result, &order_wallet.relayer_api_client).await?;
    //     assert_eq!(tx_hash.order_status, OrderStatus::SETTLED);
    //     let response = order_wallet.query_trader_order(account_index).await?;
    //     assert_eq!(response.order_status, OrderStatus::SETTLED);
    //     let zk_account = order_wallet.zk_accounts.get_account(&account_index)?;
    //     assert_eq!(zk_account.io_type, IOType::Coin);
    //     assert_eq!(zk_account.balance, response.available_margin as u64);
    //     let receiver_account_index = order_wallet.trading_to_trading(account_index).await?;
    //     assert_ne!(account_index, receiver_account_index);
    //     assert_eq!(
    //         order_wallet
    //             .zk_accounts
    //             .get_account(&account_index)?
    //             .on_chain,
    //         false
    //     );
    //     info!("receiver_account_index: {:?}", receiver_account_index);
    //     let result1 = order_wallet
    //         .open_trader_order(
    //             receiver_account_index,
    //             OrderType::MARKET,
    //             PositionType::LONG,
    //             entry_price,
    //             10,
    //         )
    //         .await?;
    //     info!("result1: {:?}", result1);
    //     let tx_hash1 = fetch_tx_hash_with_retry(&result1, &order_wallet.relayer_api_client).await?;
    //     info!("tx_hash1: {:?}", tx_hash1);
    //     assert_eq!(tx_hash1.order_status, OrderStatus::FILLED);
    //     let response = order_wallet
    //         .query_trader_order(receiver_account_index)
    //         .await?;
    //     info!("response: {:?}", response);
    //     assert_eq!(response.order_status, OrderStatus::FILLED);
    //     assert_eq!(
    //         order_wallet
    //             .zk_accounts
    //             .get_account(&receiver_account_index)?
    //             .io_type,
    //         IOType::Memo
    //     );
    //     let result2 = order_wallet
    //         .close_trader_order(receiver_account_index, OrderType::MARKET, 0.0)
    //         .await?;
    //     let tx_hash2 = fetch_tx_hash_with_retry(&result2, &order_wallet.relayer_api_client).await?;
    //     assert_eq!(tx_hash2.order_status, OrderStatus::SETTLED);
    //     let response2 = order_wallet
    //         .query_trader_order(receiver_account_index)
    //         .await?;
    //     assert_eq!(response2.order_status, OrderStatus::SETTLED);
    //     assert_eq!(
    //         order_wallet
    //             .zk_accounts
    //             .get_account(&receiver_account_index)?
    //             .io_type,
    //         IOType::Coin
    //     );
    //     let new_recever_index = order_wallet
    //         .trading_to_trading(receiver_account_index)
    //         .await?;
    //     assert_ne!(receiver_account_index, new_recever_index);
    //     assert_eq!(
    //         order_wallet
    //             .zk_accounts
    //             .get_account(&new_recever_index)?
    //             .io_type,
    //         IOType::Coin
    //     );
    //     let result3 = order_wallet
    //         .open_trader_order(
    //             new_recever_index,
    //             OrderType::MARKET,
    //             PositionType::LONG,
    //             entry_price,
    //             10,
    //         )
    //         .await?;
    //     let tx_hash3 = fetch_tx_hash_with_retry(&result3, &order_wallet.relayer_api_client).await?;
    //     info!("tx_hash3: {:?}", tx_hash3);
    //     assert_eq!(tx_hash3.order_status, OrderStatus::FILLED);
    //     let response3 = order_wallet.query_trader_order(new_recever_index).await?;
    //     info!("response3: {:?}", response3);
    //     assert_eq!(response3.order_status, OrderStatus::FILLED);
    //     assert_eq!(
    //         order_wallet
    //             .zk_accounts
    //             .get_account(&new_recever_index)?
    //             .io_type,
    //         IOType::Memo
    //     );
    //     let result4 = order_wallet
    //         .close_trader_order(new_recever_index, OrderType::MARKET, 0.0)
    //         .await?;
    //     debug!("result4: {:?}", result4);

    //     Ok(())
    // }
    // cargo test --no-default-features --features postgresql --lib -- relayer_module::order_wallet::tests::test_create_order --exact --show-output
    // cargo test --no-default-features --features sqlite --lib -- relayer_module::order_wallet::tests::test_create_order --exact --show-output
    // cargo test --all-features --lib -- relayer_module::order_wallet::tests::test_create_order --exact --show-output
    // #[cfg(feature = "sqlite")]
    #[tokio::test]
    #[serial]
    async fn test_create_order_complete_cycle_sltp() -> Result<(), String> {
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
            .close_trader_order_sltp(
                account_index,
                OrderType::SLTP,
                0.0,
                Some(entry_price as f64 - 1000.0),
                Some(entry_price as f64 + 1000.0),
            )
            .await?;

        // let tx_hash = fetch_tx_hash_with_retry(&result, &order_wallet.relayer_api_client).await?;
        // assert_eq!(tx_hash.order_status, OrderStatus::SETTLED);
        // let response = order_wallet.query_trader_order(account_index).await?;
        // assert_eq!(response.order_status, OrderStatus::SETTLED);
        // let zk_account = order_wallet.zk_accounts.get_account(&account_index)?;
        // assert_eq!(zk_account.io_type, IOType::Coin);
        // assert_eq!(zk_account.balance, response.available_margin as u64);
        // let receiver_account_index = order_wallet.trading_to_trading(account_index).await?;
        // assert_ne!(account_index, receiver_account_index);
        // assert_eq!(
        //     order_wallet
        //         .zk_accounts
        //         .get_account(&account_index)?
        //         .on_chain,
        //     false
        // );
        // info!("receiver_account_index: {:?}", receiver_account_index);
        // let result1 = order_wallet
        //     .open_trader_order(
        //         receiver_account_index,
        //         OrderType::MARKET,
        //         PositionType::LONG,
        //         entry_price,
        //         10,
        //     )
        //     .await?;
        // info!("result1: {:?}", result1);
        // let tx_hash1 = fetch_tx_hash_with_retry(&result1, &order_wallet.relayer_api_client).await?;
        // info!("tx_hash1: {:?}", tx_hash1);
        // assert_eq!(tx_hash1.order_status, OrderStatus::FILLED);
        // let response = order_wallet
        //     .query_trader_order(receiver_account_index)
        //     .await?;
        // info!("response: {:?}", response);
        // assert_eq!(response.order_status, OrderStatus::FILLED);
        // assert_eq!(
        //     order_wallet
        //         .zk_accounts
        //         .get_account(&receiver_account_index)?
        //         .io_type,
        //     IOType::Memo
        // );
        // let result2 = order_wallet
        //     .close_trader_order_sltp(
        //         receiver_account_index,
        //         OrderType::MARKET,
        //         0.0,
        //         Some(entry_price as f64 - 1000.0),
        //         Some(entry_price as f64 + 1000.0),
        //     )
        //     .await?;
        // let tx_hash2 = fetch_tx_hash_with_retry(&result2, &order_wallet.relayer_api_client).await?;
        // assert_eq!(tx_hash2.order_status, OrderStatus::SETTLED);
        // let response2 = order_wallet
        //     .query_trader_order(receiver_account_index)
        //     .await?;
        // assert_eq!(response2.order_status, OrderStatus::SETTLED);
        // assert_eq!(
        //     order_wallet
        //         .zk_accounts
        //         .get_account(&receiver_account_index)?
        //         .io_type,
        //     IOType::Coin
        // );
        // let new_recever_index = order_wallet
        //     .trading_to_trading(receiver_account_index)
        //     .await?;
        // assert_ne!(receiver_account_index, new_recever_index);
        // assert_eq!(
        //     order_wallet
        //         .zk_accounts
        //         .get_account(&new_recever_index)?
        //         .io_type,
        //     IOType::Coin
        // );
        // let result3 = order_wallet
        //     .open_trader_order(
        //         new_recever_index,
        //         OrderType::MARKET,
        //         PositionType::LONG,
        //         entry_price,
        //         10,
        //     )
        //     .await?;
        // let tx_hash3 = fetch_tx_hash_with_retry(&result3, &order_wallet.relayer_api_client).await?;
        // info!("tx_hash3: {:?}", tx_hash3);
        // assert_eq!(tx_hash3.order_status, OrderStatus::FILLED);
        // let response3 = order_wallet.query_trader_order(new_recever_index).await?;
        // info!("response3: {:?}", response3);
        // assert_eq!(response3.order_status, OrderStatus::FILLED);
        // assert_eq!(
        //     order_wallet
        //         .zk_accounts
        //         .get_account(&new_recever_index)?
        //         .io_type,
        //     IOType::Memo
        // );
        // let result4 = order_wallet
        //     .close_trader_order(new_recever_index, OrderType::MARKET, 0.0)
        //     .await?;
        // debug!("result4: {:?}", result4);

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

    #[tokio::test]
    #[serial]
    async fn test_trading_to_funding() -> Result<(), String> {
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
        sleep(Duration::from_secs(10)).await;
        order_wallet
            .wallet
            .update_balance()
            .await
            .map_err(|e| e.to_string())?;
        println!("wallet balance: {:?}", order_wallet.wallet.balance_sats);
        let result = order_wallet.trading_to_funding(account_index).await?;
        println!("result: {:?}", result);
        sleep(Duration::from_secs(10)).await;
        order_wallet
            .wallet
            .update_balance()
            .await
            .map_err(|e| e.to_string())?;
        println!("wallet balance: {:?}", order_wallet.wallet.balance_sats);
        Ok(())
    }
}
