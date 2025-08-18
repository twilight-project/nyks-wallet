//! # Simple Market Maker Bot
//!
//! This example demonstrates a basic market making strategy using the Nyks Wallet SDK.
//! The bot places both buy and sell orders around the current market price to capture spreads.
//!
//! ## Strategy Overview
//! - Places limit orders on both sides of the market
//! - Adjusts orders based on market movement
//! - Manages inventory to avoid excessive long/short exposure
//! - Implements basic risk management
//!
//! ## Usage
//! ```bash
//! cargo run --bin simple_market_maker -- --spread 0.002 --order-size 1000 --max-inventory 10000
//! ```

use anyhow::{Context, Result};
use clap::Parser;
use log::{error, info, warn};
use nyks_wallet::relayer_module::order_wallet::{AccountIndex, OrderWallet};
use nyks_wallet::relayer_module::relayer_types::{IOType, OrderStatus, OrderType, PositionType};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};
use tokio::time::{interval, sleep};
/// Market maker bot command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Spread percentage (0.002 = 0.2%)
    #[arg(short, long, default_value = "0.002")]
    spread: f64,

    /// Order size in satoshis
    #[arg(short, long, default_value = "1000")]
    order_size: u64,

    /// Maximum inventory position
    #[arg(short, long, default_value = "10000")]
    max_inventory: i64,

    /// Initial capital in satoshis
    #[arg(short, long, default_value = "50000")]
    initial_capital: u64,

    /// Order refresh interval in seconds
    #[arg(short, long, default_value = "60")]
    refresh_interval: u64,

    /// Enable paper trading mode
    #[arg(short, long)]
    paper_trading: bool,

    /// Maximum leverage for hedging
    #[arg(long, default_value = "3")]
    max_leverage: u64,

    /// Use enhanced market data (order book + recent trades)
    #[arg(long)]
    enhanced_market_data: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct MarketMaker {
    /// Configuration
    config: MarketMakerConfig,
    /// Available trading accounts (fresh, ready to use)
    available_accounts: Vec<(AccountIndex, u64)>, // (account_index, balance)
    /// Current orders with account info
    active_orders: HashMap<AccountIndex, OrderInfo>,
    /// Inventory tracking
    inventory: i64, // Positive = long, negative = short
    /// Statistics
    stats: MarketMakerStats,
    /// Current market price estimate
    estimated_market_price: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct MarketMakerConfig {
    spread: f64,
    order_size: u64,
    max_inventory: i64,
    initial_capital: u64,
    refresh_interval: Duration,
    paper_trading: bool,
    max_leverage: u64,
    enhanced_market_data: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OrderInfo {
    order_type: PositionType,
    price: u64,
    size: u64,
    request_id: String,
    placed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct MarketMakerStats {
    orders_placed: u32,
    orders_filled: u32,
    total_volume: u64,
    estimated_pnl: i64,
    max_inventory_reached: i64,
    uptime_seconds: u64,
}

impl MarketMaker {
    /// Create a new market maker with the given configuration
    fn new(args: Args) -> Self {
        Self {
            config: MarketMakerConfig {
                spread: args.spread,
                order_size: args.order_size,
                max_inventory: args.max_inventory,
                initial_capital: args.initial_capital,
                refresh_interval: Duration::from_secs(args.refresh_interval),
                paper_trading: args.paper_trading,
                max_leverage: args.max_leverage,
                enhanced_market_data: args.enhanced_market_data,
            },
            available_accounts: Vec::new(),
            active_orders: HashMap::new(),
            inventory: 0,
            stats: MarketMakerStats::default(),
            estimated_market_price: 50000, // Default starting price
        }
    }

    /// Initialize trading accounts for market making using the proper ZkOS pattern
    async fn initialize_accounts(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        info!("Initializing market maker accounts using ZkOS pattern...");

        // Step 1: Create a master trading account with the full capital
        let (tx_result, master_account) = order_wallet
            .funding_to_trading(self.config.initial_capital)
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

        // Step 2: Split the master account into multiple smaller accounts for market making
        // Each account gets a portion of capital for individual orders
        let account_count = 6; // Create 6 accounts for buy/sell rotation
        let capital_per_account = self.config.initial_capital / account_count;
        let splits = vec![capital_per_account; account_count as usize];

        info!(
            "Splitting master account into {} accounts with {} sats each",
            account_count, capital_per_account
        );

        let accounts = order_wallet
            .trading_to_trading_multiple_accounts(master_account, splits)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to split accounts: {}", e))?;

        // Store all accounts as available for trading
        self.available_accounts = accounts;

        info!(
            "Successfully created {} trading accounts, ready for market making",
            self.available_accounts.len()
        );

        // Log account details
        for (i, (account_index, balance)) in self.available_accounts.iter().enumerate() {
            info!(
                "Account {}: index={}, balance={} sats",
                i + 1,
                account_index,
                balance
            );
        }

        Ok(())
    }

    /// Run the market making strategy
    async fn run(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        info!(
            "Starting market maker with spread: {:.3}%",
            self.config.spread * 100.0
        );

        let mut refresh_timer = interval(self.config.refresh_interval);
        let start_time = chrono::Utc::now();

        loop {
            refresh_timer.tick().await;

            // Update statistics
            self.stats.uptime_seconds = chrono::Utc::now()
                .signed_duration_since(start_time)
                .num_seconds() as u64;

            // Check and update active orders
            if let Err(e) = self.update_orders(order_wallet).await {
                error!("Error updating orders: {}", e);
                continue;
            }

            // Update market price estimate
            if self.config.enhanced_market_data {
                self.update_enhanced_market_data(order_wallet).await?;
            } else {
                self.update_market_price(order_wallet).await?;
            }

            // Manage inventory if needed
            if let Err(e) = self.manage_inventory(order_wallet).await {
                error!("Error managing inventory: {}", e);
            }

            // Place new orders if needed
            if let Err(e) = self.place_market_making_orders(order_wallet).await {
                error!("Error placing orders: {}", e);
                // Wait before retrying
                sleep(Duration::from_secs(5)).await;
            }

            // Log status periodically
            if self.stats.uptime_seconds % 300 == 0 {
                // Every 5 minutes
                self.log_status();
            }
        }
    }

    /// Update the status of active orders and handle account rotation
    async fn update_orders(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        let mut completed_orders = Vec::new();

        for (account_index, order_info) in &self.active_orders {
            match order_wallet.query_trader_order(*account_index).await {
                Ok(trader_order) => {
                    match trader_order.order_status {
                        OrderStatus::FILLED => {
                            info!(
                                "Order filled on account {}: {:?} (closing position)",
                                account_index, order_info.order_type
                            );

                            // Close the position immediately to settle the account
                            match order_wallet
                                .close_trader_order(*account_index, OrderType::MARKET, 0.0)
                                .await
                            {
                                Ok(close_request_id) => {
                                    info!("Position closed with request ID: {}", close_request_id);

                                    // Update inventory
                                    match order_info.order_type {
                                        PositionType::LONG => {
                                            self.inventory += order_info.size as i64;
                                        }
                                        PositionType::SHORT => {
                                            self.inventory -= order_info.size as i64;
                                        }
                                    }

                                    self.stats.orders_filled += 1;
                                    self.stats.total_volume += order_info.size;
                                    // completed_orders.push(*account_index);
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to close position on account {}: {}",
                                        account_index, e
                                    );
                                }
                            }
                        }
                        OrderStatus::SETTLED => {
                            info!(
                                "Order settled on account {}, rotating account",
                                account_index
                            );

                            // Rotate the account to get a fresh one
                            match order_wallet.trading_to_trading(*account_index).await {
                                Ok(new_account_index) => {
                                    // Query the new account balance
                                    if let Ok(new_balance) = self
                                        .get_account_balance(order_wallet, new_account_index)
                                        .await
                                    {
                                        info!(
                                            "Account rotated: {} -> {} (balance: {} sats)",
                                            account_index, new_account_index, new_balance
                                        );

                                        // Add the new account back to available pool
                                        self.available_accounts
                                            .push((new_account_index, new_balance));
                                        completed_orders.push(*account_index);
                                    } else {
                                        error!(
                                            "Failed to query balance for rotated account {}",
                                            new_account_index
                                        );
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to rotate account {}: {}", account_index, e);
                                }
                            }
                        }
                        OrderStatus::CANCELLED => {
                            info!(
                                "Order cancelled on account {}, can reuse account",
                                account_index
                            );

                            // For cancelled orders, we can reuse the same account (no rotation needed)
                            if let Ok(balance) =
                                self.get_account_balance(order_wallet, *account_index).await
                            {
                                self.available_accounts.push((*account_index, balance));
                            }
                            completed_orders.push(*account_index);
                        }
                        OrderStatus::PENDING => {
                            // Check if order is too old
                            let order_age = chrono::Utc::now()
                                .signed_duration_since(order_info.placed_at)
                                .num_seconds();

                            if order_age > 300 {
                                // 5 minutes
                                info!("Cancelling old order on account {}", account_index);
                                if let Err(e) =
                                    order_wallet.cancel_trader_order(*account_index).await
                                {
                                    error!("Failed to cancel order: {}", e);
                                }
                            }
                        }
                        _ => {
                            // Other statuses like LIQUIDATED, etc.
                            warn!(
                                "Order on account {} has status: {:?}",
                                account_index, trader_order.order_status
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to query order for account {}: {}", account_index, e);
                }
            }
        }

        // Remove completed orders from active tracking
        for account_index in completed_orders {
            self.active_orders.remove(&account_index);
        }

        Ok(())
    }

    /// Helper to get account balance
    async fn get_account_balance(
        &self,
        order_wallet: &OrderWallet,
        account_index: AccountIndex,
    ) -> Result<u64> {
        // Try to get balance from ZkAccountDB
        if let Some(balance) = order_wallet.zk_accounts.get_balance(&account_index) {
            Ok(balance)
        } else {
            Err(anyhow::anyhow!(
                "Account {} not found in ZkAccountDB",
                account_index
            ))
        }
    }
    /// Get an available account that's ready for trading (Coin state, non-zero balance)
    fn get_available_account(&mut self, order_wallet: &OrderWallet) -> Option<(AccountIndex, u64)> {
        // Find the first account that's in the correct state
        for i in 0..self.available_accounts.len() {
            let (account_index, _balance) = self.available_accounts[i];

            // Check account state
            if let Ok(account) = order_wallet.zk_accounts.get_account(&account_index) {
                if account.io_type == IOType::Coin && account.balance > 0 {
                    // Remove and return this account
                    return Some(self.available_accounts.remove(i));
                } else {
                    info!(
                        "Skipping account {}: io_type={:?}, balance={}",
                        account_index, account.io_type, account.balance
                    );
                }
            } else {
                warn!("Cannot access account {}", account_index);
            }
        }

        // No valid accounts found
        info!(
            "No accounts in valid state (Coin, balance > 0) found in pool of {}",
            self.available_accounts.len()
        );
        None
    }
    /// Update market price estimate using real data from relayer API
    async fn update_market_price(&mut self, order_wallet: &OrderWallet) -> Result<()> {
        // Fetch current BTC/USD price from relayer
        match order_wallet.relayer_api_client.btc_usd_price().await {
            Ok(btc_price) => {
                let new_price = btc_price.price as u64;

                if new_price != self.estimated_market_price {
                    let price_change_pct = if self.estimated_market_price > 0 {
                        ((new_price as f64 - self.estimated_market_price as f64)
                            / self.estimated_market_price as f64)
                            * 100.0
                    } else {
                        0.0
                    };

                    info!(
                        "Market price updated from relayer: {} -> {} (change: {:.3}%)",
                        self.estimated_market_price, new_price, price_change_pct
                    );

                    self.estimated_market_price = new_price;
                } else {
                    // Price unchanged, just log timestamp
                    info!(
                        "Market price unchanged at {} (timestamp: {})",
                        new_price, btc_price.timestamp
                    );
                }
            }
            Err(e) => {
                // If API call fails, fall back to simulated price movement
                warn!(
                    "Failed to fetch price from relayer API: {}, using simulation",
                    e
                );

                let price_change = (rand::random::<f64>() - 0.5) * 0.001; // ±0.05% random walk
                let new_price = (self.estimated_market_price as f64 * (1.0 + price_change)) as u64;

                if new_price != self.estimated_market_price {
                    info!(
                        "Market price simulated: {} -> {} (fallback mode)",
                        self.estimated_market_price, new_price
                    );
                    self.estimated_market_price = new_price;
                }
            }
        }

        Ok(())
    }

    /// Enhanced market data update with additional sources (optional)
    async fn update_enhanced_market_data(&mut self, order_wallet: &OrderWallet) -> Result<()> {
        // Fetch multiple data sources for more informed pricing
        let price_future = order_wallet.relayer_api_client.btc_usd_price();
        let order_book_future = order_wallet.relayer_api_client.open_limit_orders();
        let recent_trades_future = order_wallet.relayer_api_client.recent_trade_orders();

        // Execute all API calls concurrently
        let (price_result, order_book_result, recent_trades_result) =
            tokio::join!(price_future, order_book_future, recent_trades_future);

        // Process price data
        let base_price = match price_result {
            Ok(btc_price) => {
                info!(
                    "Latest BTC price: {} (timestamp: {})",
                    btc_price.price, btc_price.timestamp
                );
                btc_price.price as u64
            }
            Err(e) => {
                warn!("Failed to fetch BTC price: {}", e);
                self.estimated_market_price // Use current estimate
            }
        };

        // Analyze order book for better price estimation
        let adjusted_price = match order_book_result {
            Ok(order_book) => {
                // Calculate mid-market price from order book
                let best_bid = order_book
                    .bid
                    .first()
                    .map(|bid| bid.price)
                    .unwrap_or(base_price as f64);
                let best_ask = order_book
                    .ask
                    .first()
                    .map(|ask| ask.price)
                    .unwrap_or(base_price as f64);
                let mid_market = (best_bid + best_ask) / 2.0;

                info!(
                    "Order book - Bid: {:.2}, Ask: {:.2}, Mid: {:.2}",
                    best_bid, best_ask, mid_market
                );

                // Weight between base price and mid-market (70% base, 30% order book)
                (base_price as f64 * 0.7 + mid_market * 0.3) as u64
            }
            Err(e) => {
                warn!("Failed to fetch order book: {}", e);
                base_price
            }
        };

        // Log recent trading activity for context
        if let Ok(recent_trades) = recent_trades_result {
            let total_volume: f64 = recent_trades
                .orders
                .iter()
                .map(|order| order.positionsize)
                .sum();
            info!(
                "Recent trading volume: {:.0} sats from {} orders",
                total_volume,
                recent_trades.orders.len()
            );
        }

        // Update our price estimate
        if adjusted_price != self.estimated_market_price {
            let change_pct = if self.estimated_market_price > 0 {
                ((adjusted_price as f64 - self.estimated_market_price as f64)
                    / self.estimated_market_price as f64)
                    * 100.0
            } else {
                0.0
            };

            info!(
                "Enhanced market price update: {} -> {} (change: {:.3}%)",
                self.estimated_market_price, adjusted_price, change_pct
            );

            self.estimated_market_price = adjusted_price;
        }

        Ok(())
    }

    /// Manage inventory through hedging if needed
    async fn manage_inventory(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        let inventory_abs = self.inventory.abs();
        let max_inventory = self.config.max_inventory;

        if inventory_abs > max_inventory {
            warn!(
                "Inventory limit exceeded: {} (max: {})",
                self.inventory, max_inventory
            );

            // Use an available account for hedging if we have one
            if let Some((hedge_account, balance)) = self.get_available_account(order_wallet) {
                // Place hedge order to reduce inventory
                let hedge_side = if self.inventory > 0 {
                    PositionType::SHORT // Sell to reduce long inventory
                } else {
                    PositionType::LONG // Buy to reduce short inventory
                };

                if !self.config.paper_trading {
                    info!(
                        "Placing hedge order: {:?} using account {} with {} sats",
                        hedge_side, hedge_account, balance
                    );

                    match order_wallet
                        .open_trader_order(
                            hedge_account,
                            OrderType::MARKET,
                            hedge_side.clone(),
                            self.estimated_market_price,
                            self.config.max_leverage,
                        )
                        .await
                    {
                        Ok(request_id) => {
                            info!("Hedge order placed with request ID: {}", request_id);

                            // Track this as an active order
                            let order_info = OrderInfo {
                                order_type: hedge_side,
                                price: self.estimated_market_price,
                                size: balance, // Full account balance
                                request_id: request_id.clone(),
                                placed_at: chrono::Utc::now(),
                            };
                            self.active_orders.insert(hedge_account, order_info);
                            self.stats.orders_placed += 1;
                        }
                        Err(e) => {
                            error!("Failed to place hedge order: {}", e);
                            // Return account to available pool if hedge failed
                            self.available_accounts.push((hedge_account, balance));
                        }
                    }
                } else {
                    info!(
                        "Paper trading: Would place hedge order: {:?} using {} sats",
                        hedge_side, balance
                    );
                    // Return account since we didn't actually use it in paper trading
                    self.available_accounts.push((hedge_account, balance));
                }
            } else {
                warn!("No available accounts for hedging inventory");
            }
        }

        // Update max inventory reached statistic
        self.stats.max_inventory_reached = self.stats.max_inventory_reached.max(inventory_abs);

        Ok(())
    }

    /// Place market making orders using available accounts
    async fn place_market_making_orders(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        // Don't place too many orders at once
        if self.active_orders.len() >= 4 {
            info!(
                "Too many active orders ({}), waiting",
                self.active_orders.len()
            );
            return Ok(());
        }

        // Need at least 2 accounts to place buy and sell orders
        if self.available_accounts.len() < 2 {
            info!(
                "Not enough available accounts ({}), waiting for rotations",
                self.available_accounts.len()
            );
            return Ok(());
        }

        let mid_price = self.estimated_market_price;
        let spread_amount = (mid_price as f64 * self.config.spread) as u64;

        let buy_price = mid_price.saturating_sub(spread_amount / 2);
        let sell_price = mid_price + spread_amount / 2;

        info!(
            "Placing market making orders: buy @ {}, sell @ {} (spread: {} sats)",
            buy_price, sell_price, spread_amount
        );

        // Place buy order (if not too long on inventory)
        if self.inventory < self.config.max_inventory / 2 {
            if let Some((buy_account, balance)) = self.get_available_account(order_wallet) {
                if let Err(e) = self
                    .place_limit_order(
                        order_wallet,
                        buy_account,
                        PositionType::LONG,
                        buy_price,
                        balance, // Use full account balance
                    )
                    .await
                {
                    error!("Failed to place buy order: {}", e);
                    // Return account to available pool if order failed
                    self.available_accounts.push((buy_account, balance));
                }
            } else {
                info!("No valid accounts available for buy order");
            }
        }

        // Place sell order (if not too short on inventory)
        if self.inventory > -(self.config.max_inventory / 2) {
            if let Some((sell_account, balance)) = self.get_available_account(order_wallet) {
                if let Err(e) = self
                    .place_limit_order(
                        order_wallet,
                        sell_account,
                        PositionType::SHORT,
                        sell_price,
                        balance, // Use full account balance
                    )
                    .await
                {
                    error!("Failed to place sell order: {}", e);
                    // Return account to available pool if order failed
                    self.available_accounts.push((sell_account, balance));
                }
            } else {
                info!("No valid accounts available for sell order");
            }
        }

        Ok(())
    }

    /// Place a limit order
    async fn place_limit_order(
        &mut self,
        order_wallet: &mut OrderWallet,
        account_index: AccountIndex,
        order_type: PositionType,
        price: u64,
        size: u64,
    ) -> Result<()> {
        if self.config.paper_trading {
            info!(
                "Paper trading: Would place {:?} order at {} for {} sats on account {}",
                order_type, price, size, account_index
            );
            return Ok(());
        }

        info!(
            "Placing {:?} order at {} for {} sats on account {}",
            order_type, price, size, account_index
        );

        // Validate account state before placing order
        let account = order_wallet
            .zk_accounts
            .get_account(&account_index)
            .map_err(|e| anyhow::anyhow!("Account {} not found: {}", account_index, e))?;

        info!(
            "Account {} state: balance={}, io_type={:?}",
            account_index, account.balance, account.io_type
        );
        // Check if account is in correct state for placing orders
        if account.io_type != IOType::Coin {
            return Err(anyhow::anyhow!(
                "Account {} is not in Coin state (current: {:?}). Cannot place order.",
                account_index,
                account.io_type
            ));
        }
        // Validate order parameters
        if price == 0 {
            return Err(anyhow::anyhow!("Invalid price: {}", price));
        }

        let leverage = 2u64; // Low leverage for market making
        if leverage == 0 || leverage > 50 {
            return Err(anyhow::anyhow!("Invalid leverage: {}", leverage));
        }
        // Check if account has sufficient balance
        if account.balance == 0 {
            return Err(anyhow::anyhow!(
                "Account {} has zero balance. Cannot place order.",
                account_index
            ));
        }
        let request_id = order_wallet
            .open_trader_order(
                account_index,
                OrderType::LIMIT,
                order_type.clone(),
                price,
                leverage, // Low leverage for market making
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to place limit order on account {}: {}",
                    account_index,
                    e
                )
            })?;

        let order_info = OrderInfo {
            order_type,
            price,
            size,
            request_id: request_id.clone(),
            placed_at: chrono::Utc::now(),
        };

        self.active_orders.insert(account_index, order_info);
        self.stats.orders_placed += 1;

        info!("Order placed with request ID: {}", request_id);

        Ok(())
    }

    /// Log current market maker status
    fn log_status(&self) {
        info!("=== Market Maker Status ===");
        info!("Estimated market price: {}", self.estimated_market_price);
        info!("Current inventory: {} sats", self.inventory);
        info!("Available accounts: {}", self.available_accounts.len());
        info!("Active orders: {}", self.active_orders.len());

        // Show account balances
        let total_available_balance: u64 = self
            .available_accounts
            .iter()
            .map(|(_, balance)| balance)
            .sum();
        info!("Total available balance: {} sats", total_available_balance);

        info!("Orders placed: {}", self.stats.orders_placed);
        info!("Orders filled: {}", self.stats.orders_filled);
        info!("Total volume: {} sats", self.stats.total_volume);
        info!(
            "Fill ratio: {:.2}%",
            if self.stats.orders_placed > 0 {
                (self.stats.orders_filled as f64 / self.stats.orders_placed as f64) * 100.0
            } else {
                0.0
            }
        );
        info!(
            "Max inventory reached: {} sats",
            self.stats.max_inventory_reached
        );
        info!("Uptime: {}s", self.stats.uptime_seconds);
        info!("============================");
    }

    /// Close all active orders and positions
    async fn shutdown(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        info!("Shutting down market maker...");

        // Cancel all active orders
        for account_index in self.active_orders.keys() {
            info!("Cancelling order on account {}", account_index);
            if let Err(e) = order_wallet.cancel_trader_order(*account_index).await {
                error!("Failed to cancel order on account {}: {}", account_index, e);
            }
        }

        // Try to close any remaining positions on active order accounts
        for account_index in self.active_orders.keys() {
            if let Err(e) = order_wallet
                .close_trader_order(*account_index, OrderType::MARKET, 0.0)
                .await
            {
                // It's okay if this fails (position might not exist or already closed)
                warn!(
                    "Could not close position on account {}: {}",
                    account_index, e
                );
            }
        }

        self.log_status();
        info!("Market maker shutdown complete");
        info!(
            "Total accounts managed: {} available + {} active",
            self.available_accounts.len(),
            self.active_orders.len()
        );

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    // Load environment variables
    dotenv::dotenv().ok();

    // Parse command line arguments
    let args = Args::parse();

    info!("Starting Simple Market Maker Bot");
    info!("Spread: {:.3}%", args.spread * 100.0);
    info!("Order size: {} sats", args.order_size);
    info!("Max inventory: {} sats", args.max_inventory);
    info!("Paper trading: {}", args.paper_trading);
    info!("Enhanced market data: {}", args.enhanced_market_data);

    // Create market maker
    let mut market_maker = MarketMaker::new(args);
    let wallet_id = "simple_market_maker".to_string();
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
    let mut wallet = order_wallet.wallet.clone();
    // Initialize OrderWallet
    let _ = nyks_wallet::wallet::get_test_tokens(&mut wallet).await?;
    // Check initial balance
    let initial_balance = order_wallet
        .wallet
        .update_balance()
        .await
        .context("Failed to get initial balance")?;

    info!(
        "Initial wallet balance: {} sats, {} nyks",
        initial_balance.sats, initial_balance.nyks
    );

    if initial_balance.sats < market_maker.config.initial_capital {
        return Err(anyhow::anyhow!(
            "Insufficient balance. Required: {} sats, Available: {} sats",
            market_maker.config.initial_capital,
            initial_balance.sats
        ));
    }

    // Initialize trading accounts
    market_maker
        .initialize_accounts(&mut order_wallet)
        .await
        .context("Failed to initialize trading accounts")?;

    // Set up shutdown handler
    let shutdown_result = tokio::select! {
        result = market_maker.run(&mut order_wallet) => {
            result.context("Market maker execution failed")
        },
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
            market_maker.shutdown(&mut order_wallet).await?;
            drop(order_wallet);
            Ok(())
        }
    };

    match shutdown_result {
        Ok(_) => info!("Market maker finished successfully"),
        Err(e) => error!("Market maker error: {}", e),
    }

    Ok(())
}
