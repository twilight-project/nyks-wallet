use clap::{Parser, Subcommand};
// use log::error;
use nyks_wallet::relayer_module::order_wallet::OrderWallet;
use nyks_wallet::relayer_module::relayer_types::OrderStatus;
use secrecy::{ExposeSecret, SecretString};

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

/// Twilight Relayer CLI — manage wallets and orders from the command line.
#[derive(Parser)]
#[command(name = "relayer-cli", version, about, long_about = None, disable_help_subcommand = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output results as JSON instead of formatted tables (useful for scripting)
    #[arg(long, global = true, default_value_t = false)]
    json: bool,
}

#[derive(Subcommand)]
#[command(name = "commands", version, about, long_about = None, disable_help_subcommand = true)]
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

    /// Bitcoin wallet commands (balance, transfer, receive)
    #[command(subcommand)]
    BitcoinWallet(BitcoinWalletCmd),

    /// Run verification tests against testnet (testnet only)
    #[command(subcommand)]
    VerifyTest(VerifyTestCmd),

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

        /// Optional BTC native SegWit address (bc1q...) to use instead of generating a random one
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

        /// Optional BTC native SegWit address (bc1q...) to use instead of deriving from mnemonic
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
        /// Wallet password (prompts interactively if omitted)
        #[arg(long)]
        password: Option<String>,
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

    /// Send tokens (nyks or sats) to another Twilight address
    Send {
        /// Recipient Twilight address
        #[arg(long)]
        to: String,

        /// Amount to send
        #[arg(long)]
        amount: u64,

        /// Token denomination (nyks or sats)
        #[arg(long, default_value = "nyks")]
        denom: String,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Register the wallet's BTC address for deposit on-chain
    RegisterBtc {
        /// Amount in satoshis the user intends to deposit
        #[arg(long)]
        amount: u64,

        /// Twilight staking amount (defaults to 10000)
        #[arg(long, default_value_t = 10000)]
        staking_amount: u64,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Show available BTC reserve addresses (where to send BTC after registration)
    Reserves {
        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Check BTC deposit registration status from chain
    DepositStatus {
        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Submit a BTC withdrawal request to your registered BTC address (mainnet only)
    WithdrawBtc {
        /// Reserve ID to withdraw from (see `wallet reserves`)
        #[arg(long)]
        reserve_id: u64,

        /// Amount in satoshis to withdraw
        #[arg(long)]
        amount: u64,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Check on-chain status of all pending BTC withdrawal requests and update DB (mainnet only)
    WithdrawStatus {
        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Record a BTC deposit after registration — shows the reserve address to pay (mainnet only)
    DepositBtc {
        /// Amount in satoshis to deposit
        #[arg(long)]
        amount: Option<u64>,

        /// Amount in milli-BTC (1 mBTC = 100,000 sats)
        #[arg(long)]
        amount_mbtc: Option<f64>,

        /// Amount in BTC (1 BTC = 100,000,000 sats)
        #[arg(long)]
        amount_btc: Option<f64>,

        /// Reserve address the user will send BTC to (from `wallet reserves`). If omitted, active reserves are listed.
        #[arg(long)]
        reserve_address: Option<String>,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Get test tokens from faucet (testnet only)
    Faucet {
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
        account_index: u64,

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
        account_index: u64,

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
// Verify-test sub-commands (testnet only)
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
enum VerifyTestCmd {
    /// Verify all wallet subcommands
    Wallet,

    /// Verify market data queries
    Market,

    /// Verify ZkOS account commands (requires funded wallet)
    Zkaccount,

    /// Verify order commands (requires funded ZkOS account)
    Order,

    /// Run all verification tests in sequence
    All,
}

// ---------------------------------------------------------------------------
// Bitcoin wallet sub-commands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
enum BitcoinWalletCmd {
    /// Check on-chain Bitcoin balance for a BTC address
    Balance {
        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,

        /// Check an arbitrary BTC address instead of the wallet's own address
        #[arg(long)]
        btc_address: Option<String>,

        /// Display balance in BTC
        #[arg(long, default_value_t = false)]
        btc: bool,

        /// Display balance in mBTC
        #[arg(long, default_value_t = false)]
        mbtc: bool,
    },

    /// Transfer BTC to a native SegWit address.
    /// Provide exactly one of --amount (sats), --amount-mbtc, or --amount-btc.
    Transfer {
        /// Destination BTC address (bc1q.../tb1q...)
        #[arg(long)]
        to: String,

        /// Amount in satoshis (priority 1)
        #[arg(long)]
        amount: Option<u64>,

        /// Amount in milli-BTC (1 mBTC = 100,000 sats, priority 2)
        #[arg(long)]
        amount_mbtc: Option<f64>,

        /// Amount in BTC (1 BTC = 100,000,000 sats, priority 3)
        #[arg(long)]
        amount_btc: Option<f64>,

        /// Fee rate in sat/vB — higher = faster confirmation (if omitted, BDK auto-estimates)
        #[arg(long)]
        fee_rate: Option<f32>,

        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Show BTC receive address and wallet details
    Receive {
        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Show BTC transfer history with confirmation status
    History {
        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,

        /// Filter by status (pending, broadcast, confirmed)
        #[arg(long)]
        status: Option<String>,

        /// Maximum number of results
        #[arg(long, default_value_t = 50)]
        limit: usize,
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
    wallet          Wallet management (create, import, load, list, balance, accounts,
                    export, backup, restore, unlock/lock, change-password, info,
                    update-btc-address, sync-nonce, send, register-btc, deposit-btc,
                    reserves, deposit-status, withdraw-btc, withdraw-status, faucet)
    bitcoin-wallet  On-chain BTC operations (balance, transfer, receive, history)
    zkaccount       ZkOS account operations (fund, withdraw, transfer, split)
    order           Trading and lending orders (open/close/cancel/query trade & lend,
                    unlock-trade, history-trade, history-lend, funding-history,
                    account-summary, tx-hashes)
    market          Market data (price, orderbook, funding-rate, fee-rate, recent-trades,
                    position-size, lend-pool, pool-share-value, last-day-apy,
                    open-interest, market-stats, server-time, history-price,
                    candles, history-funding, history-fees, apy-chart)
    history         Local DB history (orders, transfers)
    portfolio       Portfolio tracking (summary, balances, risks)
    verify-test     Run verification tests against testnet (testnet only)

GLOBAL FLAGS:
    --json      Output results as JSON (for scripting)

RESOLUTION PRIORITY (wallet-id & password):
    --flag  >  session cache (wallet unlock)  >  env var

ENVIRONMENT:
    NYKS_WALLET_ID          Default wallet ID
    NYKS_WALLET_PASSPHRASE  Default password
    BTC_NETWORK_TYPE        Bitcoin network (mainnet/testnet, falls back to mainnet)

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
    send                Send tokens (nyks or sats) to a Twilight address
    register-btc        Register BTC deposit address on-chain (mainnet only)
    deposit-btc         Record a BTC deposit after registration (mainnet only)
    reserves            Show available BTC reserve addresses
    deposit-status      Check BTC deposit & confirmation status (mainnet only)
    withdraw-btc        Submit a BTC withdrawal request (mainnet only)
    withdraw-status     Check pending BTC withdrawal status (mainnet only)
    faucet              Get test tokens from faucet (testnet only)

EXAMPLES:
    relayer-cli wallet create --btc-address bc1q...
    relayer-cli wallet unlock                         # interactive prompt
    relayer-cli wallet balance                        # uses session cache
    relayer-cli wallet accounts --on-chain-only
    relayer-cli wallet send --to twilight1... --amount 1000
    relayer-cli wallet register-btc --amount 50000    # mainnet: register for 50k sats deposit
    relayer-cli wallet deposit-btc --amount 50000 --reserve-address bc1q...
    relayer-cli wallet reserves                       # see where to send BTC
    relayer-cli wallet deposit-status                 # check if confirmed by validators
    relayer-cli wallet withdraw-btc --reserve-id 1 --amount 50000
    relayer-cli wallet withdraw-status                # check pending withdrawals
    relayer-cli wallet faucet                         # testnet only: get test tokens"#
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
    relayer-cli zkaccount transfer --account-index 1
    relayer-cli zkaccount split --account-index 0 --balances "10000,20000,30000""#
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

fn print_bitcoin_wallet_help() {
    println!(
        r#"On-chain Bitcoin operations — check balance, transfer BTC, view receive address, and transfer history.

USAGE:
    relayer-cli bitcoin-wallet <SUBCOMMAND>

SUBCOMMANDS:
    balance     Check on-chain BTC balance (confirmed + unconfirmed)
    transfer    Send BTC to a native SegWit address
    receive     Show BTC receive address and wallet details
    history     Show BTC transfer history with confirmation status

AMOUNTS (transfer):
    --amount <sats>           Satoshis (priority 1)
    --amount-mbtc <mbtc>      Milli-BTC — 1 mBTC = 100,000 sats (priority 2)
    --amount-btc <btc>        BTC — 1 BTC = 100,000,000 sats (priority 3)

DISPLAY UNIT (balance):
    --btc                     Show balance in BTC
    --mbtc                    Show balance in mBTC
    (default: sats)

EXAMPLES:
    relayer-cli bitcoin-wallet balance
    relayer-cli bitcoin-wallet balance --btc
    relayer-cli bitcoin-wallet balance --btc-address bc1q...
    relayer-cli bitcoin-wallet transfer --to bc1q... --amount 50000
    relayer-cli bitcoin-wallet transfer --to bc1q... --amount-mbtc 0.5 --fee-rate 5
    relayer-cli bitcoin-wallet receive
    relayer-cli bitcoin-wallet history
    relayer-cli bitcoin-wallet history --status confirmed"#
    );
}

fn print_verify_test_help() {
    println!(
        r#"Run verification tests against testnet (testnet only).

USAGE:
    relayer-cli verify-test <SUBCOMMAND>

SUBCOMMANDS:
    wallet      Verify wallet commands (create, balance, faucet, send, etc.)
    market      Verify market data queries
    zkaccount   Verify ZkOS account commands (requires funded wallet)
    order       Verify order commands (requires funded ZkOS account)
    all         Run all verification tests in sequence

EXAMPLES:
    NETWORK_TYPE=testnet relayer-cli verify-test all
    NETWORK_TYPE=testnet relayer-cli verify-test wallet
    NETWORK_TYPE=testnet relayer-cli verify-test market"#
    );
}

fn print_subcommand_help(group: &str) {
    match group.to_lowercase().replace('-', "").as_str() {
        "wallet" => print_wallet_help(),
        "bitcoinwallet" => print_bitcoin_wallet_help(),
        "zkaccount" => print_zkaccount_help(),
        "order" => print_order_help(),
        "market" => print_market_help(),
        "history" => print_history_help(),
        "portfolio" => print_portfolio_help(),
        "verifytest" => print_verify_test_help(),
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
        Commands::History(cmd) => handle_history(cmd, json_output).await,
        Commands::Portfolio(cmd) => handle_portfolio(cmd, json_output).await,
        Commands::BitcoinWallet(cmd) => handle_bitcoin_wallet(cmd).await,
        Commands::VerifyTest(cmd) => handle_verify_test(cmd).await,
        Commands::Help { command } => {
            match command {
                Some(group) => print_subcommand_help(&group),
                None => print_global_help(),
            }
            Ok(())
        }
    };

    if let Err(e) = result {
        // error!("{}", e);
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Wallet handlers
// ---------------------------------------------------------------------------

/// Validate that a string is a valid BTC SegWit address (bc1q... or bc1p...) on mainnet.
use nyks_wallet::wallet::btc_wallet::validation::validate_btc_segwit_address;

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

            // Check BTC address registration status on-chain
            if let Some(addr) = btc_address {
                // User provided a custom BTC address — check if it's already linked elsewhere
                match ow.wallet.fetch_registered_btc_by_address(&addr).await {
                    Ok(Some(info)) => {
                        if info.twilight_address != ow.wallet.twilightaddress {
                            return Err(format!(
                                "BTC address {} is already registered to a different twilight address: {}.\n\
                                 You cannot use this BTC address with your wallet ({}).",
                                addr, info.twilight_address, ow.wallet.twilightaddress
                            ));
                        }
                        // Registered to this wallet — set flag
                        ow.wallet.btc_address = addr;
                        ow.wallet.btc_address_registered = true;
                    }
                    Ok(None) => {
                        // Not registered yet — just set the address
                        ow.wallet.btc_address = addr;
                        ow.wallet.btc_address_registered = false;
                    }
                    Err(e) => {
                        eprintln!("Warning: Could not check BTC registration status: {e}");
                        ow.wallet.btc_address = addr;
                        ow.wallet.btc_address_registered = false;
                    }
                }
            } else {
                // No custom BTC address — check if the wallet's default btc_address is registered
                match ow
                    .wallet
                    .fetch_registered_btc_by_address(&ow.wallet.btc_address)
                    .await
                {
                    Ok(Some(info)) => {
                        if info.twilight_address == ow.wallet.twilightaddress {
                            ow.wallet.btc_address_registered = true;
                        } else {
                            return Err(format!(
                                "BTC address {} is already registered to a different twilight address: {}.\n\
                                 You cannot use this BTC address with your wallet ({}). pass a different BTC address with wallet create or import",
                                 &ow.wallet.btc_address, info.twilight_address, ow.wallet.twilightaddress
                            ));
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        eprintln!("Warning: Could not check BTC registration status: {e}");
                    }
                }
            }

            println!("Wallet imported successfully");
            println!("  Address: {}", ow.wallet.twilightaddress);
            println!("  BTC address: {}", ow.wallet.btc_address);
            println!(
                "  BTC registered: {}",
                if ow.wallet.btc_address_registered {
                    "yes"
                } else {
                    "no"
                }
            );

            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            {
                let pwd = resolve_password(password).map(|p| SecretString::new(p.into()));
                ow.with_db(pwd, wallet_id.clone())?;
                println!(
                    "  Wallet ID: {}",
                    wallet_id.unwrap_or_else(|| ow.wallet.twilightaddress.clone())
                );
            }

            if !ow.wallet.btc_address_registered {
                println!();
                println!("Note: If the BTC address above is not the one you use, update it with:");
                println!("  relayer-cli wallet update-btc-address --btc-address <your-bc1q-address> --wallet-id <your_wallet_id>");
            }
            println!();
            println!("Tip: To avoid typing --wallet-id and --password on every command, run:");
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

        WalletCmd::Unlock { wallet_id, password, force } => {
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

            let password = if let Some(p) = password {
                p
            } else if let Ok(p) = std::env::var("NYKS_WALLET_PASSPHRASE") {
                println!("Using password from NYKS_WALLET_PASSPHRASE env var.");
                p
            } else {
                rpassword::prompt_password("Wallet password: ").map_err(|e| e.to_string())?
            };
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

            // Block update if already registered on-chain — each twilight address
            // can only be linked to a single BTC address.
            if ow.wallet.btc_address_registered {
                return Err(format!(
                    "Cannot update BTC address: your current address ({}) is already registered on-chain.\n\
                     Each twilight address can only be linked to a single BTC address.",
                    ow.wallet.btc_address
                ));
            }

            let old_address = ow.wallet.btc_address.clone();
            ow.wallet.btc_address = btc_address.clone();

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
            println!("  Note: Register on-chain with `wallet register-btc` before depositing.");
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::UpdateBtcAddress { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::Send {
            to,
            amount,
            denom,
            wallet_id,
            password,
        } => {
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            let from_addr = ow.wallet.twilightaddress.clone();

            println!("Sending {amount} {denom}");
            println!("  From: {from_addr}");
            println!("  To:   {to}");

            match ow.wallet.send_tokens(&to, amount, &denom).await {
                Ok(tx_hash) => {
                    println!("Transaction successful");
                    println!("  TX Hash: {tx_hash}");
                    Ok(())
                }
                Err(e) => Err(format!("Send failed: {e}")),
            }
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::Send { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::RegisterBtc {
            amount,
            staking_amount,
            wallet_id,
            password,
        } => {
            if nyks_wallet::config::NETWORK_TYPE.as_str() != "mainnet" {
                return Err("register-btc is only available on mainnet. Use `wallet faucet` for testnet tokens.".to_string());
            }
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            let btc_addr = ow.wallet.btc_address.clone();
            let tw_addr = ow.wallet.twilightaddress.clone();

            // 1. Check if BTC address is already registered on-chain
            println!("Checking if BTC address is already registered...");
            match ow.wallet.fetch_registered_btc_by_address(&btc_addr).await {
                Ok(Some(info)) => {
                    if info.twilight_address == tw_addr {
                        println!("BTC address {btc_addr} is already registered to your wallet ({tw_addr}).");
                        println!("Use `wallet deposit-btc --reserve-address <addr>` to record a deposit.");
                        return Ok(());
                    } else {
                        return Err(format!(
                            "BTC address {btc_addr} is already registered to another twilight address: {}",
                            info.twilight_address
                        ));
                    }
                }
                Ok(None) => {
                    println!("BTC address not yet registered. Proceeding...\n");
                }
                Err(e) => {
                    eprintln!("Warning: Could not check registration status: {e}");
                    println!("Proceeding with registration anyway...\n");
                }
            }

            // 2. Check reserves status — warn if all are CRITICAL or EXPIRED
            println!("Checking BTC reserve status...");
            let reserves = ow
                .wallet
                .fetch_btc_reserves()
                .await
                .map_err(|e| format!("Failed to fetch reserves: {e}"))?;

            if reserves.is_empty() {
                return Err(
                    "No BTC reserves found on chain. Cannot register — try again later."
                        .to_string(),
                );
            }

            let btc_height = nyks_wallet::wallet::wallet::fetch_btc_block_height()
                .await
                .unwrap_or(0);

            if btc_height > 0 {
                let has_active = reserves.iter().any(|r| {
                    let blocks_left = if r.unlock_height + 144 > btc_height {
                        r.unlock_height + 144 - btc_height
                    } else {
                        0
                    };
                    blocks_left > 4
                });

                if !has_active {
                    // All reserves are CRITICAL or EXPIRED
                    let any_expired = reserves.iter().any(|r| r.unlock_height + 144 <= btc_height);
                    if any_expired {
                        let max_unlock =
                            reserves.iter().map(|r| r.unlock_height).max().unwrap_or(0);
                        let new_reserve_at = max_unlock + 148;
                        if new_reserve_at > btc_height {
                            let blocks_until = new_reserve_at - btc_height;
                            return Err(format!(
                                "All reserves are expired or critical. A new reserve address will be \
                                 available in ~{blocks_until} BTC blocks (~{} min). Try again later.",
                                blocks_until * 10
                            ));
                        }
                    } else {
                        return Err(
                            "All reserves are in CRITICAL status (less than 4 blocks remaining). \
                             Wait for the next reserve rotation before registering."
                                .to_string(),
                        );
                    }
                }
            }

            // 3. Register on-chain
            println!("Registering BTC deposit address on-chain");
            println!("  Twilight address: {tw_addr}");
            println!("  BTC address:      {btc_addr}");
            println!("  Deposit amount:   {amount} sats");
            println!("  Staking amount:   {staking_amount}");

            match ow.wallet.register_btc_deposit(amount, staking_amount).await {
                Ok(tx_hash) => {
                    println!("\nRegistration submitted successfully");
                    println!("  TX Hash: {tx_hash}");

                    // Persist the updated btc_address_registered flag
                    if let Some(db_manager) = ow.get_db_manager() {
                        if let Some(wallet_password) = ow.get_wallet_password() {
                            let _ = db_manager.save_encrypted_wallet(&ow.wallet, wallet_password);
                        }

                        // Save deposit record to database
                        let now = chrono::Utc::now().naive_utc();
                        let deposit_entry = nyks_wallet::database::models::NewDbBtcDeposit {
                            wallet_id: db_manager.get_wallet_id().to_string(),
                            network_type: nyks_wallet::config::NETWORK_TYPE.to_string(),
                            btc_address: btc_addr.clone(),
                            twilight_address: tw_addr.clone(),
                            reserve_address: None,
                            amount: amount as i64,
                            staking_amount: staking_amount as i64,
                            registration_tx_hash: Some(tx_hash.clone()),
                            status: "registered".to_string(),
                            created_at: now,
                            updated_at: now,
                        };
                        if let Err(e) = db_manager.save_btc_deposit(deposit_entry) {
                            eprintln!("Warning: Could not save deposit to database: {e}");
                        }
                    }

                    // Show active reserves
                    println!("\nActive reserve addresses to send {amount} sats to:");
                    println!(
                        "\n{:<6} {:<50} {:<15} {:<10}",
                        "ID", "RESERVE ADDRESS", "TOTAL VALUE", "STATUS"
                    );
                    println!("{}", "-".repeat(85));
                    for r in &reserves {
                        let blocks_left = if btc_height > 0 && r.unlock_height + 144 > btc_height {
                            r.unlock_height + 144 - btc_height
                        } else {
                            0
                        };
                        let status = if blocks_left > 72 {
                            "ACTIVE"
                        } else if blocks_left > 4 {
                            "WARNING"
                        } else {
                            continue; // skip CRITICAL/EXPIRED
                        };
                        println!(
                            "{:<6} {:<50} {:<15} {:<10}",
                            r.reserve_id, r.reserve_address, r.total_value, status
                        );
                    }

                    println!("\nNext steps:");
                    println!("  1. Pick an ACTIVE reserve address above");
                    println!("  2. Run: wallet deposit-btc --reserve-address <reserve_addr>");
                    println!("  3. Send {amount} sats from your registered BTC address ({btc_addr}) to the reserve");
                    println!("  4. Check status with: wallet deposit-status");
                    Ok(())
                }
                Err(e) => Err(format!("Registration failed: {e}")),
            }
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::RegisterBtc { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::Reserves {
            wallet_id,
            password,
        } => {
            let ow = resolve_order_wallet(wallet_id, password).await?;
            match ow.wallet.fetch_btc_reserves().await {
                Ok(reserves) => {
                    if reserves.is_empty() {
                        println!("No BTC reserves found on chain.");
                    } else {
                        // Fetch current BTC block height for status calculation
                        let btc_height = nyks_wallet::wallet::wallet::fetch_btc_block_height()
                            .await
                            .unwrap_or(0);
                        if btc_height > 0 {
                            println!("Current BTC block height: {btc_height}\n");
                        }

                        println!(
                            "{:<6} {:<50} {:<15} {:<14} {:<10}",
                            "ID", "RESERVE ADDRESS", "TOTAL VALUE", "BLOCKS LEFT", "STATUS"
                        );
                        println!("{}", "-".repeat(98));
                        for r in &reserves {
                            let next_unlock = r.unlock_height + 144;
                            let (blocks_left, status) = if btc_height > 0 {
                                if next_unlock <= btc_height {
                                    ("expired".to_string(), "EXPIRED")
                                } else {
                                    let remaining = next_unlock - btc_height;
                                    let st = if remaining <= 4 {
                                        "CRITICAL"
                                    } else if remaining <= 72 {
                                        "WARNING"
                                    } else {
                                        "ACTIVE"
                                    };
                                    (remaining.to_string(), st)
                                }
                            } else {
                                (format!("unlock:{}", r.unlock_height), "UNKNOWN")
                            };
                            println!(
                                "{:<6} {:<50} {:<15} {:<14} {:<10}",
                                r.reserve_id, r.reserve_address, r.total_value, blocks_left, status
                            );
                        }
                        println!("\nTotal: {} reserve(s)", reserves.len());

                        // Check if any reserves are expired and show new-address ETA
                        if btc_height > 0 {
                            let any_expired =
                                reserves.iter().any(|r| r.unlock_height + 144 <= btc_height);
                            if any_expired {
                                // New reserve address becomes available at unlock_height + 148 (4 blocks after expiry)
                                let max_unlock =
                                    reserves.iter().map(|r| r.unlock_height).max().unwrap_or(0);
                                let new_reserve_at = max_unlock + 148;
                                if new_reserve_at > btc_height {
                                    let blocks_until = new_reserve_at - btc_height;
                                    println!("\nNote: Expired reserves are sweeping. A new reserve address will be");
                                    println!("available in ~{blocks_until} BTC blocks (~{} min) at block {new_reserve_at}.",
                                        blocks_until * 10);
                                } else {
                                    println!("\nNote: New reserve address should already be available. Re-run this command to refresh.");
                                }
                            }
                        }

                        println!("\nSTATUS KEY:");
                        println!("  ACTIVE   - Safe to send BTC");
                        println!("  WARNING  - Less than ~12h remaining, send only if your BTC tx will confirm quickly");
                        println!("  CRITICAL - Less than 4 blocks remaining, do NOT send");
                        println!("  EXPIRED  - Reserve is sweeping, do NOT send (new address available ~4 blocks after expiry)");
                        println!("\nReserve addresses rotate every ~144 BTC blocks (~24 hours).");
                        println!("The reserve must still be ACTIVE when your BTC transaction confirms on Bitcoin.");
                    }
                    Ok(())
                }
                Err(e) => Err(format!("Failed to fetch reserves: {e}")),
            }
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::Reserves { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::DepositStatus {
            wallet_id,
            password,
        } => {
            if nyks_wallet::config::NETWORK_TYPE.as_str() != "mainnet" {
                return Err(
                    "deposit-status is only available on mainnet. Testnet uses faucet for tokens."
                        .to_string(),
                );
            }
            let ow = resolve_order_wallet(wallet_id, password).await?;
            let tw_addr = ow.wallet.twilightaddress.clone();

            println!("Checking deposit & withdrawal status for {tw_addr}...\n");

            // ---- Section 1: On-chain / Indexer data (confirmed transactions) ----
            match ow.wallet.fetch_account_from_indexer().await {
                Ok(info) => {
                    println!("Account: {}", info.address);
                    println!("  Transactions: {}", info.tx_count);
                    if !info.first_seen.is_empty() {
                        println!(
                            "  First seen:   {}",
                            &info.first_seen[..std::cmp::min(19, info.first_seen.len())]
                        );
                    }
                    if !info.last_seen.is_empty() {
                        println!(
                            "  Last seen:    {}",
                            &info.last_seen[..std::cmp::min(19, info.last_seen.len())]
                        );
                    }
                    println!();

                    if !info.balances.is_empty() {
                        println!("Balances:");
                        for b in &info.balances {
                            println!("  {}: {}", b.denom, b.amount);
                        }
                        println!();
                    }

                    // Confirmed Deposits (from indexer)
                    if !info.deposits.is_empty() {
                        println!("Confirmed Deposits ({}):", info.deposits.len());
                        println!(
                            "  {:<6} {:<12} {:<12} {:<10} {:<8} {:<22}",
                            "ID", "AMOUNT", "BTC HEIGHT", "CONFIRMED", "VOTES", "DATE"
                        );
                        println!("  {}", "-".repeat(72));
                        for d in &info.deposits {
                            let date = if d.created_at.len() >= 19 {
                                &d.created_at[..19]
                            } else {
                                &d.created_at
                            };
                            println!(
                                "  {:<6} {:<12} {:<12} {:<10} {:<8} {:<22}",
                                d.id,
                                d.deposit_amount,
                                d.btc_height,
                                if d.confirmed { "YES" } else { "NO" },
                                d.votes,
                                date
                            );
                        }

                        let confirmed = info.deposits.iter().filter(|d| d.confirmed).count();
                        let pending_on_chain = info.deposits.len() - confirmed;
                        let total_deposited: u64 = info
                            .deposits
                            .iter()
                            .filter(|d| d.confirmed)
                            .filter_map(|d| d.deposit_amount.parse::<u64>().ok())
                            .sum();
                        println!("\n  Total confirmed deposits: {total_deposited} sats ({confirmed} confirmed, {pending_on_chain} pending on-chain)");
                    }

                    // Confirmed Withdrawals (from indexer)
                    if !info.withdrawals.is_empty() {
                        println!("\nConfirmed Withdrawals ({}):", info.withdrawals.len());
                        println!(
                            "  {:<6} {:<50} {:<12} {:<10} {:<22}",
                            "ID", "BTC ADDRESS", "AMOUNT", "CONFIRMED", "DATE"
                        );
                        println!("  {}", "-".repeat(102));
                        for w in &info.withdrawals {
                            let date = if w.created_at.len() >= 19 {
                                &w.created_at[..19]
                            } else {
                                &w.created_at
                            };
                            println!(
                                "  {:<6} {:<50} {:<12} {:<10} {:<22}",
                                w.withdraw_identifier,
                                w.withdraw_address,
                                w.withdraw_amount,
                                if w.is_confirmed { "YES" } else { "NO" },
                                date
                            );
                        }

                        let w_confirmed =
                            info.withdrawals.iter().filter(|w| w.is_confirmed).count();
                        let w_pending = info.withdrawals.len() - w_confirmed;
                        let total_withdrawn: u64 = info
                            .withdrawals
                            .iter()
                            .filter(|w| w.is_confirmed)
                            .filter_map(|w| w.withdraw_amount.parse::<u64>().ok())
                            .sum();
                        println!("\n  Total confirmed withdrawals: {total_withdrawn} sats ({w_confirmed} confirmed, {w_pending} pending)");
                    }

                    // Update local DB: mark deposits as confirmed if they appear on the indexer
                    if let Some(db_manager) = ow.get_db_manager() {
                        let local_deposits = db_manager.load_btc_deposits().unwrap_or_default();
                        let confirmed_amounts: std::collections::HashSet<i64> = info
                            .deposits
                            .iter()
                            .filter(|d| d.confirmed)
                            .filter_map(|d| d.deposit_amount.parse::<i64>().ok())
                            .collect();
                        for dep in &local_deposits {
                            if dep.status != "confirmed" && confirmed_amounts.contains(&dep.amount)
                            {
                                if let Some(id) = dep.id {
                                    let _ = db_manager.update_btc_deposit_status(id, "confirmed");
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Could not fetch from indexer: {e}");
                    println!("Showing local database records only.\n");
                }
            }

            // ---- Section 2: Local DB pending deposits (not yet on indexer) ----
            if let Some(db_manager) = ow.get_db_manager() {
                let local_deposits = db_manager.load_btc_deposits().unwrap_or_default();
                let pending_deposits: Vec<_> = local_deposits
                    .iter()
                    .filter(|d| d.status != "confirmed")
                    .collect();

                if !pending_deposits.is_empty() {
                    println!(
                        "\nPending Deposits — local (not yet confirmed on-chain) ({}):",
                        pending_deposits.len()
                    );
                    println!(
                        "  {:<4} {:<50} {:<12} {:<50} {:<10} {:<20}",
                        "ID", "BTC ADDRESS", "AMOUNT", "RESERVE ADDRESS", "STATUS", "DATE"
                    );
                    println!("  {}", "-".repeat(148));
                    for d in &pending_deposits {
                        let date = d.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
                        let reserve = d.reserve_address.as_deref().unwrap_or("-");
                        let status_display = match d.status.as_str() {
                            "registered" => "REGISTERED",
                            "pending" => "PENDING",
                            other => other,
                        };
                        println!(
                            "  {:<4} {:<50} {:<12} {:<50} {:<10} {:<20}",
                            d.id.unwrap_or(0),
                            d.btc_address,
                            d.amount,
                            reserve,
                            status_display,
                            date
                        );
                    }
                    let total_pending: i64 = pending_deposits.iter().map(|d| d.amount).sum();
                    println!(
                        "\n  Total pending: {total_pending} sats ({} deposit(s))",
                        pending_deposits.len()
                    );
                    println!("\n  Pending deposits require:");
                    println!("    1. BTC sent to an active reserve address (run: wallet reserves)");
                    println!("    2. BTC transaction confirmed on Bitcoin (~10 min)");
                    println!("    3. Validator detection and confirmation (can take 1+ hours)");
                } else if local_deposits.is_empty() {
                    println!("\nNo deposit records in local database.");
                    println!("Register with: wallet register-btc --amount <sats>");
                }

                // Local DB pending withdrawals
                let local_withdrawals = db_manager.load_btc_withdrawals().unwrap_or_default();
                let pending_withdrawals: Vec<_> = local_withdrawals
                    .iter()
                    .filter(|w| w.status != "confirmed")
                    .collect();

                if !pending_withdrawals.is_empty() {
                    println!(
                        "\nPending Withdrawals — local (not yet confirmed on-chain) ({}):",
                        pending_withdrawals.len()
                    );
                    println!(
                        "  {:<4} {:<50} {:<8} {:<12} {:<10} {:<20}",
                        "ID", "BTC ADDRESS", "RESERVE", "AMOUNT", "STATUS", "DATE"
                    );
                    println!("  {}", "-".repeat(106));
                    for w in &pending_withdrawals {
                        let date = w.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
                        println!(
                            "  {:<4} {:<50} {:<8} {:<12} {:<10} {:<20}",
                            w.id.unwrap_or(0),
                            w.withdraw_address,
                            w.reserve_id,
                            w.amount,
                            "PENDING",
                            date
                        );
                    }
                    let total_pending_wd: i64 = pending_withdrawals.iter().map(|w| w.amount).sum();
                    println!(
                        "\n  Total pending withdrawals: {total_pending_wd} sats ({} request(s))",
                        pending_withdrawals.len()
                    );
                    println!(
                        "  Run `wallet withdraw-status` to check and update confirmation status."
                    );
                }
            }

            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::DepositStatus { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::WithdrawBtc {
            reserve_id,
            amount,
            wallet_id,
            password,
        } => {
            if nyks_wallet::config::NETWORK_TYPE.as_str() != "mainnet" {
                return Err("withdraw-btc is only available on mainnet.".to_string());
            }

            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            let tw_addr = ow.wallet.twilightaddress.clone();
            let btc_addr = ow.wallet.btc_address.clone();

            // Withdrawals go to the wallet's registered BTC address
            if !ow.wallet.btc_address_registered {
                return Err(format!(
                    "BTC address {} is not registered on-chain. Register first with:\n  \
                     wallet register-btc --amount <sats>",
                    btc_addr
                ));
            }

            println!("Submitting BTC withdrawal request");
            println!("  From:       {tw_addr}");
            println!("  To (BTC):   {btc_addr}");
            println!("  Reserve ID: {reserve_id}");
            println!("  Amount:     {amount} sats");

            match ow.wallet.withdraw_btc(&btc_addr, reserve_id, amount).await {
                Ok(tx_hash) => {
                    println!("\nWithdrawal request submitted successfully");
                    println!("  TX Hash: {tx_hash}");

                    // Save withdrawal record to database
                    if let Some(db_manager) = ow.get_db_manager() {
                        let now = chrono::Utc::now().naive_utc();
                        let withdrawal_entry = nyks_wallet::database::models::NewDbBtcWithdrawal {
                            wallet_id: db_manager.get_wallet_id().to_string(),
                            network_type: nyks_wallet::config::NETWORK_TYPE.to_string(),
                            withdraw_address: btc_addr.clone(),
                            twilight_address: tw_addr.clone(),
                            reserve_id: reserve_id as i64,
                            amount: amount as i64,
                            tx_hash: Some(tx_hash.clone()),
                            status: "submitted".to_string(),
                            created_at: now,
                            updated_at: now,
                        };
                        if let Err(e) = db_manager.save_btc_withdrawal(withdrawal_entry) {
                            eprintln!("Warning: Could not save withdrawal to database: {e}");
                        }
                    }

                    println!("\nThe withdrawal will be processed by validators.");
                    println!("Check status with: wallet withdraw-status");
                    Ok(())
                }
                Err(e) => Err(format!("Withdrawal failed: {e}")),
            }
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::WithdrawBtc { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::WithdrawStatus {
            wallet_id,
            password,
        } => {
            if nyks_wallet::config::NETWORK_TYPE.as_str() != "mainnet" {
                return Err("withdraw-status is only available on mainnet.".to_string());
            }
            let ow = resolve_order_wallet(wallet_id, password).await?;

            let db_manager = ow.get_db_manager().ok_or("No database manager available")?;

            // Load all withdrawals (not just pending) for display
            let all_withdrawals = db_manager
                .load_btc_withdrawals()
                .map_err(|e| format!("Failed to load withdrawals: {e}"))?;

            if all_withdrawals.is_empty() {
                println!("No BTC withdrawal requests found in database.");
                println!("Submit one with: wallet withdraw-btc --to <btc_addr> --reserve-id <id> --amount <sats>");
                return Ok(());
            }

            // Check pending withdrawals against on-chain status
            let pending: Vec<_> = all_withdrawals
                .iter()
                .filter(|w| w.status == "submitted")
                .collect();

            if !pending.is_empty() {
                println!(
                    "Checking {} pending withdrawal(s) against on-chain status...\n",
                    pending.len()
                );
                let mut updated_count = 0;
                for w in &pending {
                    let amount: u64 = w.amount as u64;
                    match ow
                        .wallet
                        .fetch_withdrawal_status(w.reserve_id as u64, &w.withdraw_address, amount)
                        .await
                    {
                        Ok(Some(status)) => {
                            if status.is_confirmed {
                                // Update DB to confirmed
                                if let Some(id) = w.id {
                                    let _ =
                                        db_manager.update_btc_withdrawal_status(id, "confirmed");
                                    updated_count += 1;
                                    println!(
                                        "  Updated withdrawal #{} ({} sats to {}) -> CONFIRMED",
                                        status.withdraw_identifier, w.amount, w.withdraw_address
                                    );
                                }
                            }
                        }
                        Ok(None) => {
                            // Not found on chain yet — might still be processing
                        }
                        Err(e) => {
                            eprintln!(
                                "  Warning: Could not check withdrawal {} sats to {}: {e}",
                                w.amount, w.withdraw_address
                            );
                        }
                    }
                }
                if updated_count > 0 {
                    println!("\n{updated_count} withdrawal(s) confirmed on-chain.\n");
                } else {
                    println!("No new confirmations found.\n");
                }
            }

            // Reload and display all withdrawals (with updated statuses)
            let withdrawals = db_manager
                .load_btc_withdrawals()
                .map_err(|e| format!("Failed to reload withdrawals: {e}"))?;

            println!("BTC Withdrawals ({}):", withdrawals.len());
            println!(
                "  {:<4} {:<50} {:<8} {:<12} {:<10} {:<20}",
                "ID", "BTC ADDRESS", "RESERVE", "AMOUNT", "STATUS", "DATE"
            );
            println!("  {}", "-".repeat(106));
            for w in &withdrawals {
                let date = w.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
                let status_display = match w.status.as_str() {
                    "confirmed" => "CONFIRMED",
                    "submitted" => "PENDING",
                    other => other,
                };
                println!(
                    "  {:<4} {:<50} {:<8} {:<12} {:<10} {:<20}",
                    w.id.unwrap_or(0),
                    w.withdraw_address,
                    w.reserve_id,
                    w.amount,
                    status_display,
                    date
                );
            }

            let confirmed_count = withdrawals
                .iter()
                .filter(|w| w.status == "confirmed")
                .count();
            let pending_count = withdrawals
                .iter()
                .filter(|w| w.status == "submitted")
                .count();
            let total_confirmed: i64 = withdrawals
                .iter()
                .filter(|w| w.status == "confirmed")
                .map(|w| w.amount)
                .sum();
            println!(
                "\n  Total: {} confirmed ({total_confirmed} sats), {} pending",
                confirmed_count, pending_count
            );
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::WithdrawStatus { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::DepositBtc {
            amount,
            amount_mbtc,
            amount_btc,
            reserve_address,
            wallet_id,
            password,
        } => {
            if nyks_wallet::config::NETWORK_TYPE.as_str() != "mainnet" {
                return Err(
                    "deposit-btc is only available on mainnet. Use `wallet faucet` for testnet tokens."
                        .to_string(),
                );
            }

            // Resolve amount (at least one is required)
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

            let ow = resolve_order_wallet(wallet_id, password).await?;
            let btc_addr = ow.wallet.btc_address.clone();
            let tw_addr = ow.wallet.twilightaddress.clone();

            // 1. Check if BTC address is registered on-chain
            println!("Checking BTC registration status...");
            let registration = match ow.wallet.fetch_registered_btc_by_address(&btc_addr).await {
                Ok(Some(info)) => {
                    if info.twilight_address != tw_addr {
                        return Err(format!(
                            "BTC address {btc_addr} is registered to a different twilight address: {}. \
                             You cannot deposit from this address.",
                            info.twilight_address
                        ));
                    }
                    info
                }
                Ok(None) => {
                    return Err(format!(
                        "BTC address {btc_addr} is not registered on-chain.\n\
                         Register first with: wallet register-btc --amount <sats>"
                    ));
                }
                Err(e) => {
                    return Err(format!("Failed to check registration: {e}"));
                }
            };

            println!("BTC address is registered:");
            println!("  BTC address:      {}", registration.btc_deposit_address);
            println!("  Twilight address: {}", registration.twilight_address);
            println!("  Deposit amount:   {amount_sats} sats");

            // 2. Show reserve addresses (or the one user chose)
            let reserves = ow
                .wallet
                .fetch_btc_reserves()
                .await
                .map_err(|e| format!("Failed to fetch reserves: {e}"))?;

            let btc_height = nyks_wallet::wallet::wallet::fetch_btc_block_height()
                .await
                .unwrap_or(0);

            if let Some(ref chosen_reserve) = reserve_address {
                // Validate the chosen reserve exists and is active
                let found = reserves
                    .iter()
                    .find(|r| r.reserve_address == *chosen_reserve);
                match found {
                    Some(r) => {
                        let blocks_left = if btc_height > 0 && r.unlock_height + 144 > btc_height {
                            r.unlock_height + 144 - btc_height
                        } else {
                            0
                        };
                        if blocks_left <= 4 {
                            return Err(format!(
                                "Reserve {} is CRITICAL or EXPIRED. Pick a different reserve with more time remaining.",
                                chosen_reserve
                            ));
                        }
                        let status = if blocks_left > 72 {
                            "ACTIVE"
                        } else {
                            "WARNING"
                        };
                        println!("\nSelected reserve:");
                        println!("  Address:       {chosen_reserve}");
                        println!("  Reserve ID:    {}", r.reserve_id);
                        println!(
                            "  Status:        {status} (~{blocks_left} blocks / ~{} min remaining)",
                            blocks_left * 10
                        );

                        // Save deposit to DB
                        if let Some(db_manager) = ow.get_db_manager() {
                            let now = chrono::Utc::now().naive_utc();
                            let deposit_entry = nyks_wallet::database::models::NewDbBtcDeposit {
                                wallet_id: db_manager.get_wallet_id().to_string(),
                                network_type: nyks_wallet::config::NETWORK_TYPE.to_string(),
                                btc_address: btc_addr.clone(),
                                twilight_address: tw_addr.clone(),
                                reserve_address: Some(chosen_reserve.clone()),
                                amount: amount_sats as i64,
                                staking_amount: 0,
                                registration_tx_hash: None,
                                status: "pending".to_string(),
                                created_at: now,
                                updated_at: now,
                            };
                            if let Err(e) = db_manager.save_btc_deposit(deposit_entry) {
                                eprintln!("Warning: Could not save deposit to database: {e}");
                            } else {
                                println!("\nDeposit recorded in database (status: pending).");
                            }
                        }

                        println!(
                            "\nSend {amount_sats} sats from your registered BTC address ONLY:"
                        );
                        println!("  From: {btc_addr}");
                        println!("  To:   {chosen_reserve}");
                        println!(
                            "\nIMPORTANT: You MUST send from {btc_addr} (the registered address)."
                        );
                        println!(
                            "Sending from any other address will NOT be credited to your account."
                        );
                        println!("\nAfter sending, check status with: wallet deposit-status");
                    }
                    None => {
                        return Err(format!(
                            "Reserve address {chosen_reserve} not found. Run `wallet reserves` to see available reserves."
                        ));
                    }
                }
            } else {
                // No reserve specified — save deposit intent and show all active reserves
                if reserves.is_empty() {
                    return Err("No BTC reserves found on chain.".to_string());
                }

                // Save deposit intent to DB (no reserve yet)
                if let Some(db_manager) = ow.get_db_manager() {
                    let now = chrono::Utc::now().naive_utc();
                    let deposit_entry = nyks_wallet::database::models::NewDbBtcDeposit {
                        wallet_id: db_manager.get_wallet_id().to_string(),
                        network_type: nyks_wallet::config::NETWORK_TYPE.to_string(),
                        btc_address: btc_addr.clone(),
                        twilight_address: tw_addr.clone(),
                        reserve_address: None,
                        amount: amount_sats as i64,
                        staking_amount: 0,
                        registration_tx_hash: None,
                        status: "pending".to_string(),
                        created_at: now,
                        updated_at: now,
                    };
                    if let Err(e) = db_manager.save_btc_deposit(deposit_entry) {
                        eprintln!("Warning: Could not save deposit to database: {e}");
                    } else {
                        println!(
                            "\nDeposit intent recorded ({amount_sats} sats, status: pending)."
                        );
                    }
                }

                println!("\nActive reserve addresses — pick one to send {amount_sats} sats to:");
                println!(
                    "\n{:<6} {:<50} {:<15} {:<10} {:<12}",
                    "ID", "RESERVE ADDRESS", "TOTAL VALUE", "STATUS", "BLOCKS LEFT"
                );
                println!("{}", "-".repeat(95));
                let mut any_active = false;
                for r in &reserves {
                    let blocks_left = if btc_height > 0 && r.unlock_height + 144 > btc_height {
                        r.unlock_height + 144 - btc_height
                    } else {
                        0
                    };
                    if blocks_left <= 4 {
                        continue; // skip CRITICAL/EXPIRED
                    }
                    any_active = true;
                    let status = if blocks_left > 72 {
                        "ACTIVE"
                    } else {
                        "WARNING"
                    };
                    println!(
                        "{:<6} {:<50} {:<15} {:<10} {:<12}",
                        r.reserve_id, r.reserve_address, r.total_value, status, blocks_left
                    );
                }

                if !any_active {
                    println!("  No active reserves available. Wait for reserve rotation.");
                    return Ok(());
                }

                println!("\nSend {amount_sats} sats from {btc_addr} to any ACTIVE reserve above.");
                println!("IMPORTANT: Send ONLY from your registered BTC address ({btc_addr}).");
                println!("\nAfter sending, check status with: wallet deposit-status");
            }
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::DepositBtc { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::Faucet {
            wallet_id,
            password,
        } => {
            if nyks_wallet::config::NETWORK_TYPE.as_str() == "mainnet" {
                return Err("faucet is only available on testnet. Use `wallet register-btc` for mainnet deposits.".to_string());
            }
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            let tw_addr = ow.wallet.twilightaddress.clone();

            println!("Requesting test tokens for {tw_addr}...");
            nyks_wallet::wallet::wallet::get_test_tokens(&mut ow.wallet)
                .await
                .map_err(|e| format!("Failed to get test tokens: {e}"))?;

            let balance = ow
                .wallet
                .update_balance()
                .await
                .map_err(|e| e.to_string())?;
            println!("\nUpdated balance:");
            println!("  NYKS: {}", balance.nyks);
            println!("  SATS: {}", balance.sats);
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::Faucet { .. } => Err(
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
            account_index,
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            println!("Transferring from ZkOS account {account_index} to new account...");
            let new_index = ow.trading_to_trading(account_index).await?;
            println!("Transfer successful");
            println!("  New account index: {new_index}");
            Ok(())
        }

        ZkaccountCmd::Split {
            account_index,
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
                account_index,
                balance_vec.len(),
                total
            );
            let results = ow
                .trading_to_trading_multiple_accounts(account_index, balance_vec)
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

            if !json_output {
                println!(
                    "Opening {side} {order_type} order on account {account_index} (price={entry_price}, leverage={leverage}x)..."
                );
            }
            let request_id = ow
                .open_trader_order(account_index, ot, ps, entry_price, leverage)
                .await?;
            if json_output {
                println!("{}", serde_json::json!({"request_id": request_id}));
            } else {
                println!("Order submitted successfully");
                println!("  Request ID: {request_id}");
            }
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

            if !json_output {
                println!("Closing trader order on account {account_index}...");
            }

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

            if json_output {
                println!("{}", serde_json::json!({"request_id": request_id}));
            } else {
                println!("Order closed successfully");
                println!("  Request ID: {request_id}");
            }
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

            if !json_output {
                println!("Cancelling trader order on account {account_index}...");
            }
            let request_id = ow.cancel_trader_order(account_index).await?;
            if json_output {
                println!("{}", serde_json::json!({"request_id": request_id}));
            } else {
                println!("Order cancelled successfully");
                println!("  Request ID: {request_id}");
            }
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
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&order).map_err(|e| e.to_string())?
                );
            } else {
                println!("Trader Order (account {account_index})");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&order).unwrap_or_else(|_| format!("{:?}", order))
                );
            }
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

            if !json_output {
                println!("Opening lend order on account {account_index}...");
            }
            let request_id = ow.open_lend_order(account_index).await?;
            if json_output {
                println!("{}", serde_json::json!({"request_id": request_id}));
            } else {
                println!("Lend order submitted successfully");
                println!("  Request ID: {request_id}");
            }
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

            if !json_output {
                println!("Closing lend order on account {account_index}...");
            }
            let request_id = ow.close_lend_order(account_index).await?;
            if json_output {
                println!("{}", serde_json::json!({"request_id": request_id}));
            } else {
                println!("Lend order closed successfully");
                println!("  Request ID: {request_id}");
            }
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
            if json_output {
                println!(
                    "{}",
                    serde_json::json!({"account_index": account_index, "status": format!("{:?}", status)})
                );
            } else {
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
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&order).map_err(|e| e.to_string())?
                );
            } else {
                println!("Lend Order (account {account_index})");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&order).unwrap_or_else(|_| format!("{:?}", order))
                );
            }
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

async fn handle_history(cmd: HistoryCmd, json_output: bool) -> Result<(), String> {
    #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
    {
        let _ = (cmd, json_output);
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

            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?
                );
                return Ok(());
            }

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

            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?
                );
                return Ok(());
            }

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

async fn handle_portfolio(cmd: PortfolioCmd, json_output: bool) -> Result<(), String> {
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

            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&portfolio).map_err(|e| e.to_string())?
                );
                return Ok(());
            }

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

            let balances = ow.get_account_balances();

            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&balances).map_err(|e| e.to_string())?
                );
                return Ok(());
            }

            let (unit_label, divisor): (&str, f64) = match unit.to_lowercase().as_str() {
                "mbtc" => ("mBTC", 100_000.0),
                "btc" => ("BTC", 100_000_000.0),
                _ => ("sats", 1.0),
            };

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

            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&risks).map_err(|e| e.to_string())?
                );
                return Ok(());
            }

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
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&price).map_err(|e| e.to_string())?
                );
            } else {
                println!("BTC/USD Price");
                println!("  Price:     ${:.2}", price.price);
                println!("  Timestamp: {}", price.timestamp);
            }
        }

        MarketCmd::Orderbook => {
            let book = client
                .open_limit_orders()
                .await
                .map_err(|e| e.to_string())?;
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&book).map_err(|e| e.to_string())?
                );
            } else {
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
        }

        MarketCmd::FundingRate => {
            let rate = client.get_funding_rate().await.map_err(|e| e.to_string())?;
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&rate).map_err(|e| e.to_string())?
                );
            } else {
                println!("Funding Rate");
                println!("  Rate:      {:.6}%", rate.rate);
                println!("  BTC price: ${:.2}", rate.btc_price);
                println!("  Timestamp: {}", rate.timestamp);
            }
        }

        MarketCmd::FeeRate => {
            let fee = client.get_fee_rate().await.map_err(|e| e.to_string())?;
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&fee).map_err(|e| e.to_string())?
                );
            } else {
                println!("Fee Rate");
                println!("  Market fill: {:.6}", fee.order_filled_on_market);
                println!("  Limit fill:  {:.6}", fee.order_filled_on_limit);
                println!("  Market settle: {:.6}", fee.order_settled_on_market);
                println!("  Limit settle:  {:.6}", fee.order_settled_on_limit);
                println!("  Timestamp:   {}", fee.timestamp);
            }
        }

        MarketCmd::RecentTrades => {
            let trades = client
                .recent_trade_orders()
                .await
                .map_err(|e| e.to_string())?;
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&trades).map_err(|e| e.to_string())?
                );
            } else {
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
        }

        MarketCmd::PositionSize => {
            let ps = client.position_size().await.map_err(|e| e.to_string())?;
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&ps).map_err(|e| e.to_string())?
                );
            } else {
                println!("Position Size");
                println!("  Total long:  {:.4}", ps.total_long_position_size);
                println!("  Total short: {:.4}", ps.total_short_position_size);
                println!("  Total:       {:.4}", ps.total_position_size);
            }
        }

        MarketCmd::LendPool => {
            let info = client.lend_pool_info().await.map_err(|e| e.to_string())?;
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&info).map_err(|e| e.to_string())?
                );
            } else {
                println!("Lend Pool Info");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&info).unwrap_or_else(|_| format!("{:?}", info))
                );
            }
        }

        MarketCmd::PoolShareValue => {
            let value = client.pool_share_value().await.map_err(|e| e.to_string())?;
            if json_output {
                println!("{}", serde_json::json!({"value": value}));
            } else {
                println!("Pool Share Value");
                println!("  Value: {:.8}", value);
            }
        }

        MarketCmd::LastDayApy => {
            let apy = client.last_day_apy().await.map_err(|e| e.to_string())?;
            if json_output {
                println!("{}", serde_json::json!({"apy": apy}));
            } else {
                println!("Last 24h APY");
                match apy {
                    Some(v) => println!("  APY: {:.4}%", v),
                    None => println!("  APY: not available"),
                }
            }
        }

        MarketCmd::OpenInterest => {
            let oi = client.open_interest().await.map_err(|e| e.to_string())?;
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&oi).map_err(|e| e.to_string())?
                );
            } else {
                println!("Open Interest");
                println!("  Long exposure:  {:.4} SATS", oi.long_exposure);
                println!("  Short exposure: {:.4} SATS", oi.short_exposure);
                if let Some(ts) = &oi.last_order_timestamp {
                    println!("  Last order:     {}", ts);
                }
            }
        }

        MarketCmd::MarketStats => {
            let stats = client.get_market_stats().await.map_err(|e| e.to_string())?;
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&stats).map_err(|e| e.to_string())?
                );
                return Ok(());
            }
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
            if json_output {
                println!("{}", serde_json::json!({"utc": time.to_string()}));
            } else {
                println!("Server Time");
                println!("  UTC: {}", time);
            }
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

// ---------------------------------------------------------------------------
// Verify-test handler (testnet only)
// ---------------------------------------------------------------------------

/// Print a step header for verify-test output.
fn vt_step(step: u32, total: u32, desc: &str) {
    println!("\n[{step}/{total}] {desc}");
    println!("{}", "-".repeat(60));
}

/// Print PASS / FAIL result for a verify-test step.
fn vt_result(name: &str, result: &Result<(), String>) {
    match result {
        Ok(()) => println!("  PASS: {name}"),
        Err(e) => println!("  FAIL: {name} -> {e}"),
    }
}

async fn handle_verify_test(cmd: VerifyTestCmd) -> Result<(), String> {
    if nyks_wallet::config::NETWORK_TYPE.as_str() == "mainnet" {
        return Err(
            "verify-test is only available on testnet. Set NETWORK_TYPE=testnet to use this command."
                .to_string(),
        );
    }

    println!("==========================================================");
    println!("  VERIFY-TEST  (testnet)");
    println!("  Network: {}", nyks_wallet::config::NETWORK_TYPE.as_str());
    println!("==========================================================");

    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut skipped = 0u32;

    match cmd {
        VerifyTestCmd::Wallet => {
            verify_wallet(&mut passed, &mut failed, &mut skipped).await;
        }
        VerifyTestCmd::Market => {
            verify_market(&mut passed, &mut failed, &mut skipped).await;
        }
        VerifyTestCmd::Zkaccount => {
            verify_zkaccount(&mut passed, &mut failed, &mut skipped).await;
        }
        VerifyTestCmd::Order => {
            verify_order(&mut passed, &mut failed, &mut skipped).await;
        }
        VerifyTestCmd::All => {
            verify_wallet(&mut passed, &mut failed, &mut skipped).await;
            verify_market(&mut passed, &mut failed, &mut skipped).await;
            verify_zkaccount(&mut passed, &mut failed, &mut skipped).await;
            verify_order(&mut passed, &mut failed, &mut skipped).await;
        }
    }

    let total = passed + failed + skipped;
    println!("\n==========================================================");
    println!("  RESULTS: {total} tests — {passed} passed, {failed} failed, {skipped} skipped");
    println!("==========================================================");

    if failed > 0 {
        Err(format!("{failed} test(s) failed"))
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// verify-test: wallet
// ---------------------------------------------------------------------------

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
async fn verify_wallet(passed: &mut u32, failed: &mut u32, _skipped: &mut u32) {
    use secrecy::SecretString;

    let total_steps = 10;
    let test_wallet_id = format!("verify-test-{}", chrono::Utc::now().timestamp());
    let test_password = "verify-test-password";

    println!("\n## Wallet Verification");
    println!("  Test wallet ID: {test_wallet_id}");

    // 1. Create wallet
    vt_step(1, total_steps, "wallet create");
    let create_result = (|| -> Result<OrderWallet, String> {
        let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;
        let pwd = Some(SecretString::new(test_password.into()));
        ow.with_db(pwd, Some(test_wallet_id.clone()))?;
        Ok(ow)
    })();
    let mut ow = match create_result {
        Ok(ow) => {
            println!("  PASS: wallet create");
            println!("    Address: {}", ow.wallet.twilightaddress);
            println!("    BTC addr: {}", ow.wallet.btc_address);
            *passed += 1;
            ow
        }
        Err(e) => {
            println!("  FAIL: wallet create -> {e}");
            *failed += 1;
            println!("  Cannot continue wallet tests without a wallet.");
            return;
        }
    };

    // 2. Wallet info (read-only)
    vt_step(2, total_steps, "wallet info");
    let info_result: Result<(), String> = {
        let addr = &ow.wallet.twilightaddress;
        let btc = &ow.wallet.btc_address;
        if addr.starts_with("twilight1") && !btc.is_empty() {
            println!("    Twilight address format: OK");
            println!("    BTC address present: OK");
            Ok(())
        } else {
            Err(format!("Unexpected address format: {addr} / {btc}"))
        }
    };
    vt_result("wallet info", &info_result);
    if info_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }

    // 3. Faucet (get test tokens)
    vt_step(3, total_steps, "wallet faucet (get test tokens)");
    let faucet_result = nyks_wallet::wallet::wallet::get_test_tokens(&mut ow.wallet)
        .await
        .map_err(|e| format!("{e}"));
    vt_result("wallet faucet", &faucet_result);
    if faucet_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }

    // 4. Balance check
    vt_step(4, total_steps, "wallet balance");
    let balance_result = ow.wallet.update_balance().await.map_err(|e| format!("{e}"));
    match &balance_result {
        Ok(bal) => {
            println!("  PASS: wallet balance");
            println!("    NYKS: {}", bal.nyks);
            println!("    SATS: {}", bal.sats);
            *passed += 1;
        }
        Err(e) => {
            println!("  FAIL: wallet balance -> {e}");
            *failed += 1;
        }
    }

    // 5. Export wallet to JSON
    vt_step(5, total_steps, "wallet export");
    let export_path = format!("/tmp/verify-test-{}.json", chrono::Utc::now().timestamp());
    let export_result = ow
        .wallet
        .export_to_json(&export_path)
        .map(|_| {
            println!("    Exported to: {export_path}");
            let _ = std::fs::remove_file(&export_path); // cleanup
        })
        .map_err(|e| format!("{e}"));
    vt_result("wallet export", &export_result);
    if export_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }

    // 6. Wallet accounts (ZkOS account list)
    vt_step(6, total_steps, "wallet accounts");
    {
        let count = ow.zk_accounts.accounts.len();
        println!("  PASS: wallet accounts ({count} ZkOS accounts)");
        *passed += 1;
    }

    // 7. Reserves query
    vt_step(7, total_steps, "wallet reserves");
    let reserves_result = ow
        .wallet
        .fetch_btc_reserves()
        .await
        .map_err(|e| format!("{e}"));
    match &reserves_result {
        Ok(reserves) => {
            println!(
                "  PASS: wallet reserves ({} reserves found)",
                reserves.len()
            );
            *passed += 1;
        }
        Err(e) => {
            println!("  FAIL: wallet reserves -> {e}");
            *failed += 1;
        }
    }

    // 8. Send tokens (small amount to self)
    vt_step(8, total_steps, "wallet send (1 nyks to self)");
    let self_addr = ow.wallet.twilightaddress.clone();
    let send_result = ow
        .wallet
        .send_tokens(&self_addr, 1, "nyks")
        .await
        .map(|tx_hash| {
            println!("    TX hash: {tx_hash}");
        })
        .map_err(|e| format!("{e}"));
    vt_result("wallet send", &send_result);
    if send_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }

    // 9. Backup/restore
    vt_step(9, total_steps, "wallet backup");
    let backup_result = if let Some(db_manager) = ow.get_db_manager() {
        db_manager.export_backup_json().map(|json| {
            println!("    Backup JSON length: {} bytes", json.len());
        })
    } else {
        Err("No DB manager".to_string())
    };
    vt_result("wallet backup", &backup_result);
    if backup_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }

    // 10. Sync nonce
    vt_step(10, total_steps, "wallet sync-nonce");
    let nonce_result = ow
        .wallet
        .update_account_info()
        .await
        .map(|_| {
            println!("    Nonce synced OK");
        })
        .map_err(|e| format!("{e}"));
    vt_result("wallet sync-nonce", &nonce_result);
    if nonce_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }
}

#[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
async fn verify_wallet(passed: &mut u32, failed: &mut u32, _skipped: &mut u32) {
    println!("\n## Wallet Verification");
    println!("  FAIL: Database features not enabled. Rebuild with --features sqlite");
    *failed += 1;
}

// ---------------------------------------------------------------------------
// verify-test: market
// ---------------------------------------------------------------------------

async fn verify_market(passed: &mut u32, failed: &mut u32, _skipped: &mut u32) {
    use nyks_wallet::relayer_module::relayer_api::RelayerJsonRpcClient;

    let total_steps = 5;
    println!("\n## Market Verification");

    let endpoint = nyks_wallet::config::RELAYER_API_RPC_SERVER_URL.to_string();
    let client = match RelayerJsonRpcClient::new(&endpoint) {
        Ok(c) => c,
        Err(e) => {
            println!("  FAIL: Cannot create RelayerJsonRpcClient: {e}");
            *failed += 1;
            return;
        }
    };
    println!("  Endpoint: {endpoint}");

    // 1. BTC/USD price
    vt_step(1, total_steps, "market price");
    let price_result = client
        .btc_usd_price()
        .await
        .map(|p| {
            println!("    BTC/USD: ${:.2}", p.price);
        })
        .map_err(|e| format!("{e}"));
    vt_result("market price", &price_result);
    if price_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }

    // 2. Orderbook
    vt_step(2, total_steps, "market orderbook");
    let book_result = client
        .open_limit_orders()
        .await
        .map(|book| {
            let count = book.bid.len() + book.ask.len();
            println!(
                "    Orders: {count} (bid: {}, ask: {})",
                book.bid.len(),
                book.ask.len()
            );
        })
        .map_err(|e| format!("{e}"));
    vt_result("market orderbook", &book_result);
    if book_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }

    // 3. Funding rate
    vt_step(3, total_steps, "market funding-rate");
    let fund_result = client
        .get_funding_rate()
        .await
        .map(|f| {
            println!("    Funding rate: {:.6}%", f.rate);
        })
        .map_err(|e| format!("{e}"));
    vt_result("market funding-rate", &fund_result);
    if fund_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }

    // 4. Lend pool info
    vt_step(4, total_steps, "market lend-pool-info");
    let pool_result = client
        .lend_pool_info()
        .await
        .map(|pool| {
            println!("    Lend pool share: {:.2}", pool.total_pool_share);
        })
        .map_err(|e| format!("{e}"));
    vt_result("market lend-pool-info", &pool_result);
    if pool_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }

    // 5. Server time
    vt_step(5, total_steps, "market server-time");
    let time_result = client
        .server_time()
        .await
        .map(|t| {
            println!("    Server time: {t}");
        })
        .map_err(|e| format!("{e}"));
    vt_result("market server-time", &time_result);
    if time_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }
}

// ---------------------------------------------------------------------------
// verify-test: zkaccount
// ---------------------------------------------------------------------------

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
async fn verify_zkaccount(passed: &mut u32, failed: &mut u32, skipped: &mut u32) {
    use secrecy::SecretString;

    let total_steps = 3;
    let test_wallet_id = format!("verify-zk-{}", chrono::Utc::now().timestamp());
    let test_password = "verify-test-password";

    println!("\n## ZkAccount Verification");
    println!("  Test wallet ID: {test_wallet_id}");

    // Setup: create wallet and get test tokens
    println!("\n  [setup] Creating wallet and funding via faucet...");
    let setup_result = async {
        let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;
        let pwd = Some(SecretString::new(test_password.into()));
        ow.with_db(pwd, Some(test_wallet_id.clone()))?;
        nyks_wallet::wallet::wallet::get_test_tokens(&mut ow.wallet)
            .await
            .map_err(|e| format!("{e}"))?;
        let bal = ow
            .wallet
            .update_balance()
            .await
            .map_err(|e| format!("{e}"))?;
        println!("    Wallet funded: {} sats", bal.sats);
        if bal.sats < 10000 {
            return Err("Insufficient sats for zkaccount tests (need >= 10000)".to_string());
        }
        Ok(ow)
    }
    .await;

    let mut ow = match setup_result {
        Ok(ow) => ow,
        Err(e) => {
            println!("  FAIL: zkaccount setup -> {e}");
            *failed += 1;
            *skipped += total_steps as u32;
            return;
        }
    };

    // 1. Fund ZkOS account
    vt_step(1, total_steps, "zkaccount fund (10000 sats)");
    let fund_result = ow.funding_to_trading(10000).await;
    let account_index = match &fund_result {
        Ok((tx, idx)) => {
            println!("  PASS: zkaccount fund");
            println!("    TX hash: {}", tx.tx_hash);
            println!("    Account index: {idx}");
            *passed += 1;
            *idx
        }
        Err(e) => {
            println!("  FAIL: zkaccount fund -> {e}");
            *failed += 1;
            *skipped += 2;
            println!("  Skipping remaining zkaccount tests.");
            return;
        }
    };

    // 2. Query ZkOS account
    vt_step(2, total_steps, "zkaccount query");
    let query_result: Result<(), String> = {
        match ow.zk_accounts.accounts.get(&account_index) {
            Some(acct) => {
                println!("  PASS: zkaccount query");
                println!("    Index: {}", acct.index);
                println!("    Balance: {}", acct.balance);
                println!("    On-chain: {}", acct.on_chain);
                *passed += 1;
                Ok(())
            }
            None => {
                let err = format!("Account index {account_index} not found");
                println!("  FAIL: zkaccount query -> {err}");
                *failed += 1;
                Err(err)
            }
        }
    };

    // 3. Withdraw ZkOS account back to on-chain
    vt_step(3, total_steps, "zkaccount withdraw");
    if query_result.is_ok() {
        let withdraw_result = ow.trading_to_funding(account_index).await;
        match &withdraw_result {
            Ok(()) => {
                println!("  PASS: zkaccount withdraw (trading_to_funding)");
                *passed += 1;
            }
            Err(e) => {
                println!("  FAIL: zkaccount withdraw -> {e}");
                *failed += 1;
            }
        }
    } else {
        println!("  SKIP: zkaccount withdraw (no account to withdraw)");
        *skipped += 1;
    }
}

#[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
async fn verify_zkaccount(passed: &mut u32, failed: &mut u32, _skipped: &mut u32) {
    println!("\n## ZkAccount Verification");
    println!("  FAIL: Database features not enabled. Rebuild with --features sqlite");
    *failed += 1;
}

// ---------------------------------------------------------------------------
// verify-test: order
// ---------------------------------------------------------------------------

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
async fn verify_order(passed: &mut u32, failed: &mut u32, skipped: &mut u32) {
    use secrecy::SecretString;
    use twilight_client_sdk::relayer_types::{OrderType, PositionType};

    let total_steps = 3;
    let test_wallet_id = format!("verify-order-{}", chrono::Utc::now().timestamp());
    let test_password = "verify-test-password";

    println!("\n## Order Verification");
    println!("  Test wallet ID: {test_wallet_id}");

    // Setup: create wallet, fund, create ZkOS account
    println!("\n  [setup] Creating wallet, funding, and creating ZkOS account...");
    let setup_result = async {
        let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;
        let pwd = Some(SecretString::new(test_password.into()));
        ow.with_db(pwd, Some(test_wallet_id.clone()))?;
        nyks_wallet::wallet::wallet::get_test_tokens(&mut ow.wallet)
            .await
            .map_err(|e| format!("{e}"))?;
        let bal = ow
            .wallet
            .update_balance()
            .await
            .map_err(|e| format!("{e}"))?;
        println!("    Wallet funded: {} sats", bal.sats);
        if bal.sats < 20000 {
            return Err("Insufficient sats for order tests (need >= 20000)".to_string());
        }
        let (_, idx) = ow.funding_to_trading(100).await?;
        println!("    ZkOS account created: index {idx}");
        Ok((ow, idx))
    }
    .await;

    let (mut ow, account_index) = match setup_result {
        Ok(v) => v,
        Err(e) => {
            println!("  FAIL: order setup -> {e}");
            *failed += 1;
            *skipped += total_steps as u32;
            return;
        }
    };

    // 1. Open a MARKET LONG order
    vt_step(1, total_steps, "order open-trade (MARKET LONG)");
    let open_result = ow
        .open_trader_order(
            account_index,
            OrderType::MARKET,
            PositionType::LONG,
            75000,
            2,
        )
        .await;
    match &open_result {
        Ok(request_id) => {
            println!("  PASS: order open-trade");
            println!("    Request ID: {request_id}");
            *passed += 1;
        }
        Err(e) => {
            println!("  FAIL: order open-trade -> {e}");
            *failed += 1;
            *skipped += 2;
            println!("  Skipping remaining order tests.");
            return;
        }
    }

    // 2. Query the order
    vt_step(2, total_steps, "order query-trade");
    // Brief wait for order to be processed
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    let query_result = ow.query_trader_order(account_index).await;
    match &query_result {
        Ok(order) => {
            println!("  PASS: order query-trade");
            println!("    Status: {:?}", order.order_status);
            *passed += 1;
        }
        Err(e) => {
            println!("  FAIL: order query-trade -> {e}");
            *failed += 1;
        }
    }

    // 3. Close the order
    vt_step(3, total_steps, "order close-trade");
    let close_result = ow
        .close_trader_order(account_index, OrderType::MARKET, 0.0)
        .await;
    match &close_result {
        Ok(request_id) => {
            println!("  PASS: order close-trade");
            println!("    Request ID: {request_id}");
            *passed += 1;
        }
        Err(e) => {
            println!("  FAIL: order close-trade -> {e}");
            *failed += 1;
        }
    }
}

#[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
async fn verify_order(passed: &mut u32, failed: &mut u32, _skipped: &mut u32) {
    println!("\n## Order Verification");
    println!("  FAIL: Database features not enabled. Rebuild with --features sqlite");
    *failed += 1;
}

// ---------------------------------------------------------------------------
// Bitcoin wallet handler
// ---------------------------------------------------------------------------

async fn handle_bitcoin_wallet(cmd: BitcoinWalletCmd) -> Result<(), String> {
    match cmd {
        BitcoinWalletCmd::Balance {
            wallet_id,
            password,
            btc_address,
            btc,
            mbtc,
        } => {
            let address = if let Some(addr) = btc_address {
                addr
            } else {
                #[cfg(any(feature = "sqlite", feature = "postgresql"))]
                let ow = resolve_order_wallet(wallet_id, password).await?;
                #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
                let ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

                if ow.wallet.btc_address.is_empty() {
                    return Err("Wallet has no BTC address configured".to_string());
                }
                ow.wallet.btc_address.clone()
            };

            validate_btc_segwit_address(&address)?;

            let network = if nyks_wallet::config::is_btc_mainnet() {
                "mainnet"
            } else {
                "testnet"
            };
            println!("Querying Bitcoin balance for: {}", address);
            println!("Network: {}", network);

            let bal = nyks_wallet::wallet::wallet::fetch_btc_balance(&address)
                .await
                .map_err(|e| e.to_string())?;

            if btc {
                let confirmed = bal.confirmed_sats as f64 / 100_000_000.0;
                let unconfirmed = bal.unconfirmed_sats as f64 / 100_000_000.0;
                let total = bal.total_sats as f64 / 100_000_000.0;
                println!("\nConfirmed:   {:.8} BTC", confirmed);
                println!("Unconfirmed: {:.8} BTC", unconfirmed);
                println!("Total:       {:.8} BTC", total);
            } else if mbtc {
                let confirmed = bal.confirmed_sats as f64 / 100_000.0;
                let unconfirmed = bal.unconfirmed_sats as f64 / 100_000.0;
                let total = bal.total_sats as f64 / 100_000.0;
                println!("\nConfirmed:   {:.5} mBTC", confirmed);
                println!("Unconfirmed: {:.5} mBTC", unconfirmed);
                println!("Total:       {:.5} mBTC", total);
            } else {
                println!("\nConfirmed:   {} sats", bal.confirmed_sats);
                println!("Unconfirmed: {} sats", bal.unconfirmed_sats);
                println!("Total:       {} sats", bal.total_sats);
            }

            Ok(())
        }

        BitcoinWalletCmd::Transfer {
            to,
            amount,
            amount_mbtc,
            amount_btc,
            fee_rate,
            wallet_id,
            password,
        } => {
            // Resolve amount: --amount > --amount-mbtc > --amount-btc
            let provided = [
                amount.is_some(),
                amount_mbtc.is_some(),
                amount_btc.is_some(),
            ]
            .iter()
            .filter(|&&v| v)
            .count();

            if provided == 0 {
                return Err(
                    "No amount specified. Provide one of:\n  \
                     --amount <sats>          Amount in satoshis\n  \
                     --amount-mbtc <mbtc>     Amount in milli-BTC (1 mBTC = 100,000 sats)\n  \
                     --amount-btc <btc>       Amount in BTC (1 BTC = 100,000,000 sats)"
                        .to_string(),
                );
            }
            if provided > 1 {
                eprintln!(
                    "Warning: Multiple amount flags provided. Using priority: --amount > --amount-mbtc > --amount-btc"
                );
            }

            let amount_sats = if let Some(sats) = amount {
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

            // Load wallet and extract BtcWallet
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let btc_wallet = ow.wallet.btc_wallet.as_ref().ok_or_else(|| {
                "BTC wallet not available. The wallet was created from a private key, \
                 not a mnemonic. Re-create or import the wallet using a mnemonic to \
                 enable BTC transfers.\n  \
                 Path: OrderWallet -> Wallet -> btc_wallet (None)"
                    .to_string()
            })?;

            // Validate destination is native SegWit
            validate_btc_segwit_address(&to)?;

            let network = if nyks_wallet::config::is_btc_mainnet() {
                "mainnet"
            } else {
                "testnet"
            };
            println!("Bitcoin Transfer");
            println!("  From:    {}", btc_wallet.address);
            println!("  To:      {}", to);
            println!("  Amount:  {} sats", amount_sats);
            println!("  Network: {}", network);
            if let Some(rate) = fee_rate {
                println!("  Fee rate: {} sat/vB (higher = faster confirmation)", rate);
            } else {
                println!("  Fee rate: auto (estimated from current mempool)");
            }
            println!();

            let from_addr = btc_wallet.address.clone();
            let to_addr = to.clone();

            let params = nyks_wallet::wallet::btc_wallet::send::SendBtcParams {
                btc_wallet,
                destination: to,
                amount_sats: amount_sats,
                fee_rate_sat_vb: fee_rate,
            };

            let result = nyks_wallet::wallet::btc_wallet::send::send_btc(params)
                .await
                .map_err(|e| e.to_string())?;

            println!("Transaction broadcast successfully!");
            println!("  TX ID: {}", result.txid);
            println!("  Fee:   {} sats", result.fee_sats);

            let explorer = if nyks_wallet::config::is_btc_mainnet() {
                format!("https://blockstream.info/tx/{}", result.txid)
            } else {
                format!("https://blockstream.info/testnet/tx/{}", result.txid)
            };
            println!("  Explorer: {}", explorer);

            // Save transfer to DB
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            if let Some(db) = ow.get_db_manager() {
                let record = nyks_wallet::database::models::NewDbBtcTransfer {
                    wallet_id: db.get_wallet_id().to_string(),
                    network_type: nyks_wallet::config::BTC_NETWORK_TYPE.clone(),
                    from_address: from_addr,
                    to_address: to_addr,
                    amount: amount_sats as i64,
                    fee: result.fee_sats as i64,
                    tx_id: Some(result.txid.clone()),
                    status: "broadcast".to_string(),
                    confirmations: 0,
                    created_at: chrono::Utc::now().naive_utc(),
                    updated_at: chrono::Utc::now().naive_utc(),
                };
                if let Err(e) = db.save_btc_transfer(record) {
                    eprintln!("Warning: Failed to save transfer to DB: {}", e);
                }
            }

            Ok(())
        }

        BitcoinWalletCmd::Receive {
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let network = if nyks_wallet::config::is_btc_mainnet() {
                "mainnet"
            } else {
                "testnet"
            };
            let address_type = if ow.wallet.btc_address.starts_with("bc1q")
                || ow.wallet.btc_address.starts_with("tb1q")
            {
                "Native SegWit (P2WPKH)"
            } else if ow.wallet.btc_address.starts_with("bc1p")
                || ow.wallet.btc_address.starts_with("tb1p")
            {
                "Taproot (P2TR)"
            } else {
                "Unknown"
            };

            println!("Bitcoin Receive Address");
            println!("{}", "-".repeat(50));
            println!("  Address:      {}", ow.wallet.btc_address);
            println!("  Network:      {}", network);
            println!("  Address type: {}", address_type);
            println!("  Registered:   {}", ow.wallet.btc_address_registered);

            if let Some(ref _btc_wallet) = ow.wallet.btc_wallet {
                println!("  BTC wallet:   available (keys loaded)");
                println!("  Derivation:   {}", nyks_wallet::wallet::btc_wallet::BTC_DERIVATION_PATH);
            } else {
                println!("  BTC wallet:   not available (created from private key)");
            }

            println!("\nSend BTC to this address to deposit into your wallet.");
            if !ow.wallet.btc_address_registered {
                println!("Note: Register this address on-chain first with `wallet register-btc`.");
            }

            Ok(())
        }

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        BitcoinWalletCmd::History {
            wallet_id,
            password,
            status,
            limit,
        } => {
            let ow = resolve_order_wallet(wallet_id, password).await?;
            let db = ow.get_db_manager().ok_or(
                "Database not available. Rebuild with --features sqlite".to_string(),
            )?;

            let transfers = if let Some(ref s) = status {
                db.load_btc_transfers_by_status(s)?
            } else {
                db.load_btc_transfers()?
            };

            if transfers.is_empty() {
                println!("No BTC transfers found.");
                if let Some(ref s) = status {
                    println!("  (filtered by status: {})", s);
                }
                return Ok(());
            }

            let display: Vec<_> = transfers.iter().take(limit).collect();

            println!(
                "{:<5} {:<12} {:<44} {:<44} {:<12} {:<8} {:<10} {:<6} {:<20}",
                "ID", "STATUS", "FROM", "TO", "AMOUNT", "FEE", "CONFIRMS", "NET", "DATE"
            );
            println!("{}", "-".repeat(160));

            for t in &display {
                println!(
                    "{:<5} {:<12} {:<44} {:<44} {:<12} {:<8} {:<10} {:<6} {:<20}",
                    t.id.unwrap_or(0),
                    t.status,
                    t.from_address,
                    t.to_address,
                    t.amount,
                    t.fee,
                    t.confirmations,
                    t.network_type,
                    t.created_at.format("%Y-%m-%d %H:%M"),
                );
            }

            let total_sent: i64 = transfers.iter().map(|t| t.amount).sum();
            let total_fees: i64 = transfers.iter().map(|t| t.fee).sum();
            let confirmed = transfers.iter().filter(|t| t.status == "confirmed").count();
            let pending = transfers.len() - confirmed;

            println!("{}", "-".repeat(160));
            println!(
                "Total: {} transfers ({} confirmed, {} pending) | {} sats sent | {} sats fees",
                transfers.len(),
                confirmed,
                pending,
                total_sent,
                total_fees,
            );

            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        BitcoinWalletCmd::History { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),
    }
}
