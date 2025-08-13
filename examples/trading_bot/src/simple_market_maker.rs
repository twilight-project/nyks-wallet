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
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};
use tokio::time::{interval, sleep};
use twilight_client_sdk::relayer_types::{OrderStatus, OrderType, PositionType};

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
    /// Trading accounts
    buy_account: Option<AccountIndex>,
    sell_account: Option<AccountIndex>,
    hedge_account: Option<AccountIndex>,
    /// Current orders
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
            buy_account: None,
            sell_account: None,
            hedge_account: None,
            active_orders: HashMap::new(),
            inventory: 0,
            stats: MarketMakerStats::default(),
            estimated_market_price: 50000, // Default starting price
        }
    }

    /// Initialize trading accounts for market making
    async fn initialize_accounts(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        info!("Initializing market maker accounts...");

        let account_capital = self.config.initial_capital / 3; // Split capital across 3 accounts

        // Create buy-side account
        let (tx_result, buy_account) = order_wallet
            .funding_to_trading(account_capital)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fund buy account: {}", e))?;

        info!(
            "Created buy account {} with {} sats (tx: {})",
            buy_account, account_capital, tx_result.tx_hash
        );
        self.buy_account = Some(buy_account);

        // Create sell-side account
        let (tx_result, sell_account) = order_wallet
            .funding_to_trading(account_capital)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fund sell account: {}", e))?;

        info!(
            "Created sell account {} with {} sats (tx: {})",
            sell_account, account_capital, tx_result.tx_hash
        );
        self.sell_account = Some(sell_account);

        // Create hedge account for inventory management
        let (tx_result, hedge_account) = order_wallet
            .funding_to_trading(account_capital)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fund hedge account: {}", e))?;

        info!(
            "Created hedge account {} with {} sats (tx: {})",
            hedge_account, account_capital, tx_result.tx_hash
        );
        self.hedge_account = Some(hedge_account);

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

    /// Update the status of active orders
    async fn update_orders(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        let mut filled_orders = Vec::new();

        for (account_index, order_info) in &self.active_orders {
            match order_wallet.query_trader_order(*account_index).await {
                Ok(trader_order) => {
                    match trader_order.order_status {
                        OrderStatus::FILLED => {
                            info!(
                                "Order filled on account {}: {:?}",
                                account_index, order_info.order_type
                            );
                            filled_orders.push(*account_index);

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
                        }
                        OrderStatus::CANCELLED => {
                            warn!("Order cancelled on account {}", account_index);
                            filled_orders.push(*account_index);
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
                        _ => {}
                    }
                }
                Err(e) => {
                    warn!("Failed to query order for account {}: {}", account_index, e);
                }
            }
        }

        // Remove filled/cancelled orders
        for account_index in filled_orders {
            self.active_orders.remove(&account_index);
        }

        Ok(())
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

            if let Some(hedge_account) = self.hedge_account {
                // Place hedge order to reduce inventory
                let hedge_side = if self.inventory > 0 {
                    PositionType::SHORT // Sell to reduce long inventory
                } else {
                    PositionType::LONG // Buy to reduce short inventory
                };

                let hedge_size = (inventory_abs - max_inventory) as u64;

                if !self.config.paper_trading {
                    info!("Placing hedge order: {:?} {} sats", hedge_side, hedge_size);

                    match order_wallet
                        .open_trader_order(
                            hedge_account,
                            OrderType::MARKET,
                            hedge_side,
                            self.estimated_market_price,
                            self.config.max_leverage,
                        )
                        .await
                    {
                        Ok(request_id) => {
                            info!("Hedge order placed with request ID: {}", request_id);
                        }
                        Err(e) => {
                            error!("Failed to place hedge order: {}", e);
                        }
                    }
                } else {
                    info!(
                        "Paper trading: Would place hedge order: {:?} {} sats",
                        hedge_side, hedge_size
                    );
                }
            }
        }

        // Update max inventory reached statistic
        self.stats.max_inventory_reached = self.stats.max_inventory_reached.max(inventory_abs);

        Ok(())
    }

    /// Place market making orders (buy and sell)
    async fn place_market_making_orders(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        // Don't place orders if we have too many active
        if self.active_orders.len() >= 2 {
            return Ok(());
        }

        let mid_price = self.estimated_market_price;
        let spread_amount = (mid_price as f64 * self.config.spread) as u64;

        let buy_price = mid_price.saturating_sub(spread_amount / 2);
        let sell_price = mid_price + spread_amount / 2;

        // Place buy order (if not too long on inventory)
        if self.inventory < self.config.max_inventory / 2 {
            if let Some(buy_account) = self.buy_account {
                if !self.active_orders.contains_key(&buy_account) {
                    if let Err(e) = self
                        .place_limit_order(
                            order_wallet,
                            buy_account,
                            PositionType::LONG,
                            buy_price,
                            self.config.order_size,
                        )
                        .await
                    {
                        error!("Failed to place buy order: {}", e);
                    }
                }
            }
        }

        // Place sell order (if not too short on inventory)
        if self.inventory > -(self.config.max_inventory / 2) {
            if let Some(sell_account) = self.sell_account {
                if !self.active_orders.contains_key(&sell_account) {
                    if let Err(e) = self
                        .place_limit_order(
                            order_wallet,
                            sell_account,
                            PositionType::SHORT,
                            sell_price,
                            self.config.order_size,
                        )
                        .await
                    {
                        error!("Failed to place sell order: {}", e);
                    }
                }
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

        let request_id = order_wallet
            .open_trader_order(
                account_index,
                OrderType::LIMIT,
                order_type.clone(),
                price,
                2, // Low leverage for market making
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to place limit order: {}", e))?;

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
        info!("Active orders: {}", self.active_orders.len());
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
        info!("Uptime: {}s", self.stats.uptime_seconds);
        info!("============================");
    }

    /// Close all active orders and positions
    async fn shutdown(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        info!("Shutting down market maker...");

        // Cancel all active orders
        for account_index in self.active_orders.keys() {
            if let Err(e) = order_wallet.cancel_trader_order(*account_index).await {
                error!("Failed to cancel order on account {}: {}", account_index, e);
            }
        }

        // Close any remaining positions
        let accounts = [self.buy_account, self.sell_account, self.hedge_account];
        for account_index in accounts.iter().filter_map(|&x| x) {
            if let Err(e) = order_wallet
                .close_trader_order(account_index, OrderType::MARKET, 0.0)
                .await
            {
                // It's okay if this fails (position might not exist)
                warn!(
                    "Could not close position on account {}: {}",
                    account_index, e
                );
            }
        }

        self.log_status();
        info!("Market maker shutdown complete");

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
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

    // Initialize OrderWallet
    let mut order_wallet = OrderWallet::new(None).context("Failed to create OrderWallet")?;
    let _ = nyks_wallet::wallet::get_test_tokens(&mut order_wallet.wallet).await?;
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
            Ok(())
        }
    };

    match shutdown_result {
        Ok(_) => info!("Market maker finished successfully"),
        Err(e) => error!("Market maker error: {}", e),
    }

    Ok(())
}
