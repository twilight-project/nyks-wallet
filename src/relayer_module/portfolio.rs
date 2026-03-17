//! Portfolio and position tracking module for the OrderWallet.
//!
//! Provides aggregate views of portfolio state, per-position PnL calculations,
//! liquidation price monitoring, and risk metrics for both trader and lend positions.

use serde::Serialize;
use twilight_client_sdk::{
    relayer_types::{LendOrder, OrderStatus, PositionType, TraderOrder},
    zkvm::IOType,
};

use super::order_wallet::AccountIndex;

/// Summary of a single trader position with PnL and risk metrics.
#[derive(Debug, Clone, Serialize)]
pub struct PositionSummary {
    pub account_index: AccountIndex,
    pub position_type: PositionType,
    pub order_status: OrderStatus,
    pub entry_price: f64,
    pub current_price: f64,
    pub initial_margin: f64,
    pub available_margin: f64,
    pub leverage: f64,
    pub position_size: f64,
    pub unrealized_pnl: f64,
    pub liquidation_price: f64,
    pub bankruptcy_price: f64,
    pub margin_ratio: f64,
    pub fee_filled: f64,
    pub fee_settled: f64,
}

impl PositionSummary {
    /// Build a position summary from a queried TraderOrder and the current market price.
    pub fn from_trader_order(
        account_index: AccountIndex,
        order: &TraderOrder,
        current_price: f64,
    ) -> Self {
        let margin_ratio = if order.initial_margin > 0.0 {
            order.available_margin / order.initial_margin
        } else {
            0.0
        };

        Self {
            account_index,
            position_type: order.position_type.clone(),
            order_status: order.order_status.clone(),
            entry_price: order.entryprice,
            current_price,
            initial_margin: order.initial_margin,
            available_margin: order.available_margin,
            leverage: order.leverage,
            position_size: order.positionsize,
            unrealized_pnl: order.unrealized_pnl,
            liquidation_price: order.liquidation_price,
            bankruptcy_price: order.bankruptcy_price,
            margin_ratio,
            fee_filled: order.fee_filled,
            fee_settled: order.fee_settled,
        }
    }
}

/// Summary of a single lend position.
#[derive(Debug, Clone, Serialize)]
pub struct LendPositionSummary {
    pub account_index: AccountIndex,
    pub order_status: OrderStatus,
    pub deposit: f64,
    pub current_value: f64,
    pub pool_share: f64,
    pub payment: f64,
    pub pnl: f64,
}

impl LendPositionSummary {
    /// Build a lend position summary from a queried LendOrder.
    pub fn from_lend_order(account_index: AccountIndex, order: &LendOrder) -> Self {
        let pnl = order.new_lend_state_amount - order.deposit;
        Self {
            account_index,
            order_status: order.order_status.clone(),
            deposit: order.deposit,
            current_value: order.new_lend_state_amount,
            pool_share: order.npoolshare,
            payment: order.payment,
            pnl,
        }
    }
}

/// Aggregate portfolio view across all ZkOS accounts.
#[derive(Debug, Clone, Serialize)]
pub struct Portfolio {
    /// On-chain wallet balance (funding layer).
    pub wallet_balance_sats: u64,
    /// Sum of all ZkOS Coin account balances (idle trading capital).
    pub total_trading_balance: u64,
    /// Sum of initial margins across open trader positions.
    pub total_margin_used: f64,
    /// Sum of unrealized PnL across open trader positions.
    pub unrealized_pnl: f64,
    /// Total deposits in active lend positions.
    pub total_lend_deposits: f64,
    /// Total current value of lend positions.
    pub total_lend_value: f64,
    /// Lend PnL (current value - deposits).
    pub lend_pnl: f64,
    /// Open trader positions.
    pub trader_positions: Vec<PositionSummary>,
    /// Active lend positions.
    pub lend_positions: Vec<LendPositionSummary>,
    /// Number of ZkOS accounts total.
    pub total_accounts: usize,
    /// Number of on-chain accounts.
    pub on_chain_accounts: usize,
    /// Margin utilization: total_margin_used / (total_trading_balance + total_margin_used).
    pub margin_utilization: f64,
}

impl Portfolio {
    /// Build a portfolio from its components.
    pub fn build(
        wallet_balance_sats: u64,
        total_trading_balance: u64,
        trader_positions: Vec<PositionSummary>,
        lend_positions: Vec<LendPositionSummary>,
        total_accounts: usize,
        on_chain_accounts: usize,
    ) -> Self {
        let total_margin_used: f64 = trader_positions.iter().map(|p| p.initial_margin).sum();
        let unrealized_pnl: f64 = trader_positions.iter().map(|p| p.unrealized_pnl).sum();
        let total_lend_deposits: f64 = lend_positions.iter().map(|p| p.deposit).sum();
        let total_lend_value: f64 = lend_positions.iter().map(|p| p.current_value).sum();
        let lend_pnl = total_lend_value - total_lend_deposits;

        let denominator = total_trading_balance as f64 + total_margin_used;
        let margin_utilization = if denominator > 0.0 {
            total_margin_used / denominator
        } else {
            0.0
        };

        Self {
            wallet_balance_sats,
            total_trading_balance,
            total_margin_used,
            unrealized_pnl,
            total_lend_deposits,
            total_lend_value,
            lend_pnl,
            trader_positions,
            lend_positions,
            total_accounts,
            on_chain_accounts,
            margin_utilization,
        }
    }
}

/// Liquidation risk info for a single position.
#[derive(Debug, Clone, Serialize)]
pub struct LiquidationRisk {
    pub account_index: AccountIndex,
    pub position_type: PositionType,
    pub liquidation_price: f64,
    pub current_price: f64,
    /// Distance to liquidation as a percentage of current price.
    /// Positive = safe, negative = already past liquidation.
    pub distance_pct: f64,
    pub margin_ratio: f64,
}

/// Per-account balance snapshot for quick overview.
#[derive(Debug, Clone, Serialize)]
pub struct AccountBalanceInfo {
    pub account_index: AccountIndex,
    pub balance: u64,
    pub io_type: IOType,
    pub on_chain: bool,
}
