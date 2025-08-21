//! # Automated Lending Bot
//!
//! This example demonstrates an automated lending strategy that provides liquidity
//! to the Twilight Protocol lending markets for yield generation.
//!
//! ## Strategy Overview
//! - Automatically provides liquidity for lending operations
//! - Monitors lending rates and adjusts positions accordingly
//! - Implements risk management for lending exposure
//! - Tracks yield generation and performance metrics
//!
//! ## Usage
//! ```bash
//! cargo run --bin lending_bot -- --min-rate 0.05 --max-exposure 0.8 --lending-amount 10000
//! ```

use anyhow::{Context, Result};
use clap::Parser;
use log::{error, info, warn};
use nyks_wallet::relayer_module::order_wallet::{AccountIndex, OrderWallet};
use nyks_wallet::relayer_module::relayer_types::{LendOrder, OrderStatus};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};
use tokio::time::{interval, sleep};

/// Lending bot command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Minimum acceptable lending rate (annual %)
    #[arg(long, default_value = "0.05")]
    min_rate: f64,

    /// Maximum exposure as percentage of capital (0.8 = 80%)
    #[arg(long, default_value = "0.8")]
    max_exposure: f64,

    /// Default lending amount per order
    #[arg(short, long, default_value = "10000")]
    lending_amount: u64,

    /// Initial capital in satoshis
    #[arg(short, long, default_value = "50000")]
    initial_capital: u64,

    /// Rate monitoring interval in seconds
    #[arg(short, long, default_value = "5")]
    monitoring_interval: u64,

    /// Enable paper trading mode
    #[arg(short, long)]
    paper_trading: bool,

    /// Maximum number of concurrent lending positions
    #[arg(long, default_value = "5")]
    max_positions: u32,

    /// Auto-reinvest profits
    #[arg(long)]
    auto_reinvest: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct LendingBot {
    /// Configuration
    config: LendingConfig,
    /// Available lending accounts (fresh, ready to use)
    available_accounts: Vec<(AccountIndex, u64)>, // (account_index, balance)
    /// Active lending positions
    active_positions: HashMap<AccountIndex, LendingPosition>,
    /// Statistics
    stats: LendingStats,
    /// Market data
    market_data: LendingMarketData,
}

#[derive(Debug, Serialize, Deserialize)]
struct LendingConfig {
    min_lending_rate: f64,
    max_exposure_percentage: f64,
    lending_amount: u64,
    initial_capital: u64,
    monitoring_interval: Duration,
    paper_trading: bool,
    max_positions: u32,
    auto_reinvest: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct LendingPosition {
    account_index: AccountIndex,
    principal_amount: u64,
    lending_rate: f64,
    started_at: chrono::DateTime<chrono::Utc>,
    request_id: String,
    current_value: u64,
    accrued_interest: u64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct LendingStats {
    total_positions_opened: u32,
    total_positions_closed: u32,
    total_principal_lent: u64,
    total_interest_earned: u64,
    average_lending_rate: f64,
    average_position_duration: f64,
    current_apy: f64,
    total_yield: f64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct LendingMarketData {
    current_lending_rates: Vec<f64>,
    average_rate: f64,
    market_utilization: f64,
    total_available_liquidity: u64,
    estimated_demand: u64,
}

impl LendingBot {
    /// Create a new lending bot with the given configuration
    fn new(args: Args) -> Self {
        Self {
            config: LendingConfig {
                min_lending_rate: args.min_rate,
                max_exposure_percentage: args.max_exposure,
                lending_amount: args.lending_amount,
                initial_capital: args.initial_capital,
                monitoring_interval: Duration::from_secs(args.monitoring_interval),
                paper_trading: args.paper_trading,
                max_positions: args.max_positions,
                auto_reinvest: args.auto_reinvest,
            },
            available_accounts: Vec::new(),
            active_positions: HashMap::new(),
            stats: LendingStats::default(),
            market_data: LendingMarketData::default(),
        }
    }

    /// Initialize lending accounts using ZkOS pattern
    async fn initialize_accounts(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        info!("Initializing lending accounts using ZkOS pattern...");

        // Step 1: Create a master trading account with the full capital
        let (tx_result, master_account) = order_wallet
            .funding_to_trading(self.config.initial_capital)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create master lending account: {}", e))?;

        if tx_result.code != 0 {
            return Err(anyhow::anyhow!(
                "Master account creation failed with code: {}",
                tx_result.code
            ));
        }

        info!(
            "Created master lending account {} with {} sats (tx: {})",
            master_account, self.config.initial_capital, tx_result.tx_hash
        );

        // Step 2: Split the master account into multiple smaller accounts for lending
        let account_count = self.config.max_positions;
        let capital_per_account = self.config.initial_capital / account_count as u64;
        let splits = vec![capital_per_account; account_count as usize];

        info!(
            "Splitting master account into {} accounts with {} sats each",
            account_count, capital_per_account
        );

        let accounts = order_wallet
            .trading_to_trading_multiple_accounts(master_account, splits)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to split accounts: {}", e))?;

        // Store all accounts as available for lending
        self.available_accounts = accounts;

        info!(
            "Successfully created {} lending accounts, ready for lending",
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

    /// Run the automated lending strategy
    async fn run(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        info!("Starting automated lending bot");
        info!("Minimum rate: {:.2}%", self.config.min_lending_rate * 100.0);
        info!(
            "Maximum exposure: {:.1}%",
            self.config.max_exposure_percentage * 100.0
        );

        let mut monitoring_timer = interval(self.config.monitoring_interval);

        loop {
            monitoring_timer.tick().await;

            // Update market data
            if let Err(e) = self.update_market_data().await {
                error!("Error updating market data: {}", e);
                continue;
            }

            // Check active positions
            if let Err(e) = self.check_active_positions(order_wallet).await {
                error!("Error checking active positions: {}", e);
            }

            // Execute lending strategy
            if let Err(e) = self.execute_lending_strategy(order_wallet).await {
                error!("Error executing lending strategy: {}", e);
                sleep(Duration::from_secs(10)).await;
            }

            // Update statistics
            self.update_statistics();

            // Log status periodically
            if self.stats.total_positions_opened > 0 && self.stats.total_positions_opened % 5 == 0 {
                self.log_status();
            }
        }
    }

    /// Update lending market data
    async fn update_market_data(&mut self) -> Result<()> {
        // In a real implementation, you would:
        // 1. Query current lending rates from the protocol
        // 2. Analyze market utilization and demand
        // 3. Monitor competitor rates and strategies
        // 4. Calculate optimal lending rates

        // For demonstration, simulate market data
        let base_rate = 0.08; // 8% base rate
        let rate_variance = (rand::random::<f64>() - 0.5) * 0.02; // ±1% variance
        let current_rate = base_rate + rate_variance;

        self.market_data.current_lending_rates.push(current_rate);

        // Keep only last 20 data points
        if self.market_data.current_lending_rates.len() > 20 {
            self.market_data.current_lending_rates.remove(0);
        }

        // Calculate average rate
        self.market_data.average_rate = self.market_data.current_lending_rates.iter().sum::<f64>()
            / self.market_data.current_lending_rates.len() as f64;

        // Simulate market utilization
        self.market_data.market_utilization = 0.6 + (rand::random::<f64>() - 0.5) * 0.3;
        self.market_data.total_available_liquidity = 1_000_000 + rand::random::<u64>() % 500_000;
        self.market_data.estimated_demand = 800_000 + rand::random::<u64>() % 400_000;

        info!(
            "Market update - Rate: {:.3}%, Utilization: {:.1}%, Avg Rate: {:.3}%",
            current_rate * 100.0,
            self.market_data.market_utilization * 100.0,
            self.market_data.average_rate * 100.0
        );

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

    /// Get an available account that's ready for lending (Coin state, non-zero balance)
    fn get_available_account(&mut self, order_wallet: &OrderWallet) -> Option<(AccountIndex, u64)> {
        use nyks_wallet::relayer_module::relayer_types::IOType;

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

    /// Check status of active lending positions and handle account rotation
    async fn check_active_positions(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        let mut completed_positions = Vec::new();

        // Collect account indices first to avoid borrowing issues
        let account_indices: Vec<AccountIndex> = self.active_positions.keys().copied().collect();

        for account_index in account_indices {
            if let Some(position) = self.active_positions.get_mut(&account_index) {
                match order_wallet.query_lend_order(account_index).await {
                    Ok(lend_order) => {
                        use nyks_wallet::relayer_module::relayer_types::OrderStatus;

                        // Update position data
                        position.current_value = lend_order.balance as u64;
                        position.accrued_interest = position
                            .current_value
                            .saturating_sub(position.principal_amount);

                        match lend_order.order_status {
                            OrderStatus::FILLED => {
                                info!(
                                    "Lend order filled on account {}: {} sats",
                                    account_index, position.current_value
                                );

                                // Check if we should close this position based on strategy criteria
                                if Self::should_close_position_static(
                                    position,
                                    &lend_order,
                                    &self.market_data,
                                ) {
                                    info!(
                                        "Strategy criteria met, closing lending position on account {}",
                                        account_index
                                    );

                                    // Close the position to initiate settlement
                                    match order_wallet.close_lend_order(account_index).await {
                                        Ok(close_request_id) => {
                                            info!(
                                                "Lending position close initiated with request ID: {}",
                                                close_request_id
                                            );
                                        }
                                        Err(e) => {
                                            error!("Failed to close lending position: {}", e);
                                        }
                                    }
                                }
                                // Note: Position will remain active until it becomes SETTLED
                            }
                            OrderStatus::SETTLED => {
                                info!(
                                    "Lending position settled on account {}, rotating account",
                                    account_index
                                );

                                // Update statistics
                                self.stats.total_positions_closed += 1;
                                self.stats.total_interest_earned += position.accrued_interest;

                                // Rotate the account to get a fresh one
                                match order_wallet.trading_to_trading(account_index).await {
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
                                            completed_positions.push(account_index);
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
                            _ => {
                                // For lending operations, only FILLED and SETTLED are valid statuses
                                warn!(
                                    "Unexpected lending order status on account {}: {:?}",
                                    account_index, lend_order.order_status
                                );
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to query lend order for account {}: {}",
                            account_index, e
                        );
                    }
                }
            }
        }

        // Remove completed positions from active tracking
        for account_index in completed_positions {
            self.active_positions.remove(&account_index);
        }

        Ok(())
    }

    /// Determine if a lending position should be closed
    fn should_close_position(&self, position: &LendingPosition, lend_order: &LendOrder) -> bool {
        // Close if rates have improved significantly
        if self.market_data.average_rate > position.lending_rate * 1.2 {
            info!(
                "Closing position due to better rates available: {:.3}% vs {:.3}%",
                self.market_data.average_rate * 100.0,
                position.lending_rate * 100.0
            );
            return true;
        }

        // Close mature positions (example: after 24 hours)
        let position_age = chrono::Utc::now()
            .signed_duration_since(position.started_at)
            .num_hours();

        if position_age > 24 {
            info!("Closing mature lending position ({}h old)", position_age);
            return true;
        }

        // Close if order status indicates completion
        if lend_order.order_status == OrderStatus::FILLED {
            let yield_earned = position.accrued_interest as f64 / position.principal_amount as f64;
            if yield_earned > 0.01 {
                // 1% minimum yield
                info!(
                    "Closing profitable lending position with {:.2}% yield",
                    yield_earned * 100.0
                );
                return true;
            }
        }

        false
    }

    /// Static version of should_close_position to avoid borrowing issues
    fn should_close_position_static(
        position: &LendingPosition,
        lend_order: &LendOrder,
        market_data: &LendingMarketData,
    ) -> bool {
        // Close if rates have improved significantly
        if market_data.average_rate > position.lending_rate * 1.2 {
            info!(
                "Closing position due to better rates available: {:.3}% vs {:.3}%",
                market_data.average_rate * 100.0,
                position.lending_rate * 100.0
            );
            return true;
        }

        // Close mature positions (example: after 24 hours)
        let position_age = chrono::Utc::now()
            .signed_duration_since(position.started_at)
            .num_hours();

        if position_age > 24 {
            info!("Closing mature lending position ({}h old)", position_age);
            return true;
        }

        // Close if order status indicates completion
        if lend_order.order_status == OrderStatus::FILLED {
            let yield_earned = position.accrued_interest as f64 / position.principal_amount as f64;
            if yield_earned > 0.01 {
                // 1% minimum yield
                info!(
                    "Closing profitable lending position with {:.2}% yield",
                    yield_earned * 100.0
                );
                return true;
            }
        }

        false
    }

    /// Execute lending strategy decisions
    async fn execute_lending_strategy(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        // Check if we can open new positions
        let current_exposure = self.calculate_current_exposure();
        let max_exposure = self.config.initial_capital as f64 * self.config.max_exposure_percentage;

        if current_exposure >= max_exposure {
            info!(
                "Maximum exposure reached: {:.0}/{:.0} sats",
                current_exposure, max_exposure
            );
            return Ok(());
        }

        // Check if current rates are attractive
        if self.market_data.average_rate < self.config.min_lending_rate {
            info!(
                "Current rates below minimum: {:.3}% < {:.3}%",
                self.market_data.average_rate * 100.0,
                self.config.min_lending_rate * 100.0
            );
            return Ok(());
        }

        // Find available account for new lending position
        if let Some((account_index, account_balance)) = self.get_available_account(order_wallet) {
            info!(
                "Using account {} with {} sats for new lending position",
                account_index, account_balance
            );

            self.open_lending_position(order_wallet, account_index, account_balance)
                .await?;
        } else {
            info!("No available accounts for new lending positions");
        }

        Ok(())
    }

    /// Calculate current lending exposure
    fn calculate_current_exposure(&self) -> f64 {
        self.active_positions
            .values()
            .map(|pos| pos.principal_amount as f64)
            .sum()
    }

    /// Calculate optimal lending amount based on market conditions
    fn calculate_optimal_lending_amount(&self) -> u64 {
        let base_amount = self.config.lending_amount;

        // Adjust based on market conditions
        let rate_multiplier = if self.market_data.average_rate > self.config.min_lending_rate * 1.5
        {
            1.5 // Increase lending when rates are high
        } else {
            1.0
        };

        let utilization_multiplier = if self.market_data.market_utilization > 0.8 {
            1.2 // Increase lending when utilization is high
        } else {
            1.0
        };

        let optimal_amount = (base_amount as f64 * rate_multiplier * utilization_multiplier) as u64;

        // With ZkOS, we use the full account balance rather than calculating optimal amounts
        // This method is kept for compatibility but the actual amount used will be the full account balance
        optimal_amount
    }

    /// Open a new lending position using full account balance (ZkOS compliant)
    async fn open_lending_position(
        &mut self,
        order_wallet: &mut OrderWallet,
        account_index: AccountIndex,
        account_balance: u64, // Full account balance - must be used entirely
    ) -> Result<()> {
        use nyks_wallet::relayer_module::relayer_types::IOType;

        if self.config.paper_trading {
            info!(
                "Paper trading: Would open lending position using {} sats at {:.3}% rate on account {}",
                account_balance,
                self.market_data.average_rate * 100.0,
                account_index
            );
            // Return account to available pool since we didn't actually use it
            self.available_accounts
                .push((account_index, account_balance));
            return Ok(());
        }

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
                "Account {} is not in Coin state (current: {:?}). Cannot place lending order.",
                account_index,
                account.io_type
            ));
        }

        // Check if account has sufficient balance
        if account.balance == 0 {
            return Err(anyhow::anyhow!(
                "Account {} has zero balance. Cannot place lending order.",
                account_index
            ));
        }

        info!(
            "Opening lending position using {} sats at {:.3}% rate on account {}",
            account_balance,
            self.market_data.average_rate * 100.0,
            account_index
        );

        let request_id = order_wallet
            .open_lend_order(account_index)
            .await
            .map_err(|e| {
                // Return account to available pool if order failed
                self.available_accounts
                    .push((account_index, account_balance));
                anyhow::anyhow!(
                    "Failed to open lend order on account {}: {}",
                    account_index,
                    e
                )
            })?;

        let position = LendingPosition {
            account_index,
            principal_amount: account_balance, // Use full account balance
            lending_rate: self.market_data.average_rate,
            started_at: chrono::Utc::now(),
            request_id: request_id.clone(),
            current_value: account_balance,
            accrued_interest: 0,
        };

        self.active_positions.insert(account_index, position);
        self.stats.total_positions_opened += 1;
        self.stats.total_principal_lent += account_balance;

        info!("Lending position opened with request ID: {}", request_id);

        Ok(())
    }

    /// Close a lending position (simplified - rotation handled in check_active_positions)
    async fn close_lending_position(
        &mut self,
        order_wallet: &mut OrderWallet,
        account_index: AccountIndex,
    ) -> Result<()> {
        if let Some(position) = self.active_positions.get(&account_index) {
            if self.config.paper_trading {
                info!(
                    "Paper trading: Would close lending position on account {}",
                    account_index
                );
                return Ok(());
            }

            info!(
                "Initiating close of lending position on account {}",
                account_index
            );

            // Simply initiate the close - the actual rotation will be handled in check_active_positions
            // when the order status becomes SETTLED
            let _close_request_id =
                order_wallet
                    .close_lend_order(account_index)
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to close lend order on account {}: {}",
                            account_index,
                            e
                        )
                    })?;

            let duration_hours = chrono::Utc::now()
                .signed_duration_since(position.started_at)
                .num_minutes() as f64
                / 60.0;

            info!(
                "Lending position close initiated: {} sats principal, {:.1}h duration",
                position.principal_amount, duration_hours,
            );
        }

        Ok(())
    }

    /// Update performance statistics
    fn update_statistics(&mut self) {
        if self.stats.total_positions_opened > 0 {
            // Calculate average lending rate
            let total_rate: f64 = self
                .active_positions
                .values()
                .map(|pos| pos.lending_rate)
                .sum();

            if !self.active_positions.is_empty() {
                self.stats.average_lending_rate = total_rate / self.active_positions.len() as f64;
            }

            // Calculate current APY
            if self.stats.total_principal_lent > 0 {
                self.stats.current_apy = (self.stats.total_interest_earned as f64
                    / self.stats.total_principal_lent as f64)
                    * 365.0;
            }

            // Calculate total yield
            let total_capital = self.config.initial_capital;
            self.stats.total_yield = self.stats.total_interest_earned as f64 / total_capital as f64;
        }
    }

    /// Log current lending bot status
    fn log_status(&self) {
        info!("=== Lending Bot Status ===");

        // Account information
        info!("Available accounts: {}", self.available_accounts.len());
        let total_available_balance: u64 = self
            .available_accounts
            .iter()
            .map(|(_, balance)| balance)
            .sum();
        info!("Total available balance: {} sats", total_available_balance);

        info!("Active positions: {}", self.active_positions.len());
        info!("Total lent: {} sats", self.stats.total_principal_lent);
        info!(
            "Total interest earned: {} sats",
            self.stats.total_interest_earned
        );
        info!(
            "Average lending rate: {:.3}%",
            self.stats.average_lending_rate * 100.0
        );
        info!(
            "Current market rate: {:.3}%",
            self.market_data.average_rate * 100.0
        );
        info!("Estimated APY: {:.2}%", self.stats.current_apy * 100.0);
        info!("Total yield: {:.2}%", self.stats.total_yield * 100.0);
        info!(
            "Market utilization: {:.1}%",
            self.market_data.market_utilization * 100.0
        );
        info!("===========================");
    }

    /// Close all lending positions
    async fn shutdown(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        info!("Shutting down lending bot...");

        let account_indices: Vec<AccountIndex> = self.active_positions.keys().copied().collect();

        for account_index in account_indices {
            if let Err(e) = self
                .close_lending_position(order_wallet, account_index)
                .await
            {
                error!(
                    "Failed to close lending position for account {}: {}",
                    account_index, e
                );
            }
        }

        self.log_status();
        info!("Lending bot shutdown complete");

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

    info!("Starting Automated Lending Bot");
    info!("Minimum rate: {:.2}%", args.min_rate * 100.0);
    info!("Maximum exposure: {:.1}%", args.max_exposure * 100.0);
    info!("Lending amount: {} sats", args.lending_amount);
    info!("Auto-reinvest: {}", args.auto_reinvest);
    info!("Paper trading: {}", args.paper_trading);

    // Create lending bot
    let mut lending_bot = LendingBot::new(args);
    let wallet_id = "lending_bot".to_string();
    let mut order_wallet;
    // Initialize OrderWallet
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

    // Get test tokens
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

    if initial_balance.sats < lending_bot.config.initial_capital {
        return Err(anyhow::anyhow!(
            "Insufficient balance. Required: {} sats, Available: {} sats",
            lending_bot.config.initial_capital,
            initial_balance.sats
        ));
    }

    // Initialize lending accounts
    lending_bot
        .initialize_accounts(&mut order_wallet)
        .await
        .context("Failed to initialize lending accounts")?;

    // Set up shutdown handler
    let shutdown_result = tokio::select! {
        result = lending_bot.run(&mut order_wallet) => {
            result.context("Lending bot execution failed")
        },
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
            lending_bot.shutdown(&mut order_wallet).await?;
            Ok(())
        }
    };

    match shutdown_result {
        Ok(_) => info!("Lending bot finished successfully"),
        Err(e) => error!("Lending bot error: {}", e),
    }

    Ok(())
}
