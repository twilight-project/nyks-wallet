use anyhow::Result;
use chrono::{DateTime, Utc};
use log::{debug, error, info, warn};
use nyks_wallet::relayer_module::order_wallet::{AccountIndex, OrderWallet};
use nyks_wallet::relayer_module::relayer_types::{IOType, OrderStatus, OrderType, PositionType};
use nyks_wallet::zkos_accounts::zkaccount;
use rand::Rng;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

/// Configuration for the random order bot
#[derive(Debug, Clone)]
struct RandomOrderBotConfig {
    /// Initial capital in sats
    initial_capital: u64,
    /// Number of trader orders to place initially
    initial_trader_orders: usize,
    /// Number of lend orders to place initially
    initial_lend_orders: usize,
    /// Interval between order operations in seconds
    order_interval_seconds: u64,
    /// Minimum leverage for trader orders
    min_leverage: u64,
    /// Maximum leverage for trader orders
    max_leverage: u64,
    /// Price variation percentage for limit orders
    price_variation_pct: f64,
}

impl Default for RandomOrderBotConfig {
    fn default() -> Self {
        Self {
            initial_capital: 100_000, // 100k sats
            initial_trader_orders: 15,
            initial_lend_orders: 15,
            order_interval_seconds: 10,
            min_leverage: 1,
            max_leverage: 10,
            price_variation_pct: 0.02, // 2% price variation
        }
    }
}

/// Information about an active order
#[derive(Debug, Clone)]
struct OrderInfo {
    account_index: AccountIndex,
    order_type: OrderType,
    position_type: Option<PositionType>, // None for lend orders
    leverage: Option<u64>,               // None for lend orders
    entry_price: Option<u64>,            // None for lend orders
    created_at: DateTime<Utc>,
    request_id: String,
}

/// Random Order Bot for automated trading
struct RandomOrderBot {
    config: RandomOrderBotConfig,
    /// Available trading accounts (account_index, balance)
    available_accounts: Vec<(AccountIndex, u64)>,
    /// Active orders being monitored
    active_orders: HashMap<AccountIndex, OrderInfo>,
    /// Current market price
    current_market_price: u64,
    /// Statistics
    stats: BotStats,
}

#[derive(Debug, Default)]
struct BotStats {
    orders_placed: u64,
    orders_closed: u64,
    orders_cancelled: u64,
    accounts_rotated: u64,
    start_time: Option<DateTime<Utc>>,
}

impl RandomOrderBot {
    /// Create a new random order bot
    fn new(config: RandomOrderBotConfig) -> Self {
        Self {
            config,
            available_accounts: Vec::new(),
            active_orders: HashMap::new(),
            current_market_price: 0,
            stats: BotStats::default(),
        }
    }

    /// Initialize trading accounts using the ZkOS pattern
    async fn initialize_accounts(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        info!("Initializing random order bot accounts...");
        //checking for avaible accounts in wallet
        let mut available_accounts: Vec<(u64, u64)> = Vec::new();
        let zkaccount = order_wallet.zk_accounts.get_all_accounts();
        if zkaccount.is_empty() {
            info!("No available accounts found in wallet");
        } else {
            info!("Found {} zk accounts in wallet", zkaccount.len());
            for account in zkaccount {
                if account.balance > 0 && account.on_chain && account.io_type == IOType::Coin {
                    available_accounts.push((account.index, account.balance));
                }
            }
            info!(
                "Found {} available accounts in wallet",
                available_accounts.len()
            );
        }
        let required_accounts = self.config.initial_lend_orders + self.config.initial_trader_orders;
        let remaining_account: u64;
        let capital_per_account: u64 = 10000;
        let mut all_accounts = available_accounts.clone();
        if available_accounts.len() >= required_accounts {
            info!("Found enough available accounts in wallet");
            // return Ok(());
        } else {
            info!("Not enough available accounts in wallet");
            info!("Creating master trading account");
            remaining_account = (required_accounts - available_accounts.len()) as u64;

            // Step 1: Create a master trading account with the full capital
            let (tx_result, master_account) = order_wallet
                .funding_to_trading(remaining_account * capital_per_account)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create master trading account: {}", e))?;

            if tx_result.code != 0 {
                return Err(anyhow::anyhow!(
                    "Master account creation failed with code: {}",
                    tx_result.code
                ));
            }

            info!(
                "Created master trading account {} with {} sats (tx: {})",
                master_account, self.config.initial_capital, tx_result.tx_hash
            );

            // Step 2: Split the master account into multiple smaller accounts
            // let total_orders = self.config.initial_trader_orders + self.config.initial_lend_orders;
            // let capital_per_account = self.config.initial_capital / total_orders as u64;

            // info!(
            //     "Splitting master account into {} accounts with {} sats each",
            //     total_orders, capital_per_account
            // );

            // Split accounts in batches of 9 (max allowed per transaction)
            const MAX_ACCOUNTS_PER_BATCH: u64 = 8;

            let current_master = master_account;

            for batch_start in (0..remaining_account).step_by(MAX_ACCOUNTS_PER_BATCH as usize) {
                let batch_end = (batch_start + MAX_ACCOUNTS_PER_BATCH).min(remaining_account);
                let batch_size = batch_end - batch_start;
                let batch_splits = vec![capital_per_account; batch_size as usize];

                info!(
                    "Creating batch of {} accounts ({} to {})",
                    batch_size,
                    batch_start + 1,
                    batch_end
                );

                let batch_accounts = order_wallet
                    .trading_to_trading_multiple_accounts(current_master, batch_splits)
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to split accounts in batch {}: {}",
                            batch_start / MAX_ACCOUNTS_PER_BATCH + 1,
                            e
                        )
                    })?;

                all_accounts.extend(batch_accounts);
            }
            // For subsequent batches, use the first account from the previous batch as the source
            // if batch_end < total_orders {
            //     current_master = all_accounts[0].0; // Use first account as source for next batch
            // }
        }

        let accounts = all_accounts;

        // Store all accounts as available for trading
        self.available_accounts = accounts;

        info!(
            "Successfully created {} trading accounts, ready for random trading",
            self.available_accounts.len()
        );

        // Log account details
        for (account_index, balance) in self.available_accounts.iter() {
            info!("Account {}: {} sats", account_index, balance);
        }

        Ok(())
    }

    /// Update current market price from relayer API
    async fn update_market_price(&mut self, order_wallet: &OrderWallet) -> Result<()> {
        match order_wallet.relayer_api_client.btc_usd_price().await {
            Ok(btc_price) => {
                let new_price = btc_price.price as u64;
                if new_price != self.current_market_price {
                    info!(
                        "Market price updated: {} -> {}",
                        self.current_market_price, new_price
                    );
                    self.current_market_price = new_price;
                }
            }
            Err(e) => {
                warn!("Failed to fetch market price: {}", e);
            }
        }
        Ok(())
    }

    /// Get a random available account for placing orders
    fn get_random_account(&mut self) -> Option<(AccountIndex, u64)> {
        if self.available_accounts.is_empty() {
            return None;
        }

        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..self.available_accounts.len());
        Some(self.available_accounts.remove(index))
    }

    /// Return an account to the available pool
    fn return_account(&mut self, account_index: AccountIndex, balance: u64) {
        self.available_accounts.push((account_index, balance));
    }

    /// Place a random trader order
    async fn place_random_trader_order(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        let (account_index, balance) = match self.get_random_account() {
            Some(account) => account,
            None => {
                debug!("No available accounts for trader order");
                return Ok(());
            }
        };

        // Generate random order parameters
        let mut rng = rand::thread_rng();
        let variation;
        let order_type = if rng.gen_bool(0.1) {
            OrderType::MARKET
        } else {
            OrderType::LIMIT
        };

        let position_type = if rng.gen_bool(0.5) {
            variation = rng.gen_range(-self.config.price_variation_pct..=0.0) as f64;
            PositionType::LONG
        } else {
            variation = rng.gen_range(0.0..=self.config.price_variation_pct) as f64;
            PositionType::SHORT
        };

        let leverage = rng.gen_range(self.config.min_leverage..=self.config.max_leverage);

        let entry_price = if order_type == OrderType::MARKET {
            self.current_market_price
        } else {
            // For limit orders, add some price variation
            // let variation =
            //     rng.gen_range(-self.config.price_variation_pct..=self.config.price_variation_pct);
            let price_multiplier = 1.0 + variation;
            ((self.current_market_price as f64) * price_multiplier) as u64
        };

        info!(
            "Placing {:?} {:?} trader order: account={}, leverage={}, price={}",
            order_type, position_type, account_index, leverage, entry_price
        );

        match order_wallet
            .open_trader_order(
                account_index,
                order_type.clone(),
                position_type.clone(),
                entry_price,
                leverage,
            )
            .await
        {
            Ok(request_id) => {
                let order_info = OrderInfo {
                    account_index,
                    order_type,
                    position_type: Some(position_type),
                    leverage: Some(leverage),
                    entry_price: Some(entry_price),
                    created_at: Utc::now(),
                    request_id,
                };

                self.active_orders.insert(account_index, order_info);
                self.stats.orders_placed += 1;

                info!("Trader order placed successfully: {}", account_index);
            }
            Err(e) => {
                error!("Failed to place trader order: {}", e);
                // Return account to available pool
                self.return_account(account_index, balance);
            }
        }

        Ok(())
    }

    /// Place a random lend order
    async fn place_random_lend_order(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        let (account_index, balance) = match self.get_random_account() {
            Some(account) => account,
            None => {
                debug!("No available accounts for lend order");
                return Ok(());
            }
        };

        info!(
            "Placing lend order: account={}, balance={}",
            account_index, balance
        );

        match order_wallet.open_lend_order(account_index).await {
            Ok(request_id) => {
                let order_info = OrderInfo {
                    account_index,
                    order_type: OrderType::LEND,
                    position_type: None,
                    leverage: None,
                    entry_price: None,
                    created_at: Utc::now(),
                    request_id,
                };

                self.active_orders.insert(account_index, order_info);
                self.stats.orders_placed += 1;

                info!("Lend order placed successfully: {}", account_index);
            }
            Err(e) => {
                error!("Failed to place lend order: {}", e);
                // Return account to available pool
                self.return_account(account_index, balance);
            }
        }

        Ok(())
    }

    /// Check and update order statuses
    async fn update_orders(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        let mut orders_to_close = Vec::new();
        let mut orders_to_remove = Vec::new();

        for (account_index, order_info) in &self.active_orders.clone() {
            match self.check_order_status(order_wallet, *account_index).await {
                Ok(Some(status)) => {
                    match status {
                        OrderStatus::FILLED => {
                            info!(
                                "Order filled: account={}, type={:?}",
                                account_index, order_info.order_type
                            );
                            orders_to_close.push(*account_index);
                        }
                        OrderStatus::CANCELLED => {
                            info!("Order cancelled: account={}", account_index);
                            orders_to_remove.push(*account_index);
                            self.stats.orders_cancelled += 1;
                        }
                        OrderStatus::SETTLED => {
                            info!("Order settled: account={}", account_index);
                            orders_to_remove.push(*account_index);
                        }
                        OrderStatus::LIQUIDATE => {
                            info!("Order liquidated: account={}", account_index);
                            orders_to_remove.push(*account_index);
                        }
                        OrderStatus::PENDING => {
                            // Order still pending, continue monitoring
                            debug!("Order pending: account={}", account_index);
                        }
                        _ => {
                            warn!(
                                "Unexpected order status: {:?} for account {}",
                                status, account_index
                            );
                        }
                    }
                }
                Ok(None) => {
                    // Order not found or no status available
                    debug!("No status available for account {}", account_index);
                }
                Err(e) => {
                    warn!(
                        "Failed to check order status for account {}: {}",
                        account_index, e
                    );
                }
            }
        }

        // Close filled orders
        for account_index in orders_to_close {
            if let Err(e) = self.close_order(order_wallet, account_index).await {
                error!("Failed to close order for account {}: {}", account_index, e);
            }
        }

        // Remove completed orders from active list
        for account_index in orders_to_remove {
            self.active_orders.remove(&account_index);
        }

        Ok(())
    }

    /// Check the status of a specific order
    async fn check_order_status(
        &mut self,
        order_wallet: &mut OrderWallet,
        account_index: AccountIndex,
    ) -> Result<Option<OrderStatus>> {
        if let Some(order_info) = self.active_orders.get(&account_index) {
            match order_info.order_type {
                OrderType::LEND => match order_wallet.query_lend_order(account_index).await {
                    Ok(lend_order) => Ok(Some(lend_order.order_status)),
                    Err(e) => {
                        warn!("Failed to query lend order: {}", e);
                        Ok(None)
                    }
                },
                _ => match order_wallet.query_trader_order(account_index).await {
                    Ok(trader_order) => Ok(Some(trader_order.order_status)),
                    Err(e) => {
                        warn!("Failed to query trader order: {}", e);
                        Ok(None)
                    }
                },
            }
        } else {
            Ok(None)
        }
    }

    /// Close an order and rotate the account
    async fn close_order(
        &mut self,
        order_wallet: &mut OrderWallet,
        account_index: AccountIndex,
    ) -> Result<()> {
        let order_info = match self.active_orders.get(&account_index) {
            Some(info) => info.clone(),
            None => return Ok(()),
        };

        info!(
            "Closing order: account={}, type={:?}",
            account_index, order_info.order_type
        );

        match order_info.order_type {
            OrderType::LEND => {
                // Close lend order
                match order_wallet.close_lend_order(account_index).await {
                    Ok(_) => {
                        info!("Lend order closed successfully: {}", account_index);
                    }
                    Err(e) => {
                        error!("Failed to close lend order: {}", e);
                        return Err(anyhow::anyhow!("Failed to close lend order: {}", e));
                    }
                }
            }
            _ => {
                // Close trader order
                match order_wallet
                    .close_trader_order(account_index, OrderType::MARKET, 0.0)
                    .await
                {
                    Ok(_) => {
                        info!("Trader order closed successfully: {}", account_index);
                    }
                    Err(e) => {
                        error!("Failed to close trader order: {}", e);
                        return Err(anyhow::anyhow!("Failed to close trader order: {}", e));
                    }
                }
            }
        }

        // Rotate the account to get a fresh one
        match order_wallet.trading_to_trading(account_index).await {
            Ok(new_account_index) => {
                // Get the balance of the new account
                if let Ok(account) = order_wallet.zk_accounts.get_account(&new_account_index) {
                    let balance = account.balance;
                    info!(
                        "Account rotated: {} -> {} (balance: {} sats)",
                        account_index, new_account_index, balance
                    );

                    // Add the new account back to available pool
                    self.return_account(new_account_index, balance);
                    self.stats.accounts_rotated += 1;
                    self.stats.orders_closed += 1;
                } else {
                    error!(
                        "Failed to get balance for rotated account {}",
                        new_account_index
                    );
                }
            }
            Err(e) => {
                error!("Failed to rotate account {}: {}", account_index, e);
            }
        }

        Ok(())
    }

    /// Place initial orders
    async fn place_initial_orders(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        info!("Placing initial orders...");

        // Place trader orders
        for i in 0..self.config.initial_trader_orders {
            if let Err(e) = self.place_random_trader_order(order_wallet).await {
                error!("Failed to place trader order {}: {}", i + 1, e);
            }
            // Small delay between orders
            sleep(Duration::from_millis(100)).await;
        }

        // // Place lend orders
        // for i in 0..self.config.initial_lend_orders {
        //     if let Err(e) = self.place_random_lend_order(order_wallet).await {
        //         error!("Failed to place lend order {}: {}", i + 1, e);
        //     }
        //     // Small delay between orders
        //     sleep(Duration::from_millis(6500)).await;
        // }

        info!(
            "Initial orders placed: {} trader, {} lend orders active",
            self.active_orders
                .values()
                .filter(|o| o.order_type != OrderType::LEND)
                .count(),
            self.active_orders
                .values()
                .filter(|o| o.order_type == OrderType::LEND)
                .count()
        );

        Ok(())
    }

    /// Run the main trading loop
    async fn run(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        info!("Starting random order bot...");
        self.stats.start_time = Some(Utc::now());

        // Initialize accounts
        self.initialize_accounts(order_wallet).await?;

        // Update market price
        self.update_market_price(order_wallet).await?;

        // // Place initial orders
        self.place_initial_orders(order_wallet).await?;

        // info!(
        //     "Starting main trading loop with {} second intervals",
        //     self.config.order_interval_seconds
        // );

        // // Main trading loop
        // loop {
        //     // Update market price
        //     if let Err(e) = self.update_market_price(order_wallet).await {
        //         warn!("Failed to update market price: {}", e);
        //     }

        //     // Update existing orders
        //     if let Err(e) = self.update_orders(order_wallet).await {
        //         error!("Failed to update orders: {}", e);
        //     }

        //     // Place new orders if we have available accounts
        //     if !self.available_accounts.is_empty() {
        //         let mut rng = rand::thread_rng();
        //         let should_place_trader = rng.gen_bool(0.6); // 60% chance for trader order
        //         let should_place_lend = rng.gen_bool(0.4); // 40% chance for lend order

        //         if should_place_trader {
        //             if let Err(e) = self.place_random_trader_order(order_wallet).await {
        //                 error!("Failed to place trader order: {}", e);
        //             }
        //         }

        //         if should_place_lend {
        //             if let Err(e) = self.place_random_lend_order(order_wallet).await {
        //                 error!("Failed to place lend order: {}", e);
        //             }
        //         }
        //     }

        //     // Log statistics
        //     self.log_stats();

        //     // Wait for next iteration
        //     sleep(Duration::from_secs(self.config.order_interval_seconds)).await;
        // }
        Ok(())
    }

    /// Log current statistics
    fn log_stats(&self) {
        let uptime = self
            .stats
            .start_time
            .map(|start| Utc::now().signed_duration_since(start).num_seconds())
            .unwrap_or(0);

        info!(
            "Bot Stats - Uptime: {}s, Active Orders: {}, Available Accounts: {}, Orders Placed: {}, Orders Closed: {}, Accounts Rotated: {}",
            uptime,
            self.active_orders.len(),
            self.available_accounts.len(),
            self.stats.orders_placed,
            self.stats.orders_closed,
            self.stats.accounts_rotated
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();
    dotenv::dotenv().ok();

    // Create bot configuration
    let config = RandomOrderBotConfig {
        initial_capital: 200_000, // 200k sats
        initial_trader_orders: 10,
        initial_lend_orders: 10,
        order_interval_seconds: 10,
        min_leverage: 10,
        max_leverage: 50,
        price_variation_pct: 0.01, // 1% price variation
    };

    // Initialize wallet
    let wallet_id = "random_order_bot1".to_string();
    let mut order_wallet;
    let wallet_exists = OrderWallet::get_wallet_id_from_db(&wallet_id, None)
        .map_err(|e| anyhow::anyhow!("Failed to check wallet ID existence: {}", e))?;

    if wallet_exists {
        order_wallet = OrderWallet::load_from_db(wallet_id, None, None)
            .map_err(|e| anyhow::anyhow!("Failed to load wallet from database: {}", e))?;
    } else {
        order_wallet = OrderWallet::new(None)
            .map_err(|e| anyhow::anyhow!("Failed to create OrderWallet: {}", e))?;
        order_wallet
            .with_db(None, Some(wallet_id))
            .map_err(|e| anyhow::anyhow!("Failed to load wallet from with_db: {}", e))?;
    }

    // Get test tokens for development
    let mut wallet = order_wallet.wallet.clone();
    let _ = nyks_wallet::wallet::get_test_tokens(&mut wallet).await?;

    // Create and run the bot
    let mut bot = RandomOrderBot::new(config);
    bot.run(&mut order_wallet).await?;

    Ok(())
}
