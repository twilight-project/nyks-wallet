use std::collections::HashMap;

use crate::{
    config::RelayerEndPointConfig,
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
use crate::database::{DatabaseManager, establish_connection, run_migrations};
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::security::SecurePassword;
use log::{debug, error};
use relayer_module::utils::{TxResult, build_and_sign_msg_mint_burn_trading_btc, send_tx_to_chain};
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use secrecy::SecretString;
use serde::Serialize;
use twilight_client_sdk::{
    quisquislib::RistrettoSecretKey,
    relayer::{query_lend_order_zkos, query_trader_order_zkos},
    relayer_rpcclient::method::UtxoDetailResponse,
    relayer_types::{
        LendOrder, OrderStatus, OrderType, PositionType, QueryLendOrderZkos, QueryTraderOrderZkos,
        TraderOrder,
    },
    transfer::create_private_transfer_tx_single,
    zkvm::IOType,
};
pub type AccountIndex = u64;
pub type RequestId = String;
#[derive(Debug, Clone, Serialize)]
pub struct OrderWallet {
    pub wallet: Wallet,
    pub zk_accounts: ZkAccountDB,
    pub chain_id: String,
    seed: String,
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
    pub fn new(
        wallet: Wallet,
        zk_accounts: ZkAccountDB,
        chain_id: &str,
        relayer_endpoint_config: Option<RelayerEndPointConfig>,
    ) -> Result<Self, String> {
        let relayer_endpoint_config =
            relayer_endpoint_config.unwrap_or(RelayerEndPointConfig::default());
        let seed = wallet.get_zk_account_seed(chain_id, DERIVATION_MESSAGE)?;
        Ok(Self {
            wallet,
            zk_accounts,
            chain_id: chain_id.to_string(),
            seed,
            utxo_details: HashMap::new(),
            request_ids: HashMap::new(),
            relayer_api_client: RelayerJsonRpcClient::new(
                &relayer_endpoint_config.relayer_api_endpoint,
            )
            .map_err(|e| e.to_string())?,
            relayer_endpoint_config: relayer_endpoint_config,
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            db_manager: None,
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            wallet_password: None,
        })
    }
    pub fn get_zk_account_seed(&self, index: AccountIndex) -> RistrettoSecretKey {
        let key_manager = KeyManager::from_cosmos_signature(self.seed.as_bytes());
        key_manager.derive_child_key(index)
    }

    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn enable_database_persistence(
        &mut self,
        wallet_password: Option<SecretString>,
    ) -> Result<(), String> {
        // Generate wallet ID from wallet address
        let wallet_id = self.wallet.twilightaddress.clone();

        // Initialize database connection and run migrations
        let mut conn = establish_connection()?;
        run_migrations(&mut conn)?;

        // Create database manager
        let db_manager = DatabaseManager::new(wallet_id);

        // Save encrypted wallet if password is provided
        if let Some(ref password) = wallet_password {
            db_manager.save_encrypted_wallet(&mut conn, &self.wallet, password)?;
        }

        // Save existing zk accounts
        for account in self.zk_accounts.get_all_accounts() {
            db_manager.save_zk_account(&mut conn, account)?;
        }

        self.db_manager = Some(db_manager);
        self.wallet_password = wallet_password;

        Ok(())
    }

    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn load_from_database(
        wallet_id: String,
        password: Option<SecretString>,
    ) -> Result<
        (
            Option<Wallet>,
            HashMap<u64, crate::zkos_accounts::zkaccount::ZkAccount>,
        ),
        String,
    > {
        let mut conn = establish_connection()?;
        run_migrations(&mut conn)?;

        let db_manager = DatabaseManager::new(wallet_id);

        // Load wallet if password is provided
        let wallet = if let Some(ref pwd) = password {
            db_manager.load_encrypted_wallet(&mut conn, pwd)?
        } else {
            None
        };

        // Load zk accounts
        let zk_accounts = db_manager.load_all_zk_accounts(&mut conn)?;

        Ok((wallet, zk_accounts))
    }

    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    fn sync_zk_account_to_db(
        &self,
        account: &crate::zkos_accounts::zkaccount::ZkAccount,
    ) -> Result<(), String> {
        if let Some(ref db_manager) = self.db_manager {
            let mut conn = establish_connection()?;
            db_manager.save_zk_account(&mut conn, account)?;
        }
        Ok(())
    }

    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    fn update_zk_account_in_db(
        &self,
        account: &crate::zkos_accounts::zkaccount::ZkAccount,
    ) -> Result<(), String> {
        if let Some(ref db_manager) = self.db_manager {
            let mut conn = establish_connection()?;
            db_manager.update_zk_account(&mut conn, account)?;
        }
        Ok(())
    }

    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn remove_zk_account_from_db(&self, account_index: u64) -> Result<(), String> {
        if let Some(ref db_manager) = self.db_manager {
            let mut conn = establish_connection()?;
            db_manager.remove_zk_account(&mut conn, account_index)?;
        }
        Ok(())
    }

    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    pub fn get_all_zk_accounts_from_db(
        &self,
    ) -> Result<HashMap<u64, crate::zkos_accounts::zkaccount::ZkAccount>, String> {
        if let Some(ref db_manager) = self.db_manager {
            let mut conn = establish_connection()?;
            db_manager.load_all_zk_accounts(&mut conn)
        } else {
            Err("Database manager not initialized".to_string())
        }
    }

    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    /// Enable database persistence with automatic password prompt
    pub fn enable_database_persistence_with_prompt(&mut self) -> Result<(), String> {
        let password = SecurePassword::get_passphrase()
            .map_err(|e| format!("Failed to get password: {}", e))?;
        self.enable_database_persistence(Some(password))
    }

    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    /// Create new wallet with database persistence and secure password
    pub async fn create_new_with_database(
        chain_id: &str,
        relayer_endpoint_config: Option<RelayerEndPointConfig>,
    ) -> Result<Self, String> {
        // Create new wallet
        let wallet = Wallet::create_new_with_random_btc_address()
            .await
            .map_err(|e| e.to_string())?;

        let zk_accounts = ZkAccountDB::new();

        // Create OrderWallet
        let mut order_wallet = Self::new(wallet, zk_accounts, chain_id, relayer_endpoint_config)?;

        // Get secure password and enable persistence
        let password = SecurePassword::create_new_passphrase()
            .map_err(|e| format!("Failed to create password: {}", e))?;

        order_wallet.enable_database_persistence(Some(password))?;

        Ok(order_wallet)
    }

    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    /// Load wallet from database with automatic password prompt
    pub fn load_from_database_with_prompt(
        wallet_id: String,
    ) -> Result<
        (
            Option<Wallet>,
            HashMap<u64, crate::zkos_accounts::zkaccount::ZkAccount>,
        ),
        String,
    > {
        let password = SecurePassword::get_passphrase()
            .map_err(|e| format!("Failed to get password: {}", e))?;
        Self::load_from_database(wallet_id, Some(password))
    }

    // Create a new zk_account and transfer sats from wallet to zk_account
    // Return the tx result (tx_hash, code) and the account index
    pub async fn funding_to_trading(&mut self, amount: u64) -> Result<(TxResult, u64), String> {
        let wallet_balance = self
            .wallet
            .update_balance()
            .await
            .map_err(|e| e.to_string())?;
        if wallet_balance.nyks > 0 && wallet_balance.sats >= amount {
            let account_index = self
                .zk_accounts
                .generate_new_account(amount, self.seed.clone())?;

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
                send_tx_to_chain(signed_tx.clone(), &self.wallet.chain_config.rpc_endpoint).await?;
            if result.code != 0 {
                return Err(format!("Failed to send tx to chain: {}", result.tx_hash));
            } else {
                let account_address = self.zk_accounts.get_account_address(&account_index)?;
                let utxo_detail =
                    match fetch_utxo_details_with_retry(account_address, 20, 1000, IOType::Coin)
                        .await
                    {
                        Ok(utxo_detail) => utxo_detail,
                        Err(e) => {
                            error!("Failed to fetch utxo details after {} attempts: {}", 10, e);
                            return Err(e);
                        }
                    };
                self.utxo_details.insert(account_index, utxo_detail);
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
        let sender_account = self.zk_accounts.get_account(&index)?;
        if sender_account.on_chain == false
            || sender_account.io_type != IOType::Coin
            || sender_account.balance == 0
        {
            error!("Account is not on chain or not a coin account");
            return Err(format!("Account is not on chain or not a coin account"));
        }
        let sender_account_address = sender_account.account.clone();
        let amount = sender_account.balance;
        let new_account_index = self
            .zk_accounts
            .generate_new_account(amount, self.seed.clone())?;

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        if let Ok(account) = self.zk_accounts.get_account(&new_account_index) {
            let _ = self.sync_zk_account_to_db(&account);
        }
        let receiver_input_string = self
            .zk_accounts
            .get_account(&new_account_index)?
            .get_input_string()?;

        let utxo_detail =
            match fetch_utxo_details_with_retry(sender_account_address, 20, 1000, IOType::Coin)
                .await
            {
                Ok(utxo_detail) => utxo_detail,
                Err(e) => {
                    error!("Failed to fetch utxo details after {} attempts: {}", 10, e);
                    return Err(e);
                }
            };
        let input = utxo_detail.get_input()?;
        let tx_wallet = create_private_transfer_tx_single(
            self.get_zk_account_seed(index),
            input,
            receiver_input_string,
            amount,
            true,
            0,
            1u64,
        );
        let response = tokio::task::spawn_blocking(move || {
            twilight_client_sdk::chain::tx_commit_broadcast_transaction(
                tx_wallet.get_tx().ok_or("Failed to get tx")?,
            )
        })
        .await
        .map_err(|e| format!("Failed to send RPC request: {}", e))?;
        debug!("response: {:?}", response);
        let utxo_detail = match fetch_utxo_details_with_retry(
            self.zk_accounts.get_account_address(&new_account_index)?,
            20,
            1000,
            IOType::Coin,
        )
        .await
        {
            Ok(utxo_detail) => utxo_detail,
            Err(e) => {
                error!("Failed to fetch utxo details after {} attempts: {}", 10, e);
                return Err(e);
            }
        };

        self.utxo_details.insert(new_account_index, utxo_detail);
        self.utxo_details.remove(&index);
        self.zk_accounts.update_on_chain(&new_account_index, true)?;
        self.zk_accounts.update_on_chain(&index, false)?;

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

    pub async fn open_trader_order(
        &mut self,
        index: AccountIndex,
        order_type: OrderType,
        order_side: PositionType,
        entry_price: u64,
        leverage: u64,
    ) -> Result<String, String> {
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let _utxo_detail =
            match fetch_utxo_details_with_retry(account_address.clone(), 60, 1000, IOType::Coin)
                .await
            {
                Ok(utxo_detail) => utxo_detail,
                Err(e) => {
                    error!("Failed to fetch utxo details after {} attempts: {}", 60, e);
                    return Err(e);
                }
            };
        let secret_key = self.get_zk_account_seed(index);
        let r_scalar = self.zk_accounts.get_account(&index)?.get_scalar()?;

        let initial_margin = self.zk_accounts.get_account(&index)?.balance;
        let position_value = initial_margin * leverage;
        let position_size = position_value * entry_price;
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

        self.request_ids.insert(index, request_id.clone());

        let utxo_detail;
        if order_type == OrderType::LIMIT {
        } else {
            utxo_detail = match fetch_utxo_details_with_retry(
                account_address,
                20,
                1000,
                IOType::Memo,
            )
            .await
            {
                Ok(utxo_detail) => utxo_detail,
                Err(e) => {
                    error!("Failed to fetch utxo details after {} attempts: {}", 10, e);
                    return Err(e);
                }
            };
            self.utxo_details.insert(index, utxo_detail);
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
        let secret_key = self.get_zk_account_seed(index);
        let tx_hash = fetch_tx_hash_with_retry(
            &self.request_ids.get(&index).unwrap().clone(),
            20,
            1000,
            &self.relayer_api_client,
        )
        .await?;
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
        let tx_hash =
            fetch_tx_hash_with_retry(&request_id, 20, 1000, &self.relayer_api_client).await?;
        if tx_hash.order_status != OrderStatus::SETTLED {
            return Err(format!(
                "Order is not settled, status: {}",
                tx_hash.order_status.to_str()
            ));
        }
        let utxo_detail =
            match fetch_utxo_details_with_retry(account_address, 20, 1000, IOType::Coin).await {
                Ok(utxo_detail) => utxo_detail,
                Err(e) => {
                    error!("Failed to fetch utxo details after {} attempts: {}", 10, e);
                    return Err(e);
                }
            };
        self.utxo_details.insert(index, utxo_detail);
        let trader_order = self.query_trader_order(index).await?;
        self.zk_accounts
            .update_balance(&index, trader_order.available_margin as u64)?;
        debug!(
            "trader_order available_margin: {:?}",
            trader_order.available_margin as u64
        );
        self.zk_accounts.update_io_type(&index, IOType::Coin)?;

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        if let Ok(account) = self.zk_accounts.get_account(&index) {
            let _ = self.update_zk_account_in_db(&account);
        }
        Ok(request_id)
    }

    pub async fn query_trader_order(&mut self, index: AccountIndex) -> Result<TraderOrder, String> {
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let secret_key = self.get_zk_account_seed(index);
        // let request_id = self.request_ids.get(&index).unwrap();
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

    pub async fn query_lend_order(&mut self, index: AccountIndex) -> Result<LendOrder, String> {
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let secret_key = self.get_zk_account_seed(index);
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

    pub async fn open_lend_order(&mut self, index: AccountIndex) -> Result<String, String> {
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let _utxo_detail =
            match fetch_utxo_details_with_retry(account_address.clone(), 60, 1000, IOType::Coin)
                .await
            {
                Ok(utxo_detail) => utxo_detail,
                Err(e) => {
                    error!("Failed to fetch utxo details after {} attempts: {}", 60, e);
                    return Err(e);
                }
            };
        let secret_key = self.get_zk_account_seed(index);
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
        let utxo_detail =
            match fetch_utxo_details_with_retry(account_address, 20, 1000, IOType::Memo).await {
                Ok(utxo_detail) => utxo_detail,
                Err(e) => {
                    error!("Failed to fetch utxo details after {} attempts: {}", 10, e);
                    return Err(e);
                }
            };
        self.utxo_details.insert(index, utxo_detail);
        self.zk_accounts.update_io_type(&index, IOType::Memo)?;

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        if let Ok(account) = self.zk_accounts.get_account(&index) {
            let _ = self.update_zk_account_in_db(&account);
        }

        Ok(request_id)
    }

    pub async fn close_lend_order(&mut self, index: AccountIndex) -> Result<String, String> {
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let secret_key = self.get_zk_account_seed(index);
        let tx_hash = fetch_tx_hash_with_retry(
            &self.request_ids.get(&index).unwrap().clone(),
            20,
            1000,
            &self.relayer_api_client,
        )
        .await?;
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
        let tx_hash =
            fetch_tx_hash_with_retry(&request_id, 20, 1000, &self.relayer_api_client).await?;
        if tx_hash.order_status != OrderStatus::SETTLED {
            return Err(format!(
                "Order is not settled, status: {}",
                tx_hash.order_status.to_str()
            ));
        }
        let utxo_detail =
            match fetch_utxo_details_with_retry(account_address, 20, 1000, IOType::Coin).await {
                Ok(utxo_detail) => utxo_detail,
                Err(e) => {
                    error!("Failed to fetch utxo details after {} attempts: {}", 10, e);
                    return Err(e);
                }
            };
        self.utxo_details.insert(index, utxo_detail);
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

    pub async fn cancel_trader_order(&mut self, index: AccountIndex) -> Result<String, String> {
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let secret_key = self.get_zk_account_seed(index);
        let tx_hash = fetch_tx_hash_with_retry(
            &self.request_ids.get(&index).unwrap().clone(),
            20,
            1000,
            &self.relayer_api_client,
        )
        .await?;
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
        let tx_hash =
            fetch_tx_hash_with_retry(&request_id, 20, 1000, &self.relayer_api_client).await?;
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
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
impl Drop for OrderWallet {
    fn drop(&mut self) {
        if let Some(ref db_manager) = self.db_manager {
            if let Ok(mut conn) = establish_connection() {
                // Save all current zk accounts to database
                for account in self.zk_accounts.get_all_accounts() {
                    if let Err(e) = db_manager.save_zk_account(&mut conn, account) {
                        error!(
                            "Failed to persist zk_account {} during drop: {}",
                            account.index, e
                        );
                    }
                }

                // Save encrypted wallet if password is available
                if let Some(ref password) = self.wallet_password {
                    if let Err(e) =
                        db_manager.save_encrypted_wallet(&mut conn, &self.wallet, password)
                    {
                        error!("Failed to persist wallet during drop: {}", e);
                    }
                }

                debug!("OrderWallet data persisted to database during drop");
            } else {
                error!("Failed to establish database connection during drop");
            }
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
        let mut wallet = Wallet::create_new_with_random_btc_address()
            .await
            .map_err(|e| e.to_string())?;
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
    #[tokio::test]
    #[serial]
    async fn test_create_order() -> Result<(), String> {
        dotenv::dotenv().ok();
        init_logger();
        let wallet = setup_wallet().await.unwrap();
        let zk_accounts = ZkAccountDB::new();

        let mut order_wallet = OrderWallet::new(wallet, zk_accounts, "nyks", None)?;
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
        let tx_hash =
            fetch_tx_hash_with_retry(&result, 20, 1000, &order_wallet.relayer_api_client).await?;
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

        let tx_hash =
            fetch_tx_hash_with_retry(&result, 20, 1000, &order_wallet.relayer_api_client).await?;
        assert_eq!(tx_hash.order_status, OrderStatus::SETTLED);
        let response = order_wallet.query_trader_order(account_index).await?;
        assert_eq!(response.order_status, OrderStatus::SETTLED);
        let zk_account = order_wallet.zk_accounts.get_account(&account_index)?;
        assert_eq!(zk_account.io_type, IOType::Coin);
        assert_eq!(zk_account.balance, response.available_margin as u64);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_trading_to_trading() -> Result<(), String> {
        dotenv::dotenv().ok();
        init_logger();
        let wallet = setup_wallet().await.unwrap();
        let zk_accounts = ZkAccountDB::new();
        let mut order_wallet = OrderWallet::new(wallet, zk_accounts, "nyks", None)?;
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
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_open_lend_order() -> Result<(), String> {
        dotenv::dotenv().ok();
        init_logger();
        let wallet = setup_wallet().await.unwrap();
        let zk_accounts = ZkAccountDB::new();
        let mut order_wallet = OrderWallet::new(wallet, zk_accounts, "nyks", None)?;
        let (tx_result, account_index) = order_wallet.funding_to_trading(6000).await?;
        if tx_result.code != 0 {
            return Err(format!("Failed to send tx to chain: {}", tx_result.tx_hash));
        }
        let result = order_wallet.open_lend_order(account_index).await?;
        let tx_hash =
            fetch_tx_hash_with_retry(&result, 20, 1000, &order_wallet.relayer_api_client).await?;
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
        let tx_hash =
            fetch_tx_hash_with_retry(&result, 20, 1000, &order_wallet.relayer_api_client).await?;
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
        let wallet = setup_wallet().await.unwrap();
        let zk_accounts = ZkAccountDB::new();

        let mut order_wallet = OrderWallet::new(wallet, zk_accounts, "nyks", None)?;
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
        let tx_hash =
            fetch_tx_hash_with_retry(&result, 20, 1000, &order_wallet.relayer_api_client).await?;
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

        let tx_hash =
            fetch_tx_hash_with_retry(&result, 20, 1000, &order_wallet.relayer_api_client).await?;
        assert_eq!(tx_hash.order_status, OrderStatus::CANCELLED);
        let response = order_wallet.query_trader_order(account_index).await?;
        assert_eq!(response.order_status, OrderStatus::CANCELLED);
        let zk_account = order_wallet.zk_accounts.get_account(&account_index)?;
        assert_eq!(zk_account.io_type, IOType::Coin);
        assert_eq!(zk_account.balance, response.available_margin as u64);

        Ok(())
    }
}
