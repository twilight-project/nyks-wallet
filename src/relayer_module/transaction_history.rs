//! Transaction history and audit log types.
//!
//! Provides structured types for order and transfer history entries,
//! plus filter structs for querying historical data.

use serde::{Deserialize, Serialize};

/// An entry in the order history audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderHistoryEntry {
    pub account_index: u64,
    pub request_id: String,
    pub action: String,
    pub order_type: String,
    pub position_type: Option<String>,
    pub amount: u64,
    pub price: Option<f64>,
    pub leverage: Option<u64>,
    pub pnl: Option<f64>,
    pub status: String,
    pub tx_hash: Option<String>,
    pub created_at: String,
}

/// An entry in the transfer history audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferHistoryEntry {
    pub direction: String,
    pub from_index: Option<u64>,
    pub to_index: Option<u64>,
    pub amount: u64,
    pub tx_hash: Option<String>,
    pub created_at: String,
}

/// Filter for querying order history.
#[derive(Debug, Clone, Default)]
pub struct OrderHistoryFilter {
    /// Filter by specific account index.
    pub account_index: Option<u64>,
    /// Maximum number of results.
    pub limit: Option<i64>,
    /// Offset for pagination.
    pub offset: Option<i64>,
}

/// Filter for querying transfer history.
#[derive(Debug, Clone, Default)]
pub struct TransferHistoryFilter {
    /// Maximum number of results.
    pub limit: Option<i64>,
    /// Offset for pagination.
    pub offset: Option<i64>,
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
impl OrderHistoryEntry {
    pub fn from_db(row: &crate::database::models::DbOrderHistory) -> Self {
        Self {
            account_index: row.account_index as u64,
            request_id: row.request_id.clone(),
            action: row.action.clone(),
            order_type: row.order_type.clone(),
            position_type: row.position_type.clone(),
            amount: row.amount as u64,
            price: row.price,
            leverage: row.leverage.map(|l| l as u64),
            pnl: row.pnl,
            status: row.status.clone(),
            tx_hash: row.tx_hash.clone(),
            created_at: row.created_at.to_string(),
        }
    }
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
impl TransferHistoryEntry {
    pub fn from_db(row: &crate::database::models::DbTransferHistory) -> Self {
        Self {
            direction: row.direction.clone(),
            from_index: row.from_index.map(|i| i as u64),
            to_index: row.to_index.map(|i| i as u64),
            amount: row.amount as u64,
            tx_hash: row.tx_hash.clone(),
            created_at: row.created_at.to_string(),
        }
    }
}
