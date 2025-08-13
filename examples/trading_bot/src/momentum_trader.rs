//! # Momentum Trading Bot
//!
//! This example demonstrates a momentum-based trading strategy using technical indicators
//! and market analysis to identify trend-following opportunities.
//!
//! ## Strategy Overview
//! - Uses moving averages and RSI for trend identification
//! - Implements position sizing based on signal strength
//! - Includes stop-loss and take-profit management
//! - Tracks multiple timeframes for confirmation
//!
//! ## Usage
//! ```bash
//! cargo run --bin momentum_trader -- --fast-ma 10 --slow-ma 30 --rsi-period 14 --position-size 5000
//! ```

use anyhow::{Context, Result};
use clap::Parser;
use log::{error, info, warn};
use nyks_wallet::relayer_module::order_wallet::{AccountIndex, OrderWallet};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, time::Duration};
use tokio::time::{interval, sleep};
use twilight_client_sdk::relayer_types::{OrderStatus, OrderType, PositionType};

/// Momentum trader command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Fast moving average period
    #[arg(long, default_value = "10")]
    fast_ma: usize,

    /// Slow moving average period
    #[arg(long, default_value = "30")]
    slow_ma: usize,

    /// RSI calculation period
    #[arg(long, default_value = "14")]
    rsi_period: usize,

    /// Position size in satoshis
    #[arg(short, long, default_value = "5000")]
    position_size: u64,

    /// Maximum leverage to use
    #[arg(short, long, default_value = "5")]
    max_leverage: u64,

    /// Initial capital in satoshis
    #[arg(short, long, default_value = "50000")]
    initial_capital: u64,

    /// Analysis interval in seconds
    #[arg(short, long, default_value = "60")]
    analysis_interval: u64,

    /// Stop loss percentage (0.05 = 5%)
    #[arg(long, default_value = "0.05")]
    stop_loss: f64,

    /// Take profit percentage (0.15 = 15%)
    #[arg(long, default_value = "0.15")]
    take_profit: f64,

    /// Enable paper trading mode
    #[arg(short, long)]
    paper_trading: bool,

    /// Minimum signal strength (0.0-1.0)
    #[arg(long, default_value = "0.7")]
    min_signal_strength: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct MomentumTrader {
    /// Configuration
    config: MomentumConfig,
    /// Trading account
    trading_account: Option<AccountIndex>,
    /// Price history for analysis
    price_history: VecDeque<PricePoint>,
    /// Current position
    current_position: Option<Position>,
    /// Technical indicators
    indicators: TechnicalIndicators,
    /// Statistics
    stats: TradingStats,
}

#[derive(Debug, Serialize, Deserialize)]
struct MomentumConfig {
    fast_ma_period: usize,
    slow_ma_period: usize,
    rsi_period: usize,
    position_size: u64,
    max_leverage: u64,
    initial_capital: u64,
    analysis_interval: Duration,
    stop_loss_pct: f64,
    take_profit_pct: f64,
    paper_trading: bool,
    min_signal_strength: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PricePoint {
    timestamp: chrono::DateTime<chrono::Utc>,
    price: f64,
    volume: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Position {
    account_index: AccountIndex,
    position_type: PositionType,
    entry_price: f64,
    size: u64,
    leverage: u64,
    stop_loss: Option<f64>,
    take_profit: Option<f64>,
    request_id: String,
    opened_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct TechnicalIndicators {
    fast_ma: Option<f64>,
    slow_ma: Option<f64>,
    rsi: Option<f64>,
    momentum: Option<f64>,
    signal_strength: f64,
    trend_direction: TrendDirection,
}

#[derive(Debug, Serialize, Deserialize, Default)]
enum TrendDirection {
    #[default]
    Sideways,
    Bullish,
    Bearish,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct TradingStats {
    total_trades: u32,
    winning_trades: u32,
    losing_trades: u32,
    total_pnl: f64,
    max_drawdown: f64,
    max_profit: f64,
    win_rate: f64,
    average_hold_time: f64,
    sharpe_ratio: f64,
}

impl MomentumTrader {
    /// Create a new momentum trader with the given configuration
    fn new(args: Args) -> Self {
        let max_history = args.slow_ma.max(args.rsi_period) * 2; // Keep enough history for calculations

        Self {
            config: MomentumConfig {
                fast_ma_period: args.fast_ma,
                slow_ma_period: args.slow_ma,
                rsi_period: args.rsi_period,
                position_size: args.position_size,
                max_leverage: args.max_leverage,
                initial_capital: args.initial_capital,
                analysis_interval: Duration::from_secs(args.analysis_interval),
                stop_loss_pct: args.stop_loss,
                take_profit_pct: args.take_profit,
                paper_trading: args.paper_trading,
                min_signal_strength: args.min_signal_strength,
            },
            trading_account: None,
            price_history: VecDeque::with_capacity(max_history),
            current_position: None,
            indicators: TechnicalIndicators::default(),
            stats: TradingStats::default(),
        }
    }

    /// Initialize trading account
    async fn initialize_account(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        info!("Initializing momentum trading account...");

        let (tx_result, account_index) = order_wallet
            .funding_to_trading(self.config.initial_capital)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fund trading account: {}", e))?;

        info!(
            "Created trading account {} with {} sats (tx: {})",
            account_index, self.config.initial_capital, tx_result.tx_hash
        );

        self.trading_account = Some(account_index);
        Ok(())
    }

    /// Run the momentum trading strategy
    async fn run(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        info!("Starting momentum trader");
        info!(
            "Fast MA: {}, Slow MA: {}, RSI: {}",
            self.config.fast_ma_period, self.config.slow_ma_period, self.config.rsi_period
        );

        let mut analysis_timer = interval(self.config.analysis_interval);

        loop {
            analysis_timer.tick().await;

            // Update market data
            if let Err(e) = self.update_market_data().await {
                error!("Error updating market data: {}", e);
                continue;
            }

            // Check current position
            if let Err(e) = self.check_position(order_wallet).await {
                error!("Error checking position: {}", e);
            }

            // Analyze market and generate signals
            self.analyze_market();

            // Execute trading decisions
            if let Err(e) = self.execute_trading_decision(order_wallet).await {
                error!("Error executing trading decision: {}", e);
                sleep(Duration::from_secs(5)).await;
            }

            // Log status periodically
            if self.stats.total_trades > 0 && self.stats.total_trades % 10 == 0 {
                self.log_status();
            }
        }
    }

    /// Update market data (simulated for demonstration)
    async fn update_market_data(&mut self) -> Result<()> {
        // In a real implementation, you would:
        // 1. Fetch data from multiple exchanges
        // 2. Calculate VWAP and other price metrics
        // 3. Get order book data for volume analysis
        // 4. Handle websocket feeds for real-time data

        // For demonstration, simulate price data with some realistic movement
        let current_time = chrono::Utc::now();

        let base_price = if let Some(last_point) = self.price_history.back() {
            last_point.price
        } else {
            50000.0 // Starting price
        };

        // Simulate price movement with some trending behavior
        let trend_factor = match self.indicators.trend_direction {
            TrendDirection::Bullish => 0.0002,
            TrendDirection::Bearish => -0.0002,
            TrendDirection::Sideways => 0.0,
        };

        let random_factor = (rand::random::<f64>() - 0.5) * 0.002; // ±0.1% random
        let price_change = trend_factor + random_factor;
        let new_price = base_price * (1.0 + price_change);

        let volume = 1000.0 + rand::random::<f64>() * 500.0; // Random volume

        let price_point = PricePoint {
            timestamp: current_time,
            price: new_price,
            volume,
        };

        self.price_history.push_back(price_point);

        // Keep only necessary history
        let max_history = self.config.slow_ma_period.max(self.config.rsi_period) * 2;
        while self.price_history.len() > max_history {
            self.price_history.pop_front();
        }

        Ok(())
    }

    /// Analyze market data and calculate technical indicators
    fn analyze_market(&mut self) {
        if self.price_history.len() < self.config.slow_ma_period {
            return; // Not enough data yet
        }

        // Calculate moving averages
        self.indicators.fast_ma = self.calculate_sma(self.config.fast_ma_period);
        self.indicators.slow_ma = self.calculate_sma(self.config.slow_ma_period);

        // Calculate RSI
        self.indicators.rsi = self.calculate_rsi();

        // Calculate momentum
        self.indicators.momentum = self.calculate_momentum();

        // Determine trend direction
        self.update_trend_direction();

        // Calculate signal strength
        self.calculate_signal_strength();

        if let (Some(fast_ma), Some(slow_ma), Some(rsi)) = (
            self.indicators.fast_ma,
            self.indicators.slow_ma,
            self.indicators.rsi,
        ) {
            info!(
                "Technical analysis - Fast MA: {:.2}, Slow MA: {:.2}, RSI: {:.2}, Signal: {:.3}",
                fast_ma, slow_ma, rsi, self.indicators.signal_strength
            );
        }
    }

    /// Calculate Simple Moving Average
    fn calculate_sma(&self, period: usize) -> Option<f64> {
        if self.price_history.len() < period {
            return None;
        }

        let sum: f64 = self
            .price_history
            .iter()
            .rev()
            .take(period)
            .map(|p| p.price)
            .sum();

        Some(sum / period as f64)
    }

    /// Calculate Relative Strength Index (RSI)
    fn calculate_rsi(&self) -> Option<f64> {
        if self.price_history.len() < self.config.rsi_period + 1 {
            return None;
        }

        let mut gains = 0.0;
        let mut losses = 0.0;

        for i in 1..=self.config.rsi_period {
            let current = self.price_history[self.price_history.len() - i].price;
            let previous = self.price_history[self.price_history.len() - i - 1].price;

            let change = current - previous;
            if change > 0.0 {
                gains += change;
            } else {
                losses += -change;
            }
        }

        if losses == 0.0 {
            return Some(100.0);
        }

        let avg_gain = gains / self.config.rsi_period as f64;
        let avg_loss = losses / self.config.rsi_period as f64;
        let rs = avg_gain / avg_loss;
        let rsi = 100.0 - (100.0 / (1.0 + rs));

        Some(rsi)
    }

    /// Calculate price momentum
    fn calculate_momentum(&self) -> Option<f64> {
        if self.price_history.len() < 10 {
            return None;
        }

        let recent_price = self.price_history.back()?.price;
        let old_price = self.price_history[self.price_history.len() - 10].price;

        Some((recent_price - old_price) / old_price)
    }

    /// Update trend direction based on moving averages
    fn update_trend_direction(&mut self) {
        if let (Some(fast_ma), Some(slow_ma)) = (self.indicators.fast_ma, self.indicators.slow_ma) {
            let ma_diff = (fast_ma - slow_ma) / slow_ma;

            self.indicators.trend_direction = if ma_diff > 0.001 {
                TrendDirection::Bullish
            } else if ma_diff < -0.001 {
                TrendDirection::Bearish
            } else {
                TrendDirection::Sideways
            };
        }
    }

    /// Calculate overall signal strength
    fn calculate_signal_strength(&mut self) {
        let mut signal = 0.0;
        let mut factors = 0;

        // Moving average signal
        if let (Some(fast_ma), Some(slow_ma)) = (self.indicators.fast_ma, self.indicators.slow_ma) {
            let ma_signal = ((fast_ma - slow_ma) / slow_ma).abs().min(0.05) / 0.05;
            signal += ma_signal;
            factors += 1;
        }

        // RSI signal (overbought/oversold)
        if let Some(rsi) = self.indicators.rsi {
            let rsi_signal = if rsi > 70.0 || rsi < 30.0 {
                1.0 // Strong signal when overbought or oversold
            } else if rsi > 60.0 || rsi < 40.0 {
                0.5 // Moderate signal
            } else {
                0.0 // Weak signal
            };
            signal += rsi_signal;
            factors += 1;
        }

        // Momentum signal
        if let Some(momentum) = self.indicators.momentum {
            let momentum_signal = momentum.abs().min(0.02) / 0.02;
            signal += momentum_signal;
            factors += 1;
        }

        self.indicators.signal_strength = if factors > 0 {
            signal / factors as f64
        } else {
            0.0
        };
    }

    /// Check current position and manage exits
    async fn check_position(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        if let Some(position) = &self.current_position {
            // Query order status
            match order_wallet
                .query_trader_order(position.account_index)
                .await
            {
                Ok(trader_order) => {
                    if trader_order.order_status == OrderStatus::FILLED {
                        // Check exit conditions
                        if self.should_exit_position(position) {
                            self.close_position(order_wallet).await?;
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to query position: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Determine if current position should be closed
    fn should_exit_position(&self, position: &Position) -> bool {
        if let Some(current_price) = self.price_history.back().map(|p| p.price) {
            let price_change = (current_price - position.entry_price) / position.entry_price;

            // Check stop loss
            if let Some(stop_loss) = position.stop_loss {
                let stop_loss_change = (current_price - stop_loss) / position.entry_price;
                match position.position_type {
                    PositionType::LONG if current_price <= stop_loss => {
                        info!(
                            "Stop loss triggered for LONG position: {:.2}%",
                            stop_loss_change * 100.0
                        );
                        return true;
                    }
                    PositionType::SHORT if current_price >= stop_loss => {
                        info!(
                            "Stop loss triggered for SHORT position: {:.2}%",
                            stop_loss_change * 100.0
                        );
                        return true;
                    }
                    _ => {}
                }
            }

            // Check take profit
            if let Some(take_profit) = position.take_profit {
                match position.position_type {
                    PositionType::LONG if current_price >= take_profit => {
                        info!(
                            "Take profit triggered for LONG position: {:.2}%",
                            price_change * 100.0
                        );
                        return true;
                    }
                    PositionType::SHORT if current_price <= take_profit => {
                        info!(
                            "Take profit triggered for SHORT position: {:.2}%",
                            price_change * 100.0
                        );
                        return true;
                    }
                    _ => {}
                }
            }

            // Exit on signal reversal
            match (&position.position_type, &self.indicators.trend_direction) {
                (PositionType::LONG, TrendDirection::Bearish)
                    if self.indicators.signal_strength > 0.5 =>
                {
                    info!("Closing LONG position due to bearish signal reversal");
                    return true;
                }
                (PositionType::SHORT, TrendDirection::Bullish)
                    if self.indicators.signal_strength > 0.5 =>
                {
                    info!("Closing SHORT position due to bullish signal reversal");
                    return true;
                }
                _ => {}
            }
        }

        false
    }

    /// Execute trading decision based on analysis
    async fn execute_trading_decision(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        // Don't trade if we already have a position
        if self.current_position.is_some() {
            return Ok(());
        }

        // Check if signal is strong enough
        if self.indicators.signal_strength < self.config.min_signal_strength {
            return Ok(());
        }

        // Determine position type based on analysis
        let position_type = match self.indicators.trend_direction {
            TrendDirection::Bullish => {
                if let Some(rsi) = self.indicators.rsi {
                    if rsi < 70.0 {
                        // Not overbought
                        Some(PositionType::LONG)
                    } else {
                        None
                    }
                } else {
                    Some(PositionType::LONG)
                }
            }
            TrendDirection::Bearish => {
                if let Some(rsi) = self.indicators.rsi {
                    if rsi > 30.0 {
                        // Not oversold
                        Some(PositionType::SHORT)
                    } else {
                        None
                    }
                } else {
                    Some(PositionType::SHORT)
                }
            }
            TrendDirection::Sideways => None,
        };

        if let Some(pos_type) = position_type {
            self.open_position(order_wallet, pos_type).await?;
        }

        Ok(())
    }

    /// Open a new position
    async fn open_position(
        &mut self,
        order_wallet: &mut OrderWallet,
        position_type: PositionType,
    ) -> Result<()> {
        let account_index = self
            .trading_account
            .context("Trading account not initialized")?;

        let current_price = self
            .price_history
            .back()
            .map(|p| p.price)
            .context("No price data available")?;

        // Calculate dynamic leverage based on signal strength
        let leverage =
            (self.indicators.signal_strength * self.config.max_leverage as f64).ceil() as u64;
        let leverage = leverage.max(1).min(self.config.max_leverage);

        if self.config.paper_trading {
            info!(
                "Paper trading: Would open {:?} position at {:.2} with {}x leverage (signal: {:.3})",
                position_type, current_price, leverage, self.indicators.signal_strength
            );
            return Ok(());
        }

        info!(
            "Opening {:?} position at {:.2} with {}x leverage (signal: {:.3})",
            position_type, current_price, leverage, self.indicators.signal_strength
        );

        let request_id = order_wallet
            .open_trader_order(
                account_index,
                OrderType::MARKET,
                position_type.clone(),
                current_price as u64,
                leverage,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to open position: {}", e))?;

        // Calculate stop loss and take profit
        let stop_loss = match position_type {
            PositionType::LONG => Some(current_price * (1.0 - self.config.stop_loss_pct)),
            PositionType::SHORT => Some(current_price * (1.0 + self.config.stop_loss_pct)),
        };

        let take_profit = match position_type {
            PositionType::LONG => Some(current_price * (1.0 + self.config.take_profit_pct)),
            PositionType::SHORT => Some(current_price * (1.0 - self.config.take_profit_pct)),
        };

        let position = Position {
            account_index,
            position_type,
            entry_price: current_price,
            size: self.config.position_size,
            leverage,
            stop_loss,
            take_profit,
            request_id: request_id.clone(),
            opened_at: chrono::Utc::now(),
        };

        self.current_position = Some(position);
        self.stats.total_trades += 1;

        info!("Position opened with request ID: {}", request_id);

        Ok(())
    }

    /// Close the current position
    async fn close_position(&mut self, order_wallet: &mut OrderWallet) -> Result<()> {
        if let Some(position) = self.current_position.take() {
            if self.config.paper_trading {
                info!(
                    "Paper trading: Would close position on account {}",
                    position.account_index
                );
                return Ok(());
            }

            info!("Closing position on account {}", position.account_index);

            let close_request_id = order_wallet
                .close_trader_order(position.account_index, OrderType::MARKET, 0.0)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to close position: {}", e))?;

            // Calculate P&L (simplified)
            if let Some(current_price) = self.price_history.back().map(|p| p.price) {
                let pnl = match position.position_type {
                    PositionType::LONG => current_price - position.entry_price,
                    PositionType::SHORT => position.entry_price - current_price,
                };

                self.stats.total_pnl += pnl;

                if pnl > 0.0 {
                    self.stats.winning_trades += 1;
                    self.stats.max_profit = self.stats.max_profit.max(pnl);
                } else {
                    self.stats.losing_trades += 1;
                    self.stats.max_drawdown = self.stats.max_drawdown.min(pnl);
                }

                info!(
                    "Position closed with P&L: {:.2} (request: {})",
                    pnl, close_request_id
                );
            }
        }

        Ok(())
    }

    /// Log current trader status
    fn log_status(&mut self) {
        info!("=== Momentum Trader Status ===");
        if let Some(position) = &self.current_position {
            info!(
                "Current position: {:?} at {:.2}",
                position.position_type, position.entry_price
            );
        } else {
            info!("No current position");
        }
        info!("Signal strength: {:.3}", self.indicators.signal_strength);
        info!("Trend: {:?}", self.indicators.trend_direction);
        if let (Some(fast_ma), Some(slow_ma), Some(rsi)) = (
            self.indicators.fast_ma,
            self.indicators.slow_ma,
            self.indicators.rsi,
        ) {
            info!(
                "Fast MA: {:.2}, Slow MA: {:.2}, RSI: {:.2}",
                fast_ma, slow_ma, rsi
            );
        }
        info!("Total trades: {}", self.stats.total_trades);
        if self.stats.total_trades > 0 {
            self.stats.win_rate = self.stats.winning_trades as f64 / self.stats.total_trades as f64;
            info!("Win rate: {:.2}%", self.stats.win_rate * 100.0);
        }
        info!("Total P&L: {:.2}", self.stats.total_pnl);
        info!("===============================");
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

    info!("Starting Momentum Trading Bot");
    info!(
        "Moving averages: {} (fast), {} (slow)",
        args.fast_ma, args.slow_ma
    );
    info!("RSI period: {}", args.rsi_period);
    info!("Position size: {} sats", args.position_size);
    info!("Paper trading: {}", args.paper_trading);

    // Create momentum trader
    let mut trader = MomentumTrader::new(args);

    // Initialize OrderWallet
    let mut order_wallet = OrderWallet::new(None).context("Failed to create OrderWallet")?;

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

    if initial_balance.sats < trader.config.initial_capital {
        return Err(anyhow::anyhow!(
            "Insufficient balance. Required: {} sats, Available: {} sats",
            trader.config.initial_capital,
            initial_balance.sats
        ));
    }

    // Initialize trading account
    trader
        .initialize_account(&mut order_wallet)
        .await
        .context("Failed to initialize trading account")?;

    // Set up shutdown handler
    let shutdown_result = tokio::select! {
        result = trader.run(&mut order_wallet) => {
            result.context("Momentum trader execution failed")
        },
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
            if let Some(_) = &trader.current_position {
                trader.close_position(&mut order_wallet).await?;
            }
            Ok(())
        }
    };

    match shutdown_result {
        Ok(_) => {
            trader.log_status();
            info!("Momentum trader finished successfully");
        }
        Err(e) => error!("Momentum trader error: {}", e),
    }

    Ok(())
}
