use clap::{Parser, Subcommand};
use log::error;
use nyks_wallet::relayer_module::order_wallet::OrderWallet;
use nyks_wallet::relayer_module::relayer_types::OrderStatus;
use secrecy::{ExposeSecret, SecretString};

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

/// Twilight Relayer CLI — manage wallets and orders from the command line.
#[derive(Parser)]
#[command(name = "relayer-cli", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output results as JSON instead of formatted tables (useful for scripting)
    #[arg(long, global = true, default_value_t = false)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Wallet management commands
    #[command(subcommand)]
    Wallet(WalletCmd),

    /// ZkOS account management (fund, withdraw, transfer, split)
    #[command(subcommand)]
    Zkaccount(ZkaccountCmd),

    /// Order and trading commands
    #[command(subcommand)]
    Order(OrderCmd),

    /// Market data queries
    #[command(subcommand)]
    Market(MarketCmd),

    /// Transaction history queries (requires DB)
    #[command(subcommand)]
    History(HistoryCmd),

    /// Portfolio and position tracking
    #[command(subcommand)]
    Portfolio(PortfolioCmd),

    /// Show help for a command group (e.g. `help wallet`)
    Help {
        /// Command group to get help for (wallet, zkaccount, order, market, history, portfolio)
        command: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Wallet sub-commands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
enum WalletCmd {
    /// Create a new wallet (always persisted to database)
    Create {
        /// Wallet ID for database storage (defaults to the Twilight address if omitted)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,

        /// Optional BTC SegWit address (bc1q... or bc1p...) to use instead of generating a random one
        #[arg(long)]
        btc_address: Option<String>,
    },

    /// Import a wallet from a mnemonic phrase (always persisted to database)
    Import {
        /// The BIP-39 mnemonic phrase (24 words). If omitted, prompts securely via TTY.
        #[arg(long)]
        mnemonic: Option<String>,

        /// Wallet ID for database storage (defaults to the Twilight address if omitted)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,

        /// Optional BTC SegWit address (bc1q... or bc1p...) to use instead of deriving from mnemonic
        #[arg(long)]
        btc_address: Option<String>,
    },

    /// Load a wallet from the database
    Load {
        /// Wallet ID stored in the database
        #[arg(long)]
        wallet_id: String,

        /// Database encryption password
        #[arg(long)]
        password: Option<String>,

        /// Optional database URL override
        #[arg(long)]
        db_url: Option<String>,
    },

    /// Show wallet balance and account info
    Balance {
        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// List all wallets stored in the database
    List {
        /// Optional database URL override
        #[arg(long)]
        db_url: Option<String>,
    },

    /// Export wallet to a JSON file
    Export {
        /// Output file path
        #[arg(long, default_value = "wallet.json")]
        output: String,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// List all ZkOS accounts for a wallet
    Accounts {
        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,

        /// Only show on-chain accounts (hide accounts where on_chain is false)
        #[arg(long, default_value_t = false)]
        on_chain_only: bool,
    },

    /// Export a full database backup to a JSON file (requires DB)
    Backup {
        /// Output file path for the backup
        #[arg(long, default_value = "wallet_backup.json")]
        output: String,

        /// Wallet ID to back up
        #[arg(long)]
        wallet_id: String,

        /// Database encryption password
        #[arg(long)]
        password: Option<String>,
    },

    /// Restore a wallet from a backup JSON file (requires DB)
    Restore {
        /// Input backup file path
        #[arg(long)]
        input: String,

        /// Wallet ID to restore into
        #[arg(long)]
        wallet_id: String,

        /// Database encryption password
        #[arg(long)]
        password: Option<String>,

        /// Force restore even if backup wallet_id doesn't match
        #[arg(long, default_value_t = false)]
        force: bool,
    },

    /// Sync the nonce/sequence manager from chain state
    SyncNonce {
        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Unlock: prompt for wallet ID and password, then cache both for this terminal session.
    /// Subsequent commands will use the cached values automatically.
    /// The cache is invalidated when the terminal (shell) is closed.
    Unlock {
        /// Wallet ID to cache (prompts interactively if omitted)
        #[arg(long)]
        wallet_id: Option<String>,
        /// Overwrite an existing session without error
        #[arg(long, default_value_t = false)]
        force: bool,
    },

    /// Lock: clear the cached session password immediately.
    Lock,

    /// Change the database encryption password for a wallet.
    /// Always prompts for both old and new passwords via TTY (ignores session cache and env var).
    ChangePassword {
        /// Wallet ID to change password for
        #[arg(long)]
        wallet_id: Option<String>,
    },

    /// Show wallet info (address, BTC address, chain_id, accounts, nonce) without chain calls
    Info {
        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Update the BTC deposit address for a wallet.
    UpdateBtcAddress {
        /// New BTC address
        #[arg(long)]
        btc_address: String,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// ZkOS account sub-commands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
enum ZkaccountCmd {
    /// Fund a new ZkOS trading account from the on-chain wallet.
    /// Provide exactly one of --amount (sats), --amount-mbtc (mBTC), or --amount-btc (BTC).
    Fund {
        /// Amount in satoshis
        #[arg(long)]
        amount: Option<u64>,

        /// Amount in milli-BTC (1 mBTC = 100,000 sats)
        #[arg(long)]
        amount_mbtc: Option<f64>,

        /// Amount in BTC (1 BTC = 100,000,000 sats)
        #[arg(long)]
        amount_btc: Option<f64>,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Withdraw from a ZkOS trading account back to the on-chain wallet
    Withdraw {
        /// ZkOS account index to withdraw from
        #[arg(long)]
        account_index: u64,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Transfer between ZkOS trading accounts
    Transfer {
        /// Source account index
        #[arg(long)]
        from: u64,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Split a ZkOS trading account into multiple new accounts.
    /// Provide exactly one of --balances (sats), --balances-mbtc, or --balances-btc.
    Split {
        /// Source account index
        #[arg(long)]
        from: u64,

        /// Comma-separated balances in satoshis (e.g. "1000,2000,3000")
        #[arg(long)]
        balances: Option<String>,

        /// Comma-separated balances in milli-BTC (e.g. "0.01,0.02,0.03")
        #[arg(long)]
        balances_mbtc: Option<String>,

        /// Comma-separated balances in BTC (e.g. "0.00001,0.00002")
        #[arg(long)]
        balances_btc: Option<String>,

        /// Load wallet from DB by wallet ID
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password
        #[arg(long)]
        password: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Order sub-commands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
enum OrderCmd {
    /// Open a trader order (leveraged derivative)
    OpenTrade {
        /// ZkOS account index to use
        #[arg(long)]
        account_index: u64,

        /// Order type: MARKET or LIMIT
        #[arg(long, default_value = "MARKET")]
        order_type: String,

        /// Position side: LONG or SHORT
        #[arg(long)]
        side: String,

        /// Entry price in USD (integer)
        #[arg(long)]
        entry_price: u64,

        /// Leverage multiplier (1-50)
        #[arg(long)]
        leverage: u64,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Close a trader order
    CloseTrade {
        /// ZkOS account index
        #[arg(long)]
        account_index: u64,

        /// Order type: MARKET or LIMIT
        #[arg(long, default_value = "MARKET")]
        order_type: String,

        /// Execution price (0.0 for market orders)
        #[arg(long, default_value_t = 0.0)]
        execution_price: f64,

        /// Stop-loss price (optional, enables SLTP close)
        #[arg(long)]
        stop_loss: Option<f64>,

        /// Take-profit price (optional, enables SLTP close)
        #[arg(long)]
        take_profit: Option<f64>,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Cancel a pending trader order
    CancelTrade {
        /// ZkOS account index
        #[arg(long)]
        account_index: u64,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Query trader order status
    QueryTrade {
        /// ZkOS account index
        #[arg(long)]
        account_index: u64,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Unlock a settled trader order (reclaim account after SLTP settlement)
    UnlockTrade {
        /// ZkOS account index
        #[arg(long)]
        account_index: u64,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Open a lend order
    OpenLend {
        /// ZkOS account index to lend from
        #[arg(long)]
        account_index: u64,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Close a lend order
    CloseLend {
        /// ZkOS account index
        #[arg(long)]
        account_index: u64,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Query lend order status
    QueryLend {
        /// ZkOS account index
        #[arg(long)]
        account_index: u64,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Query historical trader orders for an account (from relayer, not local DB)
    HistoryTrade {
        /// ZkOS account index
        #[arg(long)]
        account_index: u64,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Query historical lend orders for an account (from relayer, not local DB)
    HistoryLend {
        /// ZkOS account index
        #[arg(long)]
        account_index: u64,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Query funding payment history for a position
    FundingHistory {
        /// ZkOS account index
        #[arg(long)]
        account_index: u64,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Query your trading activity summary (fills, settles, liquidations)
    AccountSummary {
        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,

        /// Start date filter (RFC3339 or YYYY-MM-DD)
        #[arg(long)]
        from: Option<String>,

        /// End date filter (RFC3339 or YYYY-MM-DD)
        #[arg(long)]
        to: Option<String>,

        /// Since date filter (RFC3339 or YYYY-MM-DD, alternative to from/to range)
        #[arg(long)]
        since: Option<String>,
    },

    /// Look up on-chain transaction hashes by request ID or account ID
    TxHashes {
        /// Lookup mode: "request" (by request ID) or "account" (by account address)
        #[arg(long, default_value = "request")]
        by: String,

        /// The request ID or account address to look up
        #[arg(long)]
        id: String,

        /// Filter by order status (e.g. FILLED, SETTLED, PENDING)
        #[arg(long)]
        status: Option<String>,

        /// Maximum number of results
        #[arg(long)]
        limit: Option<i64>,

        /// Offset for pagination
        #[arg(long)]
        offset: Option<i64>,
    },
}

// ---------------------------------------------------------------------------
// History sub-commands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
enum HistoryCmd {
    /// Show order history (open, close, cancel events)
    Orders {
        /// Wallet ID (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,

        /// Filter by account index
        #[arg(long)]
        account_index: Option<u64>,

        /// Maximum number of results
        #[arg(long, default_value_t = 50)]
        limit: i64,

        /// Offset for pagination
        #[arg(long, default_value_t = 0)]
        offset: i64,
    },

    /// Show transfer history (fund, withdraw, transfer events)
    Transfers {
        /// Wallet ID (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,

        /// Maximum number of results
        #[arg(long, default_value_t = 50)]
        limit: i64,

        /// Offset for pagination
        #[arg(long, default_value_t = 0)]
        offset: i64,
    },
}

// ---------------------------------------------------------------------------
// Portfolio sub-commands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
enum PortfolioCmd {
    /// Show full portfolio summary (balances, positions, PnL)
    Summary {
        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Show per-account balance breakdown
    Balances {
        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,

        /// Display unit: sats (default), mbtc, or btc
        #[arg(long, default_value = "sats")]
        unit: String,
    },

    /// Show liquidation risk for open positions
    Risks {
        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Market sub-commands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
enum MarketCmd {
    /// Get current BTC/USD price
    Price,

    /// Get the order book (open limit orders)
    Orderbook,

    /// Get current funding rate
    FundingRate,

    /// Get current fee rate
    FeeRate,

    /// Get recent trade orders
    RecentTrades,

    /// Get position size summary
    PositionSize,

    /// Get lend pool info
    LendPool,

    /// Get current pool share value
    PoolShareValue,

    /// Get last 24-hour annualized percentage yield (APY)
    LastDayApy,

    /// Get open interest (long/short exposure)
    OpenInterest,

    /// Get comprehensive market risk statistics
    MarketStats,

    /// Get relayer server time
    ServerTime,

    /// Query historical BTC/USD prices over a date range
    HistoryPrice {
        /// Start date (RFC3339 or YYYY-MM-DD)
        #[arg(long)]
        from: String,

        /// End date (RFC3339 or YYYY-MM-DD)
        #[arg(long)]
        to: String,

        /// Maximum number of results
        #[arg(long, default_value_t = 50)]
        limit: i64,

        /// Offset for pagination
        #[arg(long, default_value_t = 0)]
        offset: i64,
    },

    /// Query OHLCV candlestick data
    Candles {
        /// Candle interval: 1m, 5m, 15m, 30m, 1h, 4h, 8h, 12h, 1d
        #[arg(long, default_value = "1h")]
        interval: String,

        /// Start date (RFC3339 or YYYY-MM-DD)
        #[arg(long)]
        since: String,

        /// Maximum number of results
        #[arg(long, default_value_t = 50)]
        limit: i64,

        /// Offset for pagination
        #[arg(long, default_value_t = 0)]
        offset: i64,
    },

    /// Query historical funding rates
    HistoryFunding {
        /// Start date (RFC3339 or YYYY-MM-DD)
        #[arg(long)]
        from: String,

        /// End date (RFC3339 or YYYY-MM-DD)
        #[arg(long)]
        to: String,

        /// Maximum number of results
        #[arg(long, default_value_t = 50)]
        limit: i64,

        /// Offset for pagination
        #[arg(long, default_value_t = 0)]
        offset: i64,
    },

    /// Query historical fee rates
    HistoryFees {
        /// Start date (RFC3339 or YYYY-MM-DD)
        #[arg(long)]
        from: String,

        /// End date (RFC3339 or YYYY-MM-DD)
        #[arg(long)]
        to: String,

        /// Maximum number of results
        #[arg(long, default_value_t = 50)]
        limit: i64,

        /// Offset for pagination
        #[arg(long, default_value_t = 0)]
        offset: i64,
    },

    /// Query APY chart data for the lend pool
    ApyChart {
        /// Time range (e.g. "7d", "30d", "1y")
        #[arg(long, default_value = "7d")]
        range: String,

        /// Step/granularity (e.g. "1h", "1d")
        #[arg(long)]
        step: Option<String>,

        /// Lookback period for rolling average
        #[arg(long)]
        lookback: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_order_type(s: &str) -> Result<twilight_client_sdk::relayer_types::OrderType, String> {
    match s.to_uppercase().as_str() {
        "MARKET" => Ok(twilight_client_sdk::relayer_types::OrderType::MARKET),
        "LIMIT" => Ok(twilight_client_sdk::relayer_types::OrderType::LIMIT),
        "SLTP" => Ok(twilight_client_sdk::relayer_types::OrderType::SLTP),
        other => Err(format!(
            "Unknown order type: {other}. Use MARKET, LIMIT, or SLTP"
        )),
    }
}

fn parse_position_type(
    s: &str,
) -> Result<twilight_client_sdk::relayer_types::PositionType, String> {
    match s.to_uppercase().as_str() {
        "LONG" => Ok(twilight_client_sdk::relayer_types::PositionType::LONG),
        "SHORT" => Ok(twilight_client_sdk::relayer_types::PositionType::SHORT),
        other => Err(format!("Unknown position side: {other}. Use LONG or SHORT")),
    }
}

/// Parse a date string (RFC3339 or YYYY-MM-DD) into a `DateTime<Utc>`.
fn parse_datetime(s: &str) -> Result<chrono::DateTime<chrono::Utc>, String> {
    use chrono::{NaiveDate, TimeZone, Utc};
    // Try RFC3339 first
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    // Try YYYY-MM-DD
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0).ok_or("invalid date")?));
    }
    Err(format!(
        "Invalid date '{}'. Use RFC3339 (2024-01-15T00:00:00Z) or YYYY-MM-DD (2024-01-15)",
        s
    ))
}

/// Parse a candle interval string into the Interval enum.
fn parse_interval(s: &str) -> Result<nyks_wallet::relayer_module::relayer_types::Interval, String> {
    use nyks_wallet::relayer_module::relayer_types::Interval;
    match s.to_lowercase().as_str() {
        "1m" | "1min" => Ok(Interval::ONE_MINUTE),
        "5m" | "5min" => Ok(Interval::FIVE_MINUTE),
        "15m" | "15min" => Ok(Interval::FIFTEEN_MINUTE),
        "30m" | "30min" => Ok(Interval::THIRTY_MINUTE),
        "1h" => Ok(Interval::ONE_HOUR),
        "4h" => Ok(Interval::FOUR_HOUR),
        "8h" => Ok(Interval::EIGHT_HOUR),
        "12h" => Ok(Interval::TWELVE_HOUR),
        "1d" => Ok(Interval::ONE_DAY),
        other => Err(format!(
            "Unknown interval: {}. Use: 1m, 5m, 15m, 30m, 1h, 4h, 8h, 12h, 1d",
            other
        )),
    }
}

/// Parse an order status string into the OrderStatus enum.
fn parse_order_status(s: &str) -> Result<OrderStatus, String> {
    match s.to_uppercase().as_str() {
        "PENDING" => Ok(OrderStatus::PENDING),
        "FILLED" => Ok(OrderStatus::FILLED),
        "SETTLED" => Ok(OrderStatus::SETTLED),
        "CANCELLED" => Ok(OrderStatus::CANCELLED),
        "LENDED" => Ok(OrderStatus::LENDED),
        "LIQUIDATE" => Ok(OrderStatus::LIQUIDATE),
        other => Err(format!(
            "Unknown order status: {}. Use: PENDING, FILLED, SETTLED, CANCELLED, LENDED, LIQUIDATE",
            other
        )),
    }
}

/// Build an `OrderWallet` from DB. Password falls back to `NYKS_WALLET_PASSPHRASE` env var.
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
fn load_order_wallet_from_db(
    wallet_id: &str,
    password: Option<String>,
    db_url: Option<String>,
) -> Result<OrderWallet, String> {
    let pwd = resolve_password(password).map(|p| SecretString::new(p.into()));
    OrderWallet::load_from_db(wallet_id.to_string(), pwd, db_url)
}

// ---------------------------------------------------------------------------
// Session password cache  (~/.cache/nyks-wallet/session-<ppid>)
// ---------------------------------------------------------------------------
//
// The file is named by the *parent* shell's PID.  Before trusting the cached
// value we verify that PID is still alive via kill(pid, 0) – so when the
// terminal is closed and the shell exits, subsequent invocations find the
// parent dead and silently discard the stale file.
//
// Security model: the file lives in ~/.cache/nyks-wallet/ (mode 0700) and is
// itself mode 0600 – the same protection as ~/.ssh/id_rsa.  No other process
// owned by the same user can read it.

#[cfg(unix)]
fn get_ppid() -> Option<u32> {
    // Use libc::getppid() which works on both Linux and macOS.
    // The previous /proc/self/status approach only worked on Linux.
    let ppid = unsafe { libc::getppid() };
    if ppid > 0 {
        Some(ppid as u32)
    } else {
        None
    }
}

#[cfg(unix)]
fn is_process_alive(pid: u32) -> bool {
    // Signal 0 checks process existence without sending a real signal.
    // Works on both Linux and macOS (unlike /proc/{pid} which is Linux-only).
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

fn session_dir() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(
        std::path::PathBuf::from(home)
            .join(".cache")
            .join("nyks-wallet"),
    )
}

#[cfg(unix)]
fn session_file_path(ppid: u32) -> Option<std::path::PathBuf> {
    Some(session_dir()?.join(format!("session-{ppid}.lock")))
}

/// Save wallet_id and password to session cache, bound to the current shell (PPID).
#[cfg(unix)]
fn session_save(wallet_id: &str, password: &str) -> Result<(), String> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let ppid = get_ppid().ok_or("cannot determine parent shell PID")?;
    let dir = session_dir().ok_or("cannot determine home directory")?;

    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
        .map_err(|e| e.to_string())?;

    let path = session_file_path(ppid).ok_or("cannot build session file path")?;
    let content = format!("{ppid}\n{wallet_id}\n{password}");
    // Create with 0o600 atomically to avoid a TOCTOU window where the file
    // is briefly world-readable under the default umask.
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)
        .map_err(|e| e.to_string())?;
    file.write_all(content.as_bytes())
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Load wallet_id and password from session cache; returns None if shell is gone or cache is missing.
#[cfg(unix)]
fn session_load() -> Option<(String, String)> {
    let ppid = get_ppid()?;
    if !is_process_alive(ppid) {
        session_clear_for(ppid); // clean up stale file
        return None;
    }
    let path = session_file_path(ppid)?;
    let content = std::fs::read_to_string(&path).ok()?;
    let mut lines = content.splitn(3, '\n');
    let stored = lines.next()?;
    if stored.trim().parse::<u32>().ok()? != ppid {
        return None; // sanity-check: file belongs to this shell
    }
    let wallet_id = lines.next()?.to_string();
    let password = lines.next()?.to_string();
    Some((wallet_id, password))
}

/// Load only the password from session cache.
#[cfg(unix)]
fn session_load_password() -> Option<String> {
    session_load().map(|(_, p)| p)
}

/// Load only the wallet_id from session cache.
#[cfg(unix)]
fn session_load_wallet_id() -> Option<String> {
    session_load().map(|(w, _)| w)
}

/// Zeroize and delete the session file for the current shell.
#[cfg(unix)]
fn session_clear() {
    if let Some(ppid) = get_ppid() {
        session_clear_for(ppid);
    }
}

#[cfg(unix)]
fn session_clear_for(ppid: u32) {
    if let Some(path) = session_file_path(ppid) {
        // Overwrite with zeros before unlinking so the content isn't recoverable
        if let Ok(meta) = std::fs::metadata(&path) {
            let zeros = vec![0u8; meta.len() as usize];
            let _ = std::fs::write(&path, &zeros);
        }
        let _ = std::fs::remove_file(path);
    }
}

// Non-Unix stubs (Windows / wasm – session cache is a no-op there).
#[cfg(not(unix))]
fn session_save(_wallet_id: &str, _password: &str) -> Result<(), String> {
    Err("session cache is only supported on Unix".to_string())
}
#[cfg(not(unix))]
fn session_load() -> Option<(String, String)> {
    None
}
#[cfg(not(unix))]
fn session_load_password() -> Option<String> {
    None
}
#[cfg(not(unix))]
fn session_load_wallet_id() -> Option<String> {
    None
}
#[cfg(not(unix))]
fn session_clear() {}

// ---------------------------------------------------------------------------
// Password / wallet-ID resolution helpers
// ---------------------------------------------------------------------------

/// Resolve password: CLI flag → session cache → `NYKS_WALLET_PASSPHRASE` env var → None.
fn resolve_password(password: Option<String>) -> Option<String> {
    password
        .or_else(session_load_password)
        .or_else(|| std::env::var("NYKS_WALLET_PASSPHRASE").ok())
}

/// Resolve wallet_id: CLI flag → session cache → `NYKS_WALLET_ID` env var → None.
fn resolve_wallet_id(wallet_id: Option<String>) -> Option<String> {
    wallet_id
        .or_else(session_load_wallet_id)
        .or_else(|| std::env::var("NYKS_WALLET_ID").ok())
}

/// Resolve an `OrderWallet` – load from DB using wallet_id (arg or env).
///
/// Priority: CLI arg → `NYKS_WALLET_ID` env var → error.
/// Password priority: CLI arg → `NYKS_WALLET_PASSPHRASE` env var → session cache.
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
async fn resolve_order_wallet(
    wallet_id: Option<String>,
    password: Option<String>,
) -> Result<OrderWallet, String> {
    let wid = resolve_wallet_id(wallet_id)
        .ok_or("wallet_id is required (pass --wallet-id, set NYKS_WALLET_ID env var, or run `wallet unlock`)")?;
    let pwd = resolve_password(password);
    load_order_wallet_from_db(&wid, pwd, None)
}

// ---------------------------------------------------------------------------
// Help
// ---------------------------------------------------------------------------

fn print_global_help() {
    println!(
        r#"Twilight Relayer CLI — manage wallets and orders from the command line.

USAGE:
    relayer-cli [--json] <COMMAND>

COMMANDS:
    wallet      Wallet management (create, import, load, list, balance, accounts,
                export, backup, restore, unlock/lock, change-password, info,
                update-btc-address, sync-nonce)
    zkaccount   ZkOS account operations (fund, withdraw, transfer, split)
    order       Trading and lending orders (open/close/cancel/query trade & lend,
                unlock-trade, history-trade, history-lend, funding-history,
                account-summary, tx-hashes)
    market      Market data (price, orderbook, funding-rate, fee-rate, recent-trades,
                position-size, lend-pool, pool-share-value, last-day-apy,
                open-interest, market-stats, server-time, history-price,
                candles, history-funding, history-fees, apy-chart)
    history     Local DB history (orders, transfers)
    portfolio   Portfolio tracking (summary, balances, risks)

GLOBAL FLAGS:
    --json      Output results as JSON (for scripting)

RESOLUTION PRIORITY (wallet-id & password):
    --flag  >  session cache (wallet unlock)  >  env var

ENVIRONMENT:
    NYKS_WALLET_ID          Default wallet ID
    NYKS_WALLET_PASSPHRASE  Default password

Run `relayer-cli help <COMMAND>` for details on a specific command group."#
    );
}

fn print_wallet_help() {
    println!(
        r#"Wallet management commands.

USAGE:
    relayer-cli wallet <SUBCOMMAND>

SUBCOMMANDS:
    create              Create a new wallet (persisted to DB)
    import              Import wallet from BIP-39 mnemonic
    load                Load a wallet from the database
    list                List all wallets in the database
    balance             Show wallet balance (on-chain query)
    info                Show wallet info (no chain calls)
    accounts            List all ZkOS accounts for a wallet
    export              Export wallet to a JSON file
    backup              Full database backup to JSON
    restore             Restore wallet from backup JSON
    unlock              Cache wallet-id + password for this terminal session
    lock                Clear cached session
    change-password     Change the DB encryption password
    update-btc-address  Update the BTC deposit address
    sync-nonce          Sync nonce/sequence from chain state

EXAMPLES:
    relayer-cli wallet create --btc-address bc1q...
    relayer-cli wallet unlock                         # interactive prompt
    relayer-cli wallet balance                        # uses session cache
    relayer-cli wallet accounts --on-chain-only"#
    );
}

fn print_zkaccount_help() {
    println!(
        r#"ZkOS account operations — fund, withdraw, transfer, and split trading accounts.

USAGE:
    relayer-cli zkaccount <SUBCOMMAND>

SUBCOMMANDS:
    fund        Fund a new ZkOS trading account from on-chain wallet
    withdraw    Withdraw from ZkOS account back to on-chain wallet
    transfer    Transfer balance between ZkOS trading accounts
    split       Split one account into multiple new accounts

AMOUNTS:
    fund/split accept amounts in multiple units (pick one):
        --amount <sats>           Satoshis
        --amount-mbtc <mbtc>      Milli-BTC (1 mBTC = 100,000 sats)
        --amount-btc <btc>        BTC (1 BTC = 100,000,000 sats)

EXAMPLES:
    relayer-cli zkaccount fund --amount 50000
    relayer-cli zkaccount withdraw --account-index 1
    relayer-cli zkaccount transfer --from 1
    relayer-cli zkaccount split --from 0 --balances "10000,20000,30000""#
    );
}

fn print_order_help() {
    println!(
        r#"Trading and lending order commands.

USAGE:
    relayer-cli order <SUBCOMMAND>

TRADING:
    open-trade          Open a leveraged position (MARKET/LIMIT)
    close-trade         Close a position (MARKET/LIMIT/SLTP)
    cancel-trade        Cancel a pending order
    query-trade         Query current order status
    unlock-trade        Reclaim account after SLTP settlement

LENDING:
    open-lend           Open a lend order
    close-lend          Close a lend order
    query-lend          Query lend order status

HISTORY & ANALYTICS (from relayer):
    history-trade       Historical trader orders for an account
    history-lend        Historical lend orders for an account
    funding-history     Funding payment history for a position
    account-summary     Trading activity summary (fills, settles, liquidations)
    tx-hashes           Look up on-chain tx hashes by request/account ID

EXAMPLES:
    relayer-cli order open-trade --account-index 1 --side LONG --entry-price 65000 --leverage 5
    relayer-cli order close-trade --account-index 1
    relayer-cli order query-trade --account-index 1
    relayer-cli order history-trade --account-index 1
    relayer-cli order account-summary --from 2024-01-01 --to 2024-12-31"#
    );
}

fn print_market_help() {
    println!(
        r#"Market data queries (no wallet required).

USAGE:
    relayer-cli market <SUBCOMMAND>

LIVE DATA:
    price               Current BTC/USD price
    orderbook           Open limit orders
    funding-rate        Current funding rate
    fee-rate            Current fee rate
    recent-trades       Recent trade orders
    position-size       Position size summary
    lend-pool           Lend pool info
    pool-share-value    Current pool share value
    last-day-apy        Last 24h annualized yield
    open-interest       Long/short exposure
    market-stats        Comprehensive risk statistics
    server-time         Relayer server time

HISTORICAL DATA:
    history-price       Historical prices over a date range
    candles             OHLCV candlestick data
    history-funding     Historical funding rates
    history-fees        Historical fee rates
    apy-chart           APY chart data for lend pool

EXAMPLES:
    relayer-cli market price
    relayer-cli market candles --interval 1h --since 2024-01-01
    relayer-cli market history-price --from 2024-01-01 --to 2024-01-31
    relayer-cli market apy-chart --range 30d --step 1d"#
    );
}

fn print_history_help() {
    println!(
        r#"Local database history queries (requires DB feature).

USAGE:
    relayer-cli history <SUBCOMMAND>

SUBCOMMANDS:
    orders      Show order history (open, close, cancel events)
    transfers   Show transfer history (fund, withdraw, transfer events)

EXAMPLES:
    relayer-cli history orders --limit 20
    relayer-cli history transfers --limit 10"#
    );
}

fn print_portfolio_help() {
    println!(
        r#"Portfolio and position tracking.

USAGE:
    relayer-cli portfolio <SUBCOMMAND>

SUBCOMMANDS:
    summary     Full portfolio summary (balances, positions, PnL)
    balances    Per-account balance breakdown (--unit sats|mbtc|btc)
    risks       Liquidation risk for open positions

EXAMPLES:
    relayer-cli portfolio summary
    relayer-cli portfolio balances --unit btc
    relayer-cli portfolio risks"#
    );
}

fn print_subcommand_help(group: &str) {
    match group.to_lowercase().as_str() {
        "wallet" => print_wallet_help(),
        "zkaccount" => print_zkaccount_help(),
        "order" => print_order_help(),
        "market" => print_market_help(),
        "history" => print_history_help(),
        "portfolio" => print_portfolio_help(),
        _ => {
            eprintln!("Unknown command group: '{}'\n", group);
            print_global_help();
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    let cli = Cli::parse();

    let json_output = cli.json;

    let result = match cli.command {
        Commands::Wallet(cmd) => handle_wallet(cmd).await,
        Commands::Zkaccount(cmd) => handle_zkaccount(cmd).await,
        Commands::Order(cmd) => handle_order(cmd, json_output).await,
        Commands::Market(cmd) => handle_market(cmd, json_output).await,
        Commands::History(cmd) => handle_history(cmd).await,
        Commands::Portfolio(cmd) => handle_portfolio(cmd, json_output).await,
        Commands::Help { command } => {
            match command {
                Some(group) => print_subcommand_help(&group),
                None => print_global_help(),
            }
            Ok(())
        }
    };

    if let Err(e) = result {
        error!("{}", e);
        // eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Wallet handlers
// ---------------------------------------------------------------------------

/// Validate that a string is a valid BTC SegWit address (bc1q... or bc1p...) on mainnet.
fn validate_btc_segwit_address(addr: &str) -> Result<(), String> {
    use bitcoin::Address;
    use std::str::FromStr;
    let parsed = Address::from_str(addr)
        .map_err(|e| format!("Invalid BTC address: {}", e))?
        .require_network(bitcoin::Network::Bitcoin)
        .map_err(|e| format!("Address is not for Bitcoin mainnet: {}", e))?;
    if !parsed.to_string().starts_with("bc1") {
        return Err("Address must be a SegWit address (bc1q... or bc1p...)".to_string());
    }
    Ok(())
}

async fn handle_wallet(cmd: WalletCmd) -> Result<(), String> {
    match cmd {
        WalletCmd::Create {
            wallet_id,
            password,
            btc_address,
        } => {
            if let Some(ref addr) = btc_address {
                validate_btc_segwit_address(addr)?;
            }
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;
            if let Some(addr) = btc_address {
                ow.wallet.btc_address = addr;
                ow.wallet.btc_address_registered = false;
            }
            println!("Wallet created successfully");
            println!("  Address: {}", ow.wallet.twilightaddress);
            println!("  BTC address: {}", ow.wallet.btc_address);

            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            {
                let pwd = resolve_password(password).map(|p| SecretString::new(p.into()));
                ow.with_db(pwd, wallet_id.clone())?;
                println!(
                    "  Wallet ID: {}",
                    wallet_id.unwrap_or_else(|| ow.wallet.twilightaddress.clone())
                );
            }
            Ok(())
        }

        WalletCmd::Import {
            mnemonic,
            wallet_id,
            password,
            btc_address,
        } => {
            if let Some(ref addr) = btc_address {
                validate_btc_segwit_address(addr)?;
            }
            let mnemonic = match mnemonic {
                Some(m) => m.trim().to_string(),
                None => {
                    let m = rpassword::prompt_password("Mnemonic phrase: ")
                        .map_err(|e| e.to_string())?;
                    if m.trim().is_empty() {
                        return Err("mnemonic must not be empty".to_string());
                    }
                    m.trim().to_string()
                }
            };
            let mut ow =
                OrderWallet::import_from_mnemonic(&mnemonic, None).map_err(|e| e.to_string())?;
            if let Some(addr) = btc_address {
                ow.wallet.btc_address = addr;
                ow.wallet.btc_address_registered = false;
            }
            println!("Wallet imported successfully");
            println!("  Address: {}", ow.wallet.twilightaddress);
            println!("  BTC address: {}", ow.wallet.btc_address);

            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            {
                let pwd = resolve_password(password).map(|p| SecretString::new(p.into()));
                ow.with_db(pwd, wallet_id.clone())?;
                println!(
                    "  Wallet ID: {}",
                    wallet_id.unwrap_or_else(|| ow.wallet.twilightaddress.clone())
                );
            }

            println!();
            println!("Note: If the BTC address above is not the one you use, update it with:");
            println!("  relayer-cli wallet update-btc-address --btc-address <your-bc1-address> --wallet-id <your_wallet_id>");
            println!();
            println!("Tip: To avoid typing --password on every command, cache it for this terminal session:");
            println!("  relayer-cli wallet unlock");
            Ok(())
        }

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::Load {
            wallet_id,
            password,
            db_url,
        } => {
            let ow = load_order_wallet_from_db(&wallet_id, password, db_url)?;
            println!("Wallet loaded from database");
            println!("  Wallet ID: {}", wallet_id);
            println!("  Address: {}", ow.wallet.twilightaddress);
            println!("  BTC address: {}", ow.wallet.btc_address);
            println!("  Chain ID: {}", ow.chain_id);
            println!("  ZkOS accounts: {}", ow.zk_accounts.accounts.len());
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::Load { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        WalletCmd::Balance {
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let balance = ow
                .wallet
                .update_balance()
                .await
                .map_err(|e| e.to_string())?;
            println!("Wallet Balance");
            println!("  Address:  {}", ow.wallet.twilightaddress);
            println!("  NYKS:     {}", balance.nyks);
            println!("  SATS:     {}", balance.sats);
            Ok(())
        }

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::List { db_url } => {
            let wallets = OrderWallet::get_wallet_list_from_db(db_url)?;
            if wallets.is_empty() {
                println!("No wallets found in database");
            } else {
                println!("{:<40} {:<20}", "WALLET ID", "CREATED AT");
                println!("{}", "-".repeat(60));
                for w in &wallets {
                    println!("{:<40} {:<20}", w.wallet_id, w.created_at);
                }
                println!("\nTotal: {} wallet(s)", wallets.len());
            }
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::List { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        WalletCmd::Export {
            output,
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            ow.wallet
                .export_to_json(&output)
                .map_err(|e| e.to_string())?;
            println!("Wallet exported to {output}");
            Ok(())
        }

        WalletCmd::Accounts {
            wallet_id,
            password,
            on_chain_only,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let mut accounts = ow.zk_accounts.get_all_accounts();
            accounts.sort_by_key(|a| a.index);
            if on_chain_only {
                accounts.retain(|a| a.on_chain);
            }
            if accounts.is_empty() {
                println!("No ZkOS accounts found");
            } else {
                println!(
                    "{:<8} {:<12} {:<10} {:<10} {:<44}",
                    "INDEX", "BALANCE", "ON-CHAIN", "IO-TYPE", "ACCOUNT"
                );
                println!("{}", "-".repeat(90));
                for acc in accounts {
                    println!(
                        "{:<8} {:<12} {:<10} {:<10} {:<44}",
                        acc.index,
                        acc.balance,
                        acc.on_chain,
                        format!("{:?}", acc.io_type),
                        &acc.account[..std::cmp::min(44, acc.account.len())],
                    );
                }
            }
            Ok(())
        }

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::Info {
            wallet_id,
            password,
        } => {
            let ow = resolve_order_wallet(wallet_id, password).await?;
            println!("Wallet Info");
            println!("  Address:         {}", ow.wallet.twilightaddress);
            println!("  BTC address:     {}", ow.wallet.btc_address);
            println!("  BTC registered:  {}", ow.wallet.btc_address_registered);
            println!("  Chain ID:        {}", ow.chain_id);
            println!("  ZkOS accounts:   {}", ow.zk_accounts.accounts.len());
            println!("  Next nonce:      {}", ow.nonce_manager.peek_next());
            println!("  Account number:  {}", ow.nonce_manager.account_number());
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::Info { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::Backup {
            output,
            wallet_id,
            password,
        } => {
            let ow = load_order_wallet_from_db(&wallet_id, password, None)?;
            let db_manager = ow
                .get_db_manager()
                .ok_or("Database not enabled on this wallet")?;
            db_manager.export_backup_to_file(&output)?;
            println!("Backup exported to {output}");
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::Backup { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::Restore {
            input,
            wallet_id,
            password,
            force,
        } => {
            let ow = load_order_wallet_from_db(&wallet_id, password, None)?;
            let db_manager = ow
                .get_db_manager()
                .ok_or("Database not enabled on this wallet")?;
            db_manager.import_backup_from_file(&input, force)?;
            println!("Backup restored from {input}");
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::Restore { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        WalletCmd::SyncNonce {
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            ow.sync_nonce().await?;
            println!("Nonce synced from chain");
            println!("  Next sequence: {}", ow.nonce_manager.peek_next());
            println!("  Account number: {}", ow.nonce_manager.account_number());
            println!(
                "  Released (pending reuse): {}",
                ow.nonce_manager.released_count()
            );
            Ok(())
        }

        WalletCmd::Unlock { wallet_id, force } => {
            // If a session is already active, error unless --force.
            if session_load().is_some() && !force {
                eprintln!(
                    "A session is already cached. Run `wallet lock` first or use `wallet unlock --force`."
                );
                return Err("session already active".to_string());
            }

            // Resolve wallet_id: flag → env → interactive prompt
            let wid = if let Some(id) = wallet_id {
                id
            } else if let Ok(id) = std::env::var("NYKS_WALLET_ID") {
                println!("Using wallet from NYKS_WALLET_ID: {}", id);
                id
            } else {
                // List available wallets before prompting
                #[cfg(any(feature = "sqlite", feature = "postgresql"))]
                {
                    match OrderWallet::get_wallet_list_from_db(None) {
                        Ok(wallets) if !wallets.is_empty() => {
                            println!("Available wallets:");
                            println!("{:<40} {:<20}", "WALLET ID", "CREATED AT");
                            println!("{}", "-".repeat(60));
                            for w in &wallets {
                                println!("{:<40} {:<20}", w.wallet_id, w.created_at);
                            }
                            println!();
                        }
                        _ => {
                            println!("No wallets found in database.\n");
                        }
                    }
                }
                let mut input = String::new();
                eprint!("Wallet ID: ");
                std::io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| e.to_string())?;
                let input = input.trim().to_string();
                if input.is_empty() {
                    return Err("wallet_id must not be empty".to_string());
                }
                input
            };

            let password =
                rpassword::prompt_password("Wallet password: ").map_err(|e| e.to_string())?;
            if password.is_empty() {
                return Err("password must not be empty".to_string());
            }

            // Verify the wallet_id + password combination before caching
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            {
                load_order_wallet_from_db(&wid, Some(password.clone()), None)
                    .map_err(|e| format!("Failed to unlock wallet '{}': {}", wid, e))?;
            }

            session_save(&wid, &password)?;
            println!("Session cached for wallet '{}' in this terminal.", wid);
            println!("Run `wallet lock` to clear it, or just close the terminal.");
            Ok(())
        }

        WalletCmd::Lock => {
            session_clear();
            println!("Session password cleared.");
            Ok(())
        }

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::ChangePassword { wallet_id } => {
            let wid = resolve_wallet_id(wallet_id)
                .ok_or("wallet_id is required (pass --wallet-id, set NYKS_WALLET_ID env var, or run `wallet unlock`)")?;

            // Always prompt via TTY — ignore session cache and env var
            let old_password =
                rpassword::prompt_password("Current password: ").map_err(|e| e.to_string())?;
            if old_password.is_empty() {
                return Err("password must not be empty".to_string());
            }

            // Load wallet with old password to verify it's correct
            let ow = load_order_wallet_from_db(&wid, Some(old_password), None)?;

            let new_password =
                rpassword::prompt_password("New password: ").map_err(|e| e.to_string())?;
            if new_password.is_empty() {
                return Err("new password must not be empty".to_string());
            }
            let confirm_password =
                rpassword::prompt_password("Confirm new password: ").map_err(|e| e.to_string())?;
            if new_password != confirm_password {
                return Err("passwords do not match".to_string());
            }

            let db_manager = ow
                .get_db_manager()
                .ok_or("database manager not available")?;
            let new_secret = SecretString::new(new_password.into());
            db_manager.save_encrypted_wallet(&ow.wallet, &new_secret)?;

            // Update session cache if one exists
            if session_load().is_some() {
                session_save(&wid, new_secret.expose_secret())?;
            }

            println!("Password changed successfully for wallet '{}'.", wid);
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::ChangePassword { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::UpdateBtcAddress {
            btc_address,
            wallet_id,
            password,
        } => {
            validate_btc_segwit_address(&btc_address)?;

            let mut ow = resolve_order_wallet(wallet_id, password).await?;

            let old_address = ow.wallet.btc_address.clone();
            ow.wallet.btc_address = btc_address.clone();
            ow.wallet.btc_address_registered = false;

            let db_manager = ow
                .get_db_manager()
                .ok_or("database manager not available")?;
            let wallet_password = ow
                .get_wallet_password()
                .ok_or("wallet password not available — cannot persist changes")?;
            db_manager.save_encrypted_wallet(&ow.wallet, wallet_password)?;

            println!("BTC address updated for wallet.");
            println!("  Old: {}", old_address);
            println!("  New: {}", btc_address);
            println!("  Registered: false (will re-register on next balance check)");
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::UpdateBtcAddress { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),
    }
}

// ---------------------------------------------------------------------------
// ZkOS account handlers
// ---------------------------------------------------------------------------

async fn handle_zkaccount(cmd: ZkaccountCmd) -> Result<(), String> {
    match cmd {
        ZkaccountCmd::Fund {
            amount,
            amount_mbtc,
            amount_btc,
            wallet_id,
            password,
        } => {
            let provided = [
                amount.is_some(),
                amount_mbtc.is_some(),
                amount_btc.is_some(),
            ]
            .iter()
            .filter(|&&v| v)
            .count();

            if provided == 0 {
                return Err("No amount specified. Provide one of:\n  \
                     --amount <sats>        Amount in satoshis\n  \
                     --amount-mbtc <mbtc>   Amount in milli-BTC (1 mBTC = 100,000 sats)\n  \
                     --amount-btc <btc>     Amount in BTC (1 BTC = 100,000,000 sats)"
                    .to_string());
            }
            if provided > 1 {
                eprintln!(
                    "Warning: Multiple amount flags provided. Using priority: --amount > --amount-mbtc > --amount-btc"
                );
            }

            let amount_sats: u64 = if let Some(sats) = amount {
                sats
            } else if let Some(mbtc) = amount_mbtc {
                (mbtc * 100_000.0).round() as u64
            } else if let Some(btc) = amount_btc {
                (btc * 100_000_000.0).round() as u64
            } else {
                unreachable!()
            };

            if amount_sats == 0 {
                return Err("Amount must be greater than 0".to_string());
            }

            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            println!("Funding {amount_sats} sats to new ZkOS trading account...");
            let (tx_result, account_index) = ow.funding_to_trading(amount_sats).await?;
            println!("Funding successful");
            println!("  TX hash: {}", tx_result.tx_hash);
            println!("  TX code: {}", tx_result.code);
            println!("  Account index: {account_index}");
            Ok(())
        }

        ZkaccountCmd::Withdraw {
            account_index,
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            println!("Withdrawing from ZkOS account {account_index} back to on-chain wallet...");
            ow.trading_to_funding(account_index).await?;
            println!("Withdrawal successful");
            Ok(())
        }

        ZkaccountCmd::Transfer {
            from,
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            println!("Transferring from ZkOS account {from} to new account...");
            let new_index = ow.trading_to_trading(from).await?;
            println!("Transfer successful");
            println!("  New account index: {new_index}");
            Ok(())
        }

        ZkaccountCmd::Split {
            from,
            balances,
            balances_mbtc,
            balances_btc,
            wallet_id,
            password,
        } => {
            let provided = [
                balances.is_some(),
                balances_mbtc.is_some(),
                balances_btc.is_some(),
            ]
            .iter()
            .filter(|&&v| v)
            .count();

            if provided == 0 {
                return Err(
                    "No balances specified. Provide one of:\n  \
                     --balances <sats>          Comma-separated balances in satoshis\n  \
                     --balances-mbtc <mbtc>     Comma-separated balances in milli-BTC (1 mBTC = 100,000 sats)\n  \
                     --balances-btc <btc>       Comma-separated balances in BTC (1 BTC = 100,000,000 sats)"
                        .to_string(),
                );
            }
            if provided > 1 {
                eprintln!(
                    "Warning: Multiple balance flags provided. Using priority: --balances > --balances-mbtc > --balances-btc"
                );
            }

            let balance_vec: Vec<u64> = if let Some(ref b) = balances {
                b.split(',')
                    .map(|s| {
                        s.trim()
                            .parse::<u64>()
                            .map_err(|e| format!("Invalid balance '{}': {}", s.trim(), e))
                    })
                    .collect::<Result<Vec<u64>, String>>()?
            } else if let Some(ref b) = balances_mbtc {
                b.split(',')
                    .map(|s| {
                        s.trim()
                            .parse::<f64>()
                            .map(|v| (v * 100_000.0).round() as u64)
                            .map_err(|e| format!("Invalid mBTC balance '{}': {}", s.trim(), e))
                    })
                    .collect::<Result<Vec<u64>, String>>()?
            } else if let Some(ref b) = balances_btc {
                b.split(',')
                    .map(|s| {
                        s.trim()
                            .parse::<f64>()
                            .map(|v| (v * 100_000_000.0).round() as u64)
                            .map_err(|e| format!("Invalid BTC balance '{}': {}", s.trim(), e))
                    })
                    .collect::<Result<Vec<u64>, String>>()?
            } else {
                unreachable!()
            };

            if balance_vec.is_empty() {
                return Err("At least one balance is required".into());
            }
            if balance_vec.iter().any(|&b| b == 0) {
                return Err("All balances must be greater than 0".to_string());
            }

            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let total: u64 = balance_vec.iter().sum();
            println!(
                "Splitting ZkOS account {} into {} accounts (total: {} sats)...",
                from,
                balance_vec.len(),
                total
            );
            let results = ow
                .trading_to_trading_multiple_accounts(from, balance_vec)
                .await?;
            println!("Split successful");
            for (idx, bal) in &results {
                println!("  Account {}: {} sats", idx, bal);
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Order handlers
// ---------------------------------------------------------------------------

async fn handle_order(cmd: OrderCmd, json_output: bool) -> Result<(), String> {
    match cmd {
        OrderCmd::OpenTrade {
            account_index,
            order_type,
            side,
            entry_price,
            leverage,
            wallet_id,
            password,
        } => {
            let ot = parse_order_type(&order_type)?;
            let ps = parse_position_type(&side)?;

            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            println!(
                "Opening {side} {order_type} order on account {account_index} (price={entry_price}, leverage={leverage}x)..."
            );
            let request_id = ow
                .open_trader_order(account_index, ot, ps, entry_price, leverage)
                .await?;
            println!("Order submitted successfully");
            println!("  Request ID: {request_id}");
            Ok(())
        }

        OrderCmd::CloseTrade {
            account_index,
            order_type,
            execution_price,
            stop_loss,
            take_profit,
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            println!("Closing trader order on account {account_index}...");

            let request_id = if stop_loss.is_some() || take_profit.is_some() {
                let ot = parse_order_type("SLTP")?;
                ow.close_trader_order_sltp(
                    account_index,
                    ot,
                    execution_price,
                    stop_loss,
                    take_profit,
                )
                .await?
            } else {
                let ot = parse_order_type(&order_type)?;
                ow.close_trader_order(account_index, ot, execution_price)
                    .await?
            };

            println!("Order closed successfully");
            println!("  Request ID: {request_id}");
            Ok(())
        }

        OrderCmd::CancelTrade {
            account_index,
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            println!("Cancelling trader order on account {account_index}...");
            let request_id = ow.cancel_trader_order(account_index).await?;
            println!("Order cancelled successfully");
            println!("  Request ID: {request_id}");
            Ok(())
        }

        OrderCmd::QueryTrade {
            account_index,
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let order = ow.query_trader_order_v1(account_index).await?;
            println!("Trader Order (account {account_index})");
            println!(
                "{}",
                serde_json::to_string_pretty(&order).unwrap_or_else(|_| format!("{:?}", order))
            );
            Ok(())
        }

        OrderCmd::OpenLend {
            account_index,
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            println!("Opening lend order on account {account_index}...");
            let request_id = ow.open_lend_order(account_index).await?;
            println!("Lend order submitted successfully");
            println!("  Request ID: {request_id}");
            Ok(())
        }

        OrderCmd::CloseLend {
            account_index,
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            println!("Closing lend order on account {account_index}...");
            let request_id = ow.close_lend_order(account_index).await?;
            println!("Lend order closed successfully");
            println!("  Request ID: {request_id}");
            Ok(())
        }

        OrderCmd::UnlockTrade {
            account_index,
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let status = ow.unlock_settled_order(account_index).await?;
            match status {
                OrderStatus::SETTLED => {
                    println!(
                        "Account {} unlocked successfully (order settled).",
                        account_index
                    );
                }
                _ => {
                    println!(
                        "Account {} not yet settled (current status: {:?}). No changes made.",
                        account_index, status
                    );
                }
            }
            Ok(())
        }

        OrderCmd::QueryLend {
            account_index,
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let order = ow.query_lend_order_v1(account_index).await?;
            println!("Lend Order (account {account_index})");
            println!(
                "{}",
                serde_json::to_string_pretty(&order).unwrap_or_else(|_| format!("{:?}", order))
            );
            Ok(())
        }

        OrderCmd::HistoryTrade {
            account_index,
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let orders = ow.historical_trader_order(account_index).await?;
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&orders)
                        .unwrap_or_else(|_| format!("{:?}", orders))
                );
            } else if orders.is_empty() {
                println!("No historical trader orders for account {account_index}");
            } else {
                println!("Historical Trader Orders (account {account_index})");
                println!("{}", "-".repeat(130));
                println!(
                    "  {:<36} {:<10} {:<8} {:<10} {:>12} {:>12} {:>6} {:>12} {:>12}",
                    "UUID", "STATUS", "TYPE", "SIDE", "ENTRY", "SIZE", "LEV", "MARGIN", "PnL"
                );
                for o in &orders {
                    println!(
                        "  {:<36} {:<10} {:<8} {:<10} {:>12.2} {:>12.2} {:>5.0}x {:>12.2} {:>12.2}",
                        o.uuid,
                        format!("{:?}", o.order_status),
                        format!("{:?}", o.order_type),
                        format!("{:?}", o.position_type),
                        o.entryprice,
                        o.positionsize,
                        o.leverage,
                        o.initial_margin,
                        o.unrealized_pnl,
                    );
                }
                println!("\nTotal: {} order(s)", orders.len());
            }
            Ok(())
        }

        OrderCmd::HistoryLend {
            account_index,
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let orders = ow.historical_lend_order(account_index).await?;
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&orders)
                        .unwrap_or_else(|_| format!("{:?}", orders))
                );
            } else if orders.is_empty() {
                println!("No historical lend orders for account {account_index}");
            } else {
                println!("Historical Lend Orders (account {account_index})");
                println!("{}", "-".repeat(100));
                println!(
                    "  {:<36} {:<10} {:>12} {:>12} {:>12} {:>12}",
                    "UUID", "STATUS", "DEPOSIT", "BALANCE", "SHARES", "PAYMENT"
                );
                for o in &orders {
                    println!(
                        "  {:<36} {:<10} {:>12.2} {:>12.2} {:>12.4} {:>12.4}",
                        o.uuid,
                        format!("{:?}", o.order_status),
                        o.deposit,
                        o.balance,
                        o.npoolshare,
                        o.payment,
                    );
                }
                println!("\nTotal: {} order(s)", orders.len());
            }
            Ok(())
        }

        OrderCmd::FundingHistory {
            account_index,
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let entries = ow.order_funding_history(account_index).await?;
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&entries)
                        .unwrap_or_else(|_| format!("{:?}", entries))
                );
            } else if entries.is_empty() {
                println!("No funding history for account {account_index}");
            } else {
                println!("Funding History (account {account_index})");
                println!("{}", "-".repeat(80));
                println!(
                    "  {:<24} {:<8} {:>14} {:>14} {:<36}",
                    "TIME", "SIDE", "PAYMENT", "RATE", "ORDER ID"
                );
                let mut total_payment = 0.0_f64;
                for e in &entries {
                    total_payment += e.payment;
                    println!(
                        "  {:<24} {:<8} {:>14.6} {:>14.8} {:<36}",
                        &e.time[..std::cmp::min(24, e.time.len())],
                        format!("{:?}", e.position_side),
                        e.payment,
                        e.funding_rate,
                        e.order_id,
                    );
                }
                println!(
                    "\n  Total funding: {:.6} over {} entries",
                    total_payment,
                    entries.len()
                );
            }
            Ok(())
        }

        OrderCmd::AccountSummary {
            wallet_id,
            password,
            from,
            to,
            since,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            use nyks_wallet::relayer_module::relayer_types::AccountSummaryArgs;
            let params = AccountSummaryArgs {
                t_address: ow.wallet.twilightaddress.clone(),
                from: from.map(|s| parse_datetime(&s)).transpose()?,
                to: to.map(|s| parse_datetime(&s)).transpose()?,
                since: since.map(|s| parse_datetime(&s)).transpose()?,
            };
            let summary = ow
                .relayer_api_client
                .account_summary_by_twilight_address(params)
                .await
                .map_err(|e| e.to_string())?;
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&summary)
                        .unwrap_or_else(|_| format!("{:?}", summary))
                );
            } else {
                println!("Account Summary for {}", ow.wallet.twilightaddress);
                println!("{}", "=".repeat(50));
                println!("  Period: {} — {}", summary.from, summary.to);
                println!("  Filled count:       {}", summary.filled_count);
                println!("  Filled size:        {:.4}", summary.filled_positionsize);
                println!("  Settled count:      {}", summary.settled_count);
                println!("  Settled size:       {:.4}", summary.settled_positionsize);
                println!("  Liquidated count:   {}", summary.liquidated_count);
                println!(
                    "  Liquidated size:    {:.4}",
                    summary.liquidated_positionsize
                );
            }
            Ok(())
        }

        OrderCmd::TxHashes {
            by,
            id,
            status,
            limit,
            offset,
        } => {
            use nyks_wallet::relayer_module::relayer_api::RelayerJsonRpcClient;
            use nyks_wallet::relayer_module::relayer_types::TransactionHashArgs;

            let endpoint = std::env::var("RELAYER_API_RPC_SERVER_URL")
                .unwrap_or_else(|_| "http://0.0.0.0:8088/api".to_string());
            let client = RelayerJsonRpcClient::new(&endpoint).map_err(|e| e.to_string())?;

            let status = status.map(|s| parse_order_status(&s)).transpose()?;
            let params = match by.to_lowercase().as_str() {
                "request" => TransactionHashArgs::RequestId {
                    id,
                    status,
                    limit,
                    offset,
                },
                "account" => TransactionHashArgs::AccountId {
                    id,
                    status,
                    limit,
                    offset,
                },
                "tx" => TransactionHashArgs::TxId {
                    id,
                    status,
                    limit,
                    offset,
                },
                other => {
                    return Err(format!(
                        "Unknown lookup mode: '{}'. Use: request, account, or tx",
                        other
                    ))
                }
            };

            let hashes = client
                .transaction_hashes(params)
                .await
                .map_err(|e| e.to_string())?;
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&hashes)
                        .unwrap_or_else(|_| format!("{:?}", hashes))
                );
            } else if hashes.is_empty() {
                println!("No transaction hashes found");
            } else {
                println!("Transaction Hashes");
                println!("{}", "-".repeat(120));
                println!(
                    "  {:<36} {:<10} {:<10} {:<64} {:<20}",
                    "ORDER ID", "STATUS", "TYPE", "TX HASH", "DATE"
                );
                for h in &hashes {
                    println!(
                        "  {:<36} {:<10} {:<10} {:<64} {:<20}",
                        h.order_id,
                        format!("{:?}", h.order_status),
                        format!("{:?}", h.order_type),
                        h.tx_hash,
                        &h.datetime[..std::cmp::min(20, h.datetime.len())],
                    );
                }
                println!("\nTotal: {} hash(es)", hashes.len());
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// History handlers
// ---------------------------------------------------------------------------

async fn handle_history(cmd: HistoryCmd) -> Result<(), String> {
    #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
    {
        let _ = cmd;
        return Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        );
    }

    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    match cmd {
        HistoryCmd::Orders {
            wallet_id,
            password,
            account_index,
            limit,
            offset,
        } => {
            let wallet_id = resolve_wallet_id(wallet_id)
                .ok_or("wallet_id is required (pass --wallet-id, set NYKS_WALLET_ID env var, or run `wallet unlock`)")?;
            let ow = load_order_wallet_from_db(&wallet_id, password, None)?;
            let filter = nyks_wallet::relayer_module::transaction_history::OrderHistoryFilter {
                account_index,
                limit: Some(limit),
                offset: Some(offset),
            };
            let entries = ow.get_order_history(filter)?;

            if entries.is_empty() {
                println!("No order history found");
            } else {
                println!(
                    "{:<6} {:<12} {:<10} {:<8} {:<10} {:<12} {:<10} {:<20}",
                    "ACCT", "ACTION", "TYPE", "SIDE", "AMOUNT", "PRICE", "STATUS", "CREATED"
                );
                println!("{}", "-".repeat(98));
                for e in &entries {
                    println!(
                        "{:<6} {:<12} {:<10} {:<8} {:<10} {:<12} {:<10} {:<20}",
                        e.account_index,
                        e.action,
                        e.order_type,
                        e.position_type.as_deref().unwrap_or("-"),
                        e.amount,
                        e.price
                            .map(|p| format!("{:.2}", p))
                            .unwrap_or_else(|| "-".to_string()),
                        e.status,
                        &e.created_at[..std::cmp::min(19, e.created_at.len())],
                    );
                }
                println!("\nShowing {} entries", entries.len());
            }
            Ok(())
        }

        HistoryCmd::Transfers {
            wallet_id,
            password,
            limit,
            offset,
        } => {
            let wallet_id = resolve_wallet_id(wallet_id)
                .ok_or("wallet_id is required (pass --wallet-id, set NYKS_WALLET_ID env var, or run `wallet unlock`)")?;
            let ow = load_order_wallet_from_db(&wallet_id, password, None)?;
            let filter = nyks_wallet::relayer_module::transaction_history::TransferHistoryFilter {
                limit: Some(limit),
                offset: Some(offset),
            };
            let entries = ow.get_transfer_history(filter)?;

            if entries.is_empty() {
                println!("No transfer history found");
            } else {
                println!(
                    "{:<16} {:<6} {:<6} {:<12} {:<20} {}",
                    "DIRECTION", "FROM", "TO", "AMOUNT", "CREATED", "TX HASH"
                );
                println!("{}", "-".repeat(100));
                for e in &entries {
                    println!(
                        "{:<16} {:<6} {:<6} {:<12} {:<20} {}",
                        e.direction,
                        e.from_index
                            .map(|i| i.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        e.to_index
                            .map(|i| i.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        e.amount,
                        &e.created_at[..std::cmp::min(19, e.created_at.len())],
                        e.tx_hash.as_deref().unwrap_or("-"),
                    );
                }
                println!("\nShowing {} entries", entries.len());
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Portfolio handlers
// ---------------------------------------------------------------------------

async fn handle_portfolio(cmd: PortfolioCmd, _json_output: bool) -> Result<(), String> {
    match cmd {
        PortfolioCmd::Summary {
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let portfolio = ow.get_portfolio_summary().await?;

            // Extract mark price from the first trader position (same for all).
            let mark_price = portfolio.trader_positions.first().map(|p| p.current_price);

            println!("Portfolio Summary");
            println!("{}", "=".repeat(50));
            if let Some(mp) = mark_price {
                println!("  Mark price:          ${:.2}", mp);
            }
            println!(
                "  On-chain balance:    {} sats",
                portfolio.wallet_balance_sats
            );
            println!(
                "  Trading balance:     {} sats",
                portfolio.total_trading_balance
            );
            println!("  Margin used:         {:.2}", portfolio.total_margin_used);
            println!("  Unrealized PnL:      {:.2}", portfolio.unrealized_pnl);
            println!("  Realised PnL:        {:.2}", portfolio.realised_pnl);
            println!("  Liquidation Loss:    {:.2}", portfolio.liquidation_loss);
            println!(
                "  Margin utilization:  {:.2}%",
                portfolio.margin_utilization * 100.0
            );
            println!();
            println!(
                "  Lend deposits:       {:.2}",
                portfolio.total_lend_deposits
            );
            println!("  Lend value:          {:.2}", portfolio.total_lend_value);
            println!("  Lend PnL:            {:.2}", portfolio.lend_pnl);
            println!();
            println!("  Total accounts:      {}", portfolio.total_accounts);
            println!("  On-chain accounts:   {}", portfolio.on_chain_accounts);

            if !portfolio.trader_positions.is_empty() {
                println!("\nTrader Positions");
                println!("{}", "-".repeat(160));
                println!(
                    "  {:<6} {:<10} {:<6} {:>12} {:>16} {:>6} {:>10} {:>12} {:>14} {:>10} {:>12} {:>10} {:>10} {:>10}",
                    "ACCT", "STATUS", "SIDE", "ENTRY", "SIZE", "LEV", "A.MARGIN",
                    "U_PnL", "LIQ PRICE", "FEE", "FUNDING", "LIMIT", "TP", "SL"
                );
                for p in &portfolio.trader_positions {
                    let funding_str = p
                        .funding_applied
                        .map(|v| format!("{:.4}", v))
                        .unwrap_or_else(|| "-".to_string());
                    let limit_str = p
                        .settle_limit
                        .as_ref()
                        .map(|t| format!("{:.2}", t.price))
                        .unwrap_or_else(|| "-".to_string());
                    let tp_str = p
                        .take_profit
                        .as_ref()
                        .map(|t| format!("{:.2}", t.price))
                        .unwrap_or_else(|| "-".to_string());
                    let sl_str = p
                        .stop_loss
                        .as_ref()
                        .map(|t| format!("{:.2}", t.price))
                        .unwrap_or_else(|| "-".to_string());
                    let is_pending = p.order_status == OrderStatus::PENDING;
                    let pnl_str = if is_pending {
                        "-".to_string()
                    } else {
                        format!("{:.2}", p.unrealized_pnl)
                    };
                    let liq_str = if is_pending {
                        "-".to_string()
                    } else {
                        format!("{:.2}", p.liquidation_price)
                    };
                    println!(
                        "  {:<6} {:<10} {:<6} {:>12.2} {:>16.2} {:>5.0}x {:>10.2} {:>12} {:>14} {:>10.4} {:>12} {:>10} {:>10} {:>10}",
                        p.account_index,
                        format!("{:?}", p.order_status),
                        format!("{:?}", p.position_type),
                        p.entry_price,
                        p.position_size,
                        p.leverage,
                        p.available_margin,
                        pnl_str,
                        liq_str,
                        p.fee_filled,
                        funding_str,
                        limit_str,
                        tp_str,
                        sl_str,
                    );
                }
            }

            if !portfolio.closed_trader_positions.is_empty() {
                println!("\nClosed Positions (Settled & Unlocked)");
                println!("{}", "-".repeat(100));
                println!(
                    "  {:<6} {:<6} {:>12} {:>16} {:>6} {:>10} {:>12} {:>12} {:>10} {:>10} {:>10}",
                    "ACCT",
                    "SIDE",
                    "ENTRY",
                    "SIZE",
                    "LEV",
                    "A.MARGIN",
                    "R_PnL",
                    "NET_PnL",
                    "FEE_FILL",
                    "FEE_SETT",
                    "FUNDING"
                );
                for p in &portfolio.closed_trader_positions {
                    let funding_str = p
                        .funding_applied
                        .map(|v| format!("{:.4}", v))
                        .unwrap_or_else(|| "-".to_string());
                    let net_pnl = p.available_margin - p.initial_margin;
                    println!(
                        "  {:<6} {:<6} {:>12.2} {:>16.2} {:>5.0}x {:>10.2} {:>12.2} {:>12.2} {:>10.4} {:>10.4} {:>10}",
                        p.account_index,
                        format!("{:?}", p.position_type),
                        p.entry_price,
                        p.position_size,
                        p.leverage,
                        p.available_margin,
                        p.unrealized_pnl,
                        net_pnl,
                        p.fee_filled,
                        p.fee_settled,
                        funding_str,
                    );
                }
                println!("\n  Total Realised PnL: {:.2}", portfolio.realised_pnl);
            }

            if !portfolio.liquidated_trader_positions.is_empty() {
                println!("\nLiquidated Positions");
                println!("{}", "-".repeat(80));
                println!(
                    "  {:<6} {:<6} {:>12} {:>16} {:>6} {:>12} {:>10} {:>10}",
                    "ACCT", "SIDE", "ENTRY", "SIZE", "LEV", "I.MARGIN", "FEE_FILL", "FEE_SETT"
                );
                for p in &portfolio.liquidated_trader_positions {
                    println!(
                        "  {:<6} {:<6} {:>12.2} {:>16.2} {:>5.0}x {:>12.2} {:>10.4} {:>10.4}",
                        p.account_index,
                        format!("{:?}", p.position_type),
                        p.entry_price,
                        p.position_size,
                        p.leverage,
                        p.initial_margin,
                        p.fee_filled,
                        p.fee_settled,
                    );
                }
                println!(
                    "\n  Total Liquidation Loss: {:.2}",
                    portfolio.liquidation_loss
                );
            }

            if !portfolio.lend_positions.is_empty() {
                println!("\nLend Positions");
                println!("{}", "-".repeat(95));
                println!(
                    "  {:<6} {:>12} {:>12} {:>12} {:>12} {:>10} {:>16}",
                    "ACCT", "DEPOSIT", "VALUE", "PnL", "uPnL", "APR %", "SHARES"
                );
                for p in &portfolio.lend_positions {
                    let upnl_str = p
                        .unrealised_pnl
                        .map(|v| format!("{:.4}", v))
                        .unwrap_or_else(|| "-".to_string());
                    let apr_str = p
                        .apr
                        .map(|v| format!("{:.2}", v))
                        .unwrap_or_else(|| "-".to_string());
                    println!(
                        "  {:<6} {:>12.2} {:>12.2} {:>12.2} {:>12} {:>10} {:>16.4}",
                        p.account_index,
                        p.deposit,
                        p.current_value,
                        p.pnl,
                        upnl_str,
                        apr_str,
                        p.pool_share,
                    );
                }
            }
            Ok(())
        }

        PortfolioCmd::Balances {
            wallet_id,
            password,
            unit,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let (unit_label, divisor): (&str, f64) = match unit.to_lowercase().as_str() {
                "mbtc" => ("mBTC", 100_000.0),
                "btc" => ("BTC", 100_000_000.0),
                _ => ("sats", 1.0),
            };

            let balances = ow.get_account_balances();
            if balances.is_empty() {
                println!("No ZkOS accounts found");
            } else {
                println!(
                    "{:<8} {:<14} {:<10} {:<10}",
                    "INDEX", "BALANCE", "IO-TYPE", "ON-CHAIN"
                );
                println!("{}", "-".repeat(46));
                let mut total: u64 = 0;
                for b in &balances {
                    let display_bal = b.balance as f64 / divisor;
                    if divisor == 1.0 {
                        println!(
                            "{:<8} {:<14} {:<10} {:<10}",
                            b.account_index,
                            b.balance,
                            format!("{:?}", b.io_type),
                            b.on_chain,
                        );
                    } else {
                        println!(
                            "{:<8} {:<14.8} {:<10} {:<10}",
                            b.account_index,
                            display_bal,
                            format!("{:?}", b.io_type),
                            b.on_chain,
                        );
                    }
                    total += b.balance;
                }
                println!("{}", "-".repeat(46));
                let display_total = total as f64 / divisor;
                if divisor == 1.0 {
                    println!(
                        "Total: {} {} across {} accounts",
                        total,
                        unit_label,
                        balances.len()
                    );
                } else {
                    println!(
                        "Total: {:.8} {} across {} accounts",
                        display_total,
                        unit_label,
                        balances.len()
                    );
                }
            }
            Ok(())
        }

        PortfolioCmd::Risks {
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let risks = ow.get_liquidation_risks().await?;
            if risks.is_empty() {
                println!("No open positions with liquidation risk");
            } else {
                println!("Liquidation Risk Report");
                println!("{}", "=".repeat(70));
                println!(
                    "{:<8} {:<8} {:<14} {:<14} {:<14} {:<12}",
                    "ACCT", "SIDE", "CURRENT", "LIQ PRICE", "DISTANCE", "MARGIN %"
                );
                println!("{}", "-".repeat(70));
                for r in &risks {
                    let distance_str = if r.distance_pct >= 0.0 {
                        format!("+{:.2}%", r.distance_pct)
                    } else {
                        format!("{:.2}%", r.distance_pct)
                    };
                    println!(
                        "{:<8} {:<8} ${:<13.2} ${:<13.2} {:<14} {:<12.2}%",
                        r.account_index,
                        format!("{:?}", r.position_type),
                        r.current_price,
                        r.liquidation_price,
                        distance_str,
                        r.margin_ratio * 100.0,
                    );
                }
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Market handlers
// ---------------------------------------------------------------------------

async fn handle_market(cmd: MarketCmd, json_output: bool) -> Result<(), String> {
    use nyks_wallet::relayer_module::relayer_api::RelayerJsonRpcClient;

    let endpoint = std::env::var("RELAYER_API_RPC_SERVER_URL")
        .unwrap_or_else(|_| "http://0.0.0.0:8088/api".to_string());
    let client = RelayerJsonRpcClient::new(&endpoint).map_err(|e| e.to_string())?;

    match cmd {
        MarketCmd::Price => {
            let price = client.btc_usd_price().await.map_err(|e| e.to_string())?;
            println!("BTC/USD Price");
            println!("  Price:     ${:.2}", price.price);
            println!("  Timestamp: {}", price.timestamp);
        }

        MarketCmd::Orderbook => {
            let book = client
                .open_limit_orders()
                .await
                .map_err(|e| e.to_string())?;
            println!("Order Book");
            println!("\n  BIDS (buy):");
            println!("  {:<16} {:<16}", "PRICE", "SIZE");
            for bid in &book.bid {
                println!("  {:<16.2} {:<16.4}", bid.price, bid.positionsize);
            }
            println!("\n  ASKS (sell):");
            println!("  {:<16} {:<16}", "PRICE", "SIZE");
            for ask in &book.ask {
                println!("  {:<16.2} {:<16.4}", ask.price, ask.positionsize);
            }
        }

        MarketCmd::FundingRate => {
            let rate = client.get_funding_rate().await.map_err(|e| e.to_string())?;
            println!("Funding Rate");
            println!("  Rate:      {:.6}%", rate.rate);
            println!("  BTC price: ${:.2}", rate.btc_price);
            println!("  Timestamp: {}", rate.timestamp);
        }

        MarketCmd::FeeRate => {
            let fee = client.get_fee_rate().await.map_err(|e| e.to_string())?;
            println!("Fee Rate");
            println!("  Market fill: {:.6}", fee.order_filled_on_market);
            println!("  Limit fill:  {:.6}", fee.order_filled_on_limit);
            println!("  Market settle: {:.6}", fee.order_settled_on_market);
            println!("  Limit settle:  {:.6}", fee.order_settled_on_limit);
            println!("  Timestamp:   {}", fee.timestamp);
        }

        MarketCmd::RecentTrades => {
            let trades = client
                .recent_trade_orders()
                .await
                .map_err(|e| e.to_string())?;
            println!("Recent Trades");
            println!(
                "  {:<38} {:<8} {:<14} {:<12}",
                "ORDER ID", "SIDE", "PRICE", "SIZE"
            );
            println!("  {}", "-".repeat(76));
            for t in &trades.orders {
                println!(
                    "  {:<38} {:<8} {:<14.2} {:<12.4}",
                    t.order_id,
                    format!("{:?}", t.side),
                    t.price,
                    t.positionsize,
                );
            }
        }

        MarketCmd::PositionSize => {
            let ps = client.position_size().await.map_err(|e| e.to_string())?;
            println!("Position Size");
            println!("  Total long:  {:.4}", ps.total_long_position_size);
            println!("  Total short: {:.4}", ps.total_short_position_size);
            println!("  Total:       {:.4}", ps.total_position_size);
        }

        MarketCmd::LendPool => {
            let info = client.lend_pool_info().await.map_err(|e| e.to_string())?;
            println!("Lend Pool Info");
            println!(
                "{}",
                serde_json::to_string_pretty(&info).unwrap_or_else(|_| format!("{:?}", info))
            );
        }

        MarketCmd::PoolShareValue => {
            let value = client.pool_share_value().await.map_err(|e| e.to_string())?;
            println!("Pool Share Value");
            println!("  Value: {:.8}", value);
        }

        MarketCmd::LastDayApy => {
            let apy = client.last_day_apy().await.map_err(|e| e.to_string())?;
            println!("Last 24h APY");
            match apy {
                Some(v) => println!("  APY: {:.4}%", v),
                None => println!("  APY: not available"),
            }
        }

        MarketCmd::OpenInterest => {
            let oi = client.open_interest().await.map_err(|e| e.to_string())?;
            println!("Open Interest");
            println!("  Long exposure:  {:.4} SATS", oi.long_exposure);
            println!("  Short exposure: {:.4} SATS", oi.short_exposure);
            if let Some(ts) = &oi.last_order_timestamp {
                println!("  Last order:     {}", ts);
            }
        }

        MarketCmd::MarketStats => {
            let stats = client.get_market_stats().await.map_err(|e| e.to_string())?;
            println!("Market Statistics");
            println!("{}", "=".repeat(45));
            println!("  Pool equity:       {:.4} SATS", stats.pool_equity_btc);
            println!("  Total long:        {:.4} SATS", stats.total_long_btc);
            println!("  Total short:       {:.4} SATS", stats.total_short_btc);
            println!(
                "  Pending long:      {:.4} SATS",
                stats.total_pending_long_btc
            );
            println!(
                "  Pending short:     {:.4} SATS",
                stats.total_pending_short_btc
            );
            println!("  Open interest:     {:.4} SATS", stats.open_interest_btc);
            println!("  Net exposure:      {:.4} SATS", stats.net_exposure_btc);
            println!("  Long %:            {:.2}%", stats.long_pct * 100.0);
            println!("  Short %:           {:.2}%", stats.short_pct * 100.0);
            println!("  Utilization:       {:.2}%", stats.utilization * 100.0);
            println!("  Max long:          {:.4} SATS", stats.max_long_btc);
            println!("  Max short:         {:.4} SATS", stats.max_short_btc);
            println!("  Status:            {}", stats.status);
            if let Some(reason) = &stats.status_reason {
                println!("  Status reason:     {}", reason);
            }
            println!("\nRisk Parameters");
            println!("{}", "-".repeat(45));
            println!("  Max OI multiplier: {:.2}", stats.params.max_oi_mult);
            println!("  Max net multiplier:{:.2}", stats.params.max_net_mult);
            println!(
                "  Max position %:    {:.2}%",
                stats.params.max_position_pct * 100.0
            );
            println!(
                "  Min position:      {:.4} BTC",
                stats.params.min_position_btc
            );
            println!("  Max leverage:      {:.0}x", stats.params.max_leverage);
            println!("  MM ratio:          {:.4}", stats.params.mm_ratio);
        }

        MarketCmd::ServerTime => {
            let time = client.server_time().await.map_err(|e| e.to_string())?;
            println!("Server Time");
            println!("  UTC: {}", time);
        }

        MarketCmd::HistoryPrice {
            from,
            to,
            limit,
            offset,
        } => {
            use nyks_wallet::relayer_module::relayer_types::HistoricalPriceArgs;
            let params = HistoricalPriceArgs {
                from: parse_datetime(&from)?,
                to: parse_datetime(&to)?,
                limit,
                offset,
            };
            let prices = client
                .historical_price(params)
                .await
                .map_err(|e| e.to_string())?;
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&prices)
                        .unwrap_or_else(|_| format!("{:?}", prices))
                );
            } else if prices.is_empty() {
                println!("No price data for the given range");
            } else {
                println!("Historical BTC/USD Prices");
                println!("{}", "-".repeat(50));
                println!("  {:<14} {:<30}", "PRICE", "TIMESTAMP");
                for p in &prices {
                    println!("  ${:<13.2} {}", p.price, p.timestamp);
                }
                println!("\n  {} entries", prices.len());
            }
        }

        MarketCmd::Candles {
            interval,
            since,
            limit,
            offset,
        } => {
            use nyks_wallet::relayer_module::relayer_types::Candles;
            let params = Candles {
                interval: parse_interval(&interval)?,
                since: parse_datetime(&since)?,
                limit,
                offset,
            };
            let candles = client
                .candle_data(params)
                .await
                .map_err(|e| e.to_string())?;
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&candles)
                        .unwrap_or_else(|_| format!("{:?}", candles))
                );
            } else if candles.is_empty() {
                println!("No candle data for the given range");
            } else {
                println!("OHLCV Candles ({interval})");
                println!("{}", "-".repeat(100));
                println!(
                    "  {:<24} {:>12} {:>12} {:>12} {:>12} {:>10} {:>6}",
                    "START", "OPEN", "HIGH", "LOW", "CLOSE", "VOLUME", "TRADES"
                );
                for c in &candles {
                    println!(
                        "  {:<24} {:>12.2} {:>12.2} {:>12.2} {:>12.2} {:>10.6} {:>6}",
                        &format!("{}", c.started_at)
                            [..std::cmp::min(24, format!("{}", c.started_at).len())],
                        c.open,
                        c.high,
                        c.low,
                        c.close,
                        c.btc_volume,
                        c.trades,
                    );
                }
                println!("\n  {} candle(s)", candles.len());
            }
        }

        MarketCmd::HistoryFunding {
            from,
            to,
            limit,
            offset,
        } => {
            use nyks_wallet::relayer_module::relayer_types::HistoricalFundingArgs;
            let params = HistoricalFundingArgs {
                from: parse_datetime(&from)?,
                to: parse_datetime(&to)?,
                limit,
                offset,
            };
            let rates = client
                .historical_funding_rate(params)
                .await
                .map_err(|e| e.to_string())?;
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&rates).unwrap_or_else(|_| format!("{:?}", rates))
                );
            } else if rates.is_empty() {
                println!("No funding rate data for the given range");
            } else {
                println!("Historical Funding Rates");
                println!("{}", "-".repeat(60));
                println!("  {:>12} {:>14} {:<30}", "RATE %", "BTC PRICE", "TIMESTAMP");
                for r in &rates {
                    println!("  {:>12.6}% ${:<13.2} {}", r.rate, r.btc_price, r.timestamp);
                }
                println!("\n  {} entries", rates.len());
            }
        }

        MarketCmd::HistoryFees {
            from,
            to,
            limit,
            offset,
        } => {
            use nyks_wallet::relayer_module::relayer_types::HistoricalFeeArgs;
            let params = HistoricalFeeArgs {
                from: parse_datetime(&from)?,
                to: parse_datetime(&to)?,
                limit,
                offset,
            };
            let fees = client
                .historical_fee_rate(params)
                .await
                .map_err(|e| e.to_string())?;
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&fees).unwrap_or_else(|_| format!("{:?}", fees))
                );
            } else if fees.is_empty() {
                println!("No fee rate data for the given range");
            } else {
                println!("Historical Fee Rates");
                println!("{}", "-".repeat(80));
                println!(
                    "  {:>12} {:>12} {:>14} {:>14} {:<24}",
                    "MKT FILL", "LMT FILL", "MKT SETTLE", "LMT SETTLE", "TIMESTAMP"
                );
                for f in &fees {
                    println!(
                        "  {:>12.6} {:>12.6} {:>14.6} {:>14.6} {}",
                        f.order_filled_on_market,
                        f.order_filled_on_limit,
                        f.order_settled_on_market,
                        f.order_settled_on_limit,
                        f.timestamp,
                    );
                }
                println!("\n  {} entries", fees.len());
            }
        }

        MarketCmd::ApyChart {
            range,
            step,
            lookback,
        } => {
            use nyks_wallet::relayer_module::relayer_types::ApyChartArgs;
            let params = ApyChartArgs {
                range,
                step,
                lookback,
            };
            let points = client.apy_chart(params).await.map_err(|e| e.to_string())?;
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&points)
                        .unwrap_or_else(|_| format!("{:?}", points))
                );
            } else if points.is_empty() {
                println!("No APY data available");
            } else {
                println!("Lend Pool APY Chart");
                println!("{}", "-".repeat(50));
                println!("  {:<24} {:>12}", "TIME", "APY %");
                for p in &points {
                    println!(
                        "  {:<24} {:>12.4}%",
                        &p.bucket_ts[..std::cmp::min(24, p.bucket_ts.len())],
                        p.apy,
                    );
                }
                println!("\n  {} data point(s)", points.len());
            }
        }
    }
    Ok(())
}
