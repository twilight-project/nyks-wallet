//! # nyks-wallet
//!
//! A comprehensive SDK for interacting with the Twilight Protocol blockchain, providing wallet
//! management, ZkOS privacy-preserving accounts, and relayer-based trading operations.
//!
//! ## Features
//!
//! - **Wallet Management**: Create, import, and manage Cosmos-based wallets with BTC bridge support
//! - **ZkOS Integration**: Privacy-preserving account system with zero-knowledge proofs
//! - **Relayer Trading**: High-level interface for leveraged derivatives and lending operations
//! - **Database Persistence**: Optional encrypted storage of wallet data and trading state
//! - **Security**: Secure key management, password handling, and mnemonic generation
//!
//! ## Quick Start
//!
//! ### Basic Wallet Operations
//!
//! ```no_run
//! use nyks_wallet::Wallet;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a new wallet (mnemonic will be printed to TTY)
//!     let mut wallet = Wallet::new(None)?;
//!     
//!     // Check balance
//!     let balance = wallet.update_balance().await?;
//!     println!("Balance: {} sats", balance.sats);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Trading with OrderWallet
//!
//! ```no_run
//! use nyks_wallet::relayer_module::order_wallet::OrderWallet;
//! use twilight_client_sdk::relayer_types::{OrderType, PositionType};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), String> {
//!     env_logger::init();
//!     
//!     // Create OrderWallet with default configuration
//!     let mut order_wallet = OrderWallet::new(None).map_err(|e| e.to_string())?;
//!     
//!     // Fund a ZkOS trading account
//!     let (tx_result, account_index) = order_wallet.funding_to_trading(10_000).await?;
//!     
//!     // Open a leveraged position
//!     let request_id = order_wallet
//!         .open_trader_order(
//!             account_index,
//!             OrderType::MARKET,
//!             PositionType::LONG,
//!             50_000, // entry price
//!             10,     // 10x leverage
//!         )
//!         .await?;
//!     
//!     // Query order status
//!     let order = order_wallet.query_trader_order(account_index).await?;
//!     println!("Order status: {:?}", order.order_status);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Database Persistence
//!
//! Enable the `sqlite` or `postgresql` feature to use database persistence:
//!
//! ```toml
//! [dependencies]
//! nyks-wallet = { path = ".", features = ["sqlite"] }
//! ```
//!
//! ```no_run
//! # #[cfg(feature = "sqlite")]
//! use nyks_wallet::relayer_module::order_wallet::OrderWallet;
//! use secrecy::SecretString;
//!
//! # #[cfg(feature = "sqlite")]
//! fn main() -> Result<(), String> {
//!     let mut order_wallet = OrderWallet::new(None).map_err(|e| e.to_string())?;
//!     
//!     // Enable database persistence with custom wallet ID
//!     let order_wallet = order_wallet.with_db(
//!         Some(SecretString::new("my_secure_password".into())),
//!         Some("my_trading_wallet".into())
//!     )?;
//!     
//!     // Save configuration to database
//!     order_wallet.save_order_wallet_to_db()?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Direct Relayer API Access
//!
//! For lower-level access to relayer endpoints:
//!
//! ```no_run
//! use nyks_wallet::config::RELAYER_API_RPC_SERVER_URL;
//! use nyks_wallet::relayer_module::relayer_api::RelayerJsonRpcClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = RelayerJsonRpcClient::new(&RELAYER_API_RPC_SERVER_URL)?;
//!
//!     // Get market data
//!     let price = client.btc_usd_price().await?;
//!     let order_book = client.open_limit_orders().await?;
//!     let funding_rate = client.get_funding_rate().await?;
//!
//!     println!("BTC/USD: ${}, Funding: {}%", price.price, funding_rate.rate);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Environment Configuration
//!
//! Endpoint defaults are selected by `NETWORK_TYPE` (mainnet vs testnet); override any
//! variable explicitly to point at a local full-node.
//!
//! | Variable | Description | Default |
//! |----------|-------------|---------|
//! | `NETWORK_TYPE` | `mainnet` or `testnet`; selects defaults for other endpoints | `mainnet` |
//! | `BTC_NETWORK_TYPE` | BTC network for Esplora endpoints (nyks chain only supports BTC `mainnet`) | `mainnet` |
//! | `CHAIN_ID` | Target chain ID | `nyks` |
//! | `NYKS_RPC_BASE_URL` | Cosmos / Tendermint RPC | mainnet: `https://rpc.twilight.org`; testnet: `https://rpc.twilight.rest` |
//! | `NYKS_LCD_BASE_URL` | Cosmos LCD REST | mainnet: `https://lcd.twilight.org`; testnet: `https://lcd.twilight.rest` |
//! | `RELAYER_API_RPC_SERVER_URL` | Relayer JSON-RPC API | mainnet: `https://api.ephemeral.fi/api`; testnet: `https://relayer.twilight.rest/api` |
//! | `ZKOS_SERVER_URL` | ZkOS JSON-RPC | mainnet: `https://zkserver.twilight.org`; testnet: `https://nykschain.twilight.rest/zkos` |
//! | `FAUCET_BASE_URL` | Testnet faucet | testnet: `https://faucet-rpc.twilight.rest` (empty on mainnet) |
//! | `TWILIGHT_INDEXER_URL` | Twilight indexer | mainnet: `https://indexer.twilight.org`; testnet: `https://indexer.twilight.rest` |
//! | `BTC_ESPLORA_PRIMARY_URL` | Primary Esplora API (driven by `BTC_NETWORK_TYPE`) | `https://blockstream.info/api` (mainnet) |
//! | `BTC_ESPLORA_FALLBACK_URL` | Fallback Esplora API (driven by `BTC_NETWORK_TYPE`) | `https://mempool.space/api` (mainnet) |
//! | `RELAYER_PROGRAM_JSON_PATH` | Path to relayer program JSON | `./relayerprogram.json` |
//! | `VALIDATOR_WALLET_PATH` | Validator mnemonic file (`validator-wallet` feature) | `validator.mnemonic` |
//! | `NYKS_WALLET_PASSPHRASE` | DB encryption passphrase | – (prompt) |
//! | `WALLET_ID` | DB wallet ID (defaults to Twilight address) | – |
//! | `DATABASE_URL_SQLITE` | SQLite path (`sqlite` feature) | `./wallet_data.db` |
//! | `DATABASE_URL_POSTGRESQL` | PostgreSQL DSN (`postgresql` feature) | – |
//! | `RUST_LOG` | Logging level | – |
//!
//! ## Feature Flags
//!
//! - `sqlite`: Enable SQLite database persistence
//! - `postgresql`: Enable PostgreSQL database persistence  
//! - `validator-wallet`: Enable validator-specific functionality
//!
//! **Note**: If both `sqlite` and `postgresql` are enabled, SQLite takes precedence.
//!
//! ## Security Considerations
//!
//! - Mnemonics are printed once to TTY and immediately cleared from memory
//! - Database encryption uses AES-256-GCM with PBKDF2 key derivation
//! - Private keys are never stored in plaintext
//! - Use environment variables or secure prompts for passwords
//!
//! ## Module Overview
//!
//! - [`wallet`]: Core wallet functionality and blockchain interactions
//! - [`relayer_module`]: Trading operations via the Twilight relayer
//!   - [`relayer_module::order_wallet`]: High-level trading interface with OrderWallet
//!   - [`relayer_module::relayer_api`]: Low-level JSON-RPC client for relayer endpoints
//! - [`zkos_accounts`]: Privacy-preserving account management
//! - [`database`]: Optional persistence layer (requires feature flags)
//! - [`security`]: Secure password and key management utilities
//! - [`config`]: Configuration management and endpoint settings
//! - [`error`]: Error types and handling
//!
//! For detailed usage examples and API documentation, see the individual module documentation
//! and the [`OrderWallet.md`](../../OrderWallet.md) guide in the repository.

pub mod nyks_rpc;
pub mod wallet;
pub use wallet::*;
pub mod config;
pub mod error;
pub mod test;
// ----------------------------------------------------------------------------
// Generated protobuf module (prost-build)
// ----------------------------------------------------------------------------

pub mod nyks {
    pub mod module {
        pub mod bridge {
            include!(concat!(env!("OUT_DIR"), "/twilightproject.nyks.bridge.rs"));
        }
        pub mod zkos {
            include!(concat!(env!("OUT_DIR"), "/twilightproject.nyks.zkos.rs"));
        }
    }
}

pub use nyks::module::bridge::MsgRegisterBtcDepositAddress;
pub use nyks::module::bridge::MsgWithdrawBtcRequest;
pub use nyks::module::zkos::MsgMintBurnTradingBtc;
pub use nyks::module::zkos::MsgTransferTx;

// -------------------------------------------------------------
// Optional validator-wallet feature
// -------------------------------------------------------------
#[cfg(feature = "validator-wallet")]
pub mod validator_wallet;

#[cfg(feature = "validator-wallet")]
pub use validator_wallet::*;

// -------------------------------------------------------------
// Optional order-wallet feature
// -------------------------------------------------------------
#[cfg(feature = "order-wallet")]
pub mod relayer_module;
#[cfg(feature = "order-wallet")]
pub mod zkos_accounts;

// Database module (optional, based on features)
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
pub mod database;

// Security module for secure password and wallet management
// #[cfg(feature = "order-wallet")]
pub mod security;

#[cfg(all(feature = "sqlite", feature = "postgresql"))]
#[deprecated(note = "Both 'sqlite' and 'postgresql' enabled; defaulting to Sqlite.")]
const _SQLITE_WINS_WHEN_BOTH: () = ();

#[cfg(all(feature = "sqlite", feature = "postgresql"))]
#[allow(deprecated)]
const _: () = {
    let _ = _SQLITE_WINS_WHEN_BOTH;
};
