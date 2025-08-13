//! Relayer module for trading operations on the Twilight Protocol.
//!
//! This module provides comprehensive functionality for interacting with the Twilight relayer,
//! enabling privacy-preserving trading operations including leveraged derivatives and lending.
//!
//! ## Module Organization
//!
//! - [`order_wallet`]: High-level trading interface that orchestrates the complete trading workflow
//! - [`relayer_api`]: Low-level JSON-RPC client for direct relayer endpoint access
//! - [`relayer_order`]: Order creation and execution primitives for trader and lend operations
//! - [`relayer_types`]: Type definitions and data structures for relayer communication
//! - [`utils`]: Utility functions for transaction building, retry logic, and chain communication
//!
//! ## Usage Patterns
//!
//! ### High-Level Trading (Recommended)
//!
//! Use [`order_wallet::OrderWallet`] for most trading operations:
//!
//! ```no_run
//! use nyks_wallet::relayer_module::order_wallet::OrderWallet;
//! use twilight_client_sdk::relayer_types::{OrderType, PositionType};
//!
//! # async fn example() -> Result<(), String> {
//! let mut order_wallet = OrderWallet::new(None).map_err(|e| e.to_string())?;
//! let (_, account_index) = order_wallet.funding_to_trading(10_000).await?;
//!
//! // Open a position
//! let request_id = order_wallet
//!     .open_trader_order(account_index, OrderType::MARKET, PositionType::LONG, 50_000, 10)
//!     .await?;
//!
//! // Query and close
//! let order = order_wallet.query_trader_order(account_index).await?;
//! let close_id = order_wallet.close_trader_order(account_index, OrderType::MARKET, 0.0).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Low-Level API Access
//!
//! Use [`relayer_api::RelayerJsonRpcClient`] for direct endpoint access:
//!
//! ```no_run
//! use nyks_wallet::relayer_module::relayer_api::RelayerJsonRpcClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = RelayerJsonRpcClient::new("http://0.0.0.0:8088/api")?;
//!
//! // Market data
//! let price = client.btc_usd_price().await?;
//! let order_book = client.open_limit_orders().await?;
//! let funding_rate = client.get_funding_rate().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Custom Order Building
//!
//! Use [`relayer_order`] functions for custom order creation:
//!
//! ```no_run
//! use nyks_wallet::relayer_module::relayer_order::create_trader_order;
//! use twilight_client_sdk::relayer_types::{OrderType, PositionType};
//!
//! # async fn example() -> Result<(), String> {
//! # let secret_key = todo!();
//! # let r_scalar = todo!();
//! # let initial_margin = 1000u64;
//! # let leverage = 10u64;
//! # let entry_price = 50000u64;
//! # let position_value = 10000u64;
//! # let position_size = 500000000u64;
//! # let relayer_program_path = "path/to/relayer.json";
//! # let account_address = "account_address".to_string();
//! # let relayer_client = todo!();
//! let request_id = create_trader_order(
//!     secret_key,
//!     r_scalar,
//!     initial_margin,
//!     PositionType::LONG,
//!     OrderType::MARKET,
//!     leverage,
//!     entry_price,
//!     position_value,
//!     position_size,
//!     relayer_program_path,
//!     account_address,
//!     &relayer_client,
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Data Flow
//!
//! ```text
//! OrderWallet
//!     ↓ (uses)
//! relayer_order (create_trader_order, create_lend_order, etc.)
//!     ↓ (submits via)
//! RelayerJsonRpcClient (submit_trade_order, settle_trade_order, etc.)
//!     ↓ (communicates with)
//! Twilight Relayer Service
//! ```
//!
//! ## Error Handling
//!
//! The module includes robust error handling with automatic retries for:
//! - UTXO detail fetching ([`fetch_utxo_details_with_retry`])
//! - Transaction hash querying ([`fetch_tx_hash_with_retry`])
//! - Network communication failures
//!
//! See [`utils`] for retry configuration and helper functions.

pub mod order_wallet;
pub mod relayer_api;
pub mod relayer_order;
pub mod relayer_types;
mod utils;
pub use utils::*;
