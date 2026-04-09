use clap::Subcommand;

// ---------------------------------------------------------------------------
// Wallet sub-commands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub(crate) enum WalletCmd {
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
pub(crate) enum ZkaccountCmd {
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
pub(crate) enum OrderCmd {
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

    /// Cancel a pending trader order, or cancel stop-loss/take-profit on a filled order
    CancelTrade {
        /// ZkOS account index
        #[arg(long)]
        account_index: u64,

        /// Cancel stop-loss (enables SLTP cancel)
        #[arg(long)]
        stop_loss: bool,

        /// Cancel take-profit (enables SLTP cancel)
        #[arg(long)]
        take_profit: bool,

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

    /// Unlock a settled order (trade or lend) based on account's TXType
    UnlockCloseOrder {
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

    /// Unlock a failed order (reclaim account when order submission failed)
    UnlockFailedOrder {
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

        /// Show reason column in output
        #[arg(long)]
        reason: bool,
    },

    /// Look up transaction hashes for a wallet account by account index
    RequestHistory {
        /// Account index to look up
        #[arg(long)]
        account_index: u64,

        /// Wallet ID for database lookup
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password
        #[arg(long)]
        password: Option<String>,

        /// Filter by order status (e.g. FILLED, SETTLED, PENDING)
        #[arg(long)]
        status: Option<String>,

        /// Maximum number of results
        #[arg(long)]
        limit: Option<i64>,

        /// Offset for pagination
        #[arg(long)]
        offset: Option<i64>,

        /// Show reason column in output
        #[arg(long)]
        reason: bool,
    },
}

// ---------------------------------------------------------------------------
// History sub-commands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub(crate) enum HistoryCmd {
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
pub(crate) enum PortfolioCmd {
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
pub(crate) enum MarketCmd {
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
pub(crate) enum VerifyTestCmd {
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
pub(crate) enum BitcoinWalletCmd {
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

    /// Update the BTC wallet by providing a new mnemonic phrase.
    /// Re-derives BIP-84 keys and updates the wallet's BTC address and keys.
    UpdateBitcoinWallet {
        /// Wallet ID to load from DB (falls back to NYKS_WALLET_ID env var)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (falls back to NYKS_WALLET_PASSPHRASE env var)
        #[arg(long)]
        password: Option<String>,

        /// Mnemonic phrase (if omitted, you will be prompted securely)
        #[arg(long)]
        mnemonic: Option<String>,
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
