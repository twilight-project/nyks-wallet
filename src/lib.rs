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
//! use nyks_wallet::relayer_module::relayer_api::RelayerJsonRpcClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = RelayerJsonRpcClient::new("http://0.0.0.0:8088/api")?;
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
//! The following environment variables can be used to configure endpoints and behavior:
//!
//! | Variable | Description | Default |
//! |----------|-------------|---------|
//! | `NYKS_RPC_BASE_URL` | Cosmos RPC endpoint | `http://0.0.0.0:26657` |
//! | `NYKS_LCD_BASE_URL` | Cosmos LCD endpoint | `http://0.0.0.0:1317` |
//! | `RELAYER_PROGRAM_JSON_PATH` | Path to relayer program config | `./relayerprogram.json` |
//! | `PUBLIC_API_RPC_SERVER_URL` | Public relayer API endpoint | Various |
//! | `RELAYER_RPC_SERVER_URL` | Relayer trading API endpoint | Various |
//! | `FAUCET_BASE_URL` | Testnet faucet endpoint | Various |
//! | `NYKS_WALLET_PASSPHRASE` | Database encryption passphrase | None |
//! | `RUST_LOG` | Logging level | `info` |
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
//! and the `OrderWalletUpdated.md` guide in the repository.

pub mod nyks_rpc;
pub mod wallet;
pub use wallet::*;
pub mod config;
pub mod error;
pub mod test;
pub mod zkos_accounts;
#[macro_use]
extern crate lazy_static;
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
pub use nyks::module::zkos::MsgMintBurnTradingBtc;
pub use nyks::module::zkos::MsgTransferTx;

// -------------------------------------------------------------
// Optional validator-wallet feature
// -------------------------------------------------------------
#[cfg(feature = "validator-wallet")]
pub mod validator_wallet;

#[cfg(feature = "validator-wallet")]
pub use validator_wallet::*;

pub mod relayer_module;

// Database module (optional, based on features)
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
pub mod database;

// Security module for secure password and wallet management
pub mod security;

#[cfg(all(feature = "sqlite", feature = "postgresql"))]
#[deprecated(note = "Both 'sqlite' and 'postgresql' enabled; defaulting to Sqlite.")]
const _SQLITE_WINS_WHEN_BOTH: () = ();

#[cfg(all(feature = "sqlite", feature = "postgresql"))]
#[allow(deprecated)]
const _: () = {
    let _ = _SQLITE_WINS_WHEN_BOTH;
};
