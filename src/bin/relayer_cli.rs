use clap::{Parser, Subcommand};
use log::error;
use nyks_wallet::relayer_module::order_wallet::OrderWallet;
use secrecy::SecretString;

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

/// Twilight Relayer CLI — manage wallets and orders from the command line.
#[derive(Parser)]
#[command(name = "relayer-cli", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Wallet management commands
    #[command(subcommand)]
    Wallet(WalletCmd),

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
}

// ---------------------------------------------------------------------------
// Wallet sub-commands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
enum WalletCmd {
    /// Create a new wallet (prints mnemonic once)
    Create {
        /// Optional wallet ID for database persistence
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password (reads from NYKS_WALLET_PASSPHRASE env if absent)
        #[arg(long)]
        password: Option<String>,

        /// Enable database persistence
        #[arg(long, default_value_t = false)]
        with_db: bool,
    },

    /// Import a wallet from a mnemonic phrase
    Import {
        /// The BIP-39 mnemonic phrase (24 words)
        #[arg(long)]
        mnemonic: String,

        /// Optional wallet ID for database persistence
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password
        #[arg(long)]
        password: Option<String>,

        /// Enable database persistence
        #[arg(long, default_value_t = false)]
        with_db: bool,
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

    /// Unlock: prompt for password once and cache it for this terminal session.
    /// Subsequent commands will use the cached password automatically.
    /// The cache is invalidated when the terminal (shell) is closed.
    Unlock,

    /// Lock: clear the cached session password immediately.
    Lock,
}

// ---------------------------------------------------------------------------
// Order sub-commands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
enum OrderCmd {
    /// Fund a new ZkOS trading account from the on-chain wallet
    Fund {
        /// Amount in satoshis to fund
        #[arg(long)]
        amount: u64,

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

    /// Split a ZkOS trading account into multiple new accounts
    Split {
        /// Source account index
        #[arg(long)]
        from: u64,

        /// Comma-separated list of balances for new accounts (e.g. "1000,2000,3000")
        #[arg(long)]
        balances: String,

        /// Load wallet from DB by wallet ID
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password
        #[arg(long)]
        password: Option<String>,
    },

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
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_order_type(s: &str) -> Result<twilight_client_sdk::relayer_types::OrderType, String> {
    match s.to_uppercase().as_str() {
        "MARKET" => Ok(twilight_client_sdk::relayer_types::OrderType::MARKET),
        "LIMIT" => Ok(twilight_client_sdk::relayer_types::OrderType::LIMIT),
        "SLTP" => Ok(twilight_client_sdk::relayer_types::OrderType::SLTP),
        other => Err(format!("Unknown order type: {other}. Use MARKET, LIMIT, or SLTP")),
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
// value we verify that PID is still alive via /proc – so when the terminal is
// closed and the shell exits, subsequent invocations find the parent dead and
// silently discard the stale file.
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
    Some(std::path::PathBuf::from(home).join(".cache").join("nyks-wallet"))
}

#[cfg(unix)]
fn session_file_path(ppid: u32) -> Option<std::path::PathBuf> {
    Some(session_dir()?.join(format!("session-{ppid}.lock")))
}

/// Save password to session cache, bound to the current shell (PPID).
#[cfg(unix)]
fn session_save(password: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let ppid = get_ppid().ok_or("cannot determine parent shell PID")?;
    let dir = session_dir().ok_or("cannot determine home directory")?;

    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
        .map_err(|e| e.to_string())?;

    let path = session_file_path(ppid).ok_or("cannot build session file path")?;
    let content = format!("{ppid}\n{password}");
    std::fs::write(&path, content.as_bytes()).map_err(|e| e.to_string())?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Load password from session cache; returns None if shell is gone or cache is missing.
#[cfg(unix)]
fn session_load() -> Option<String> {
    let ppid = get_ppid()?;
    if !is_process_alive(ppid) {
        session_clear_for(ppid); // clean up stale file
        return None;
    }
    let path = session_file_path(ppid)?;
    let content = std::fs::read_to_string(&path).ok()?;
    let (stored, password) = content.split_once('\n')?;
    if stored.trim().parse::<u32>().ok()? != ppid {
        return None; // sanity-check: file belongs to this shell
    }
    Some(password.to_string())
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
fn session_save(_password: &str) -> Result<(), String> {
    Err("session cache is only supported on Unix".to_string())
}
#[cfg(not(unix))]
fn session_load() -> Option<String> { None }
#[cfg(not(unix))]
fn session_clear() {}

// ---------------------------------------------------------------------------
// Password / wallet-ID resolution helpers
// ---------------------------------------------------------------------------

/// Resolve password: CLI arg → `NYKS_WALLET_PASSPHRASE` env var → session cache → None.
fn resolve_password(password: Option<String>) -> Option<String> {
    password
        .or_else(|| std::env::var("NYKS_WALLET_PASSPHRASE").ok())
        .or_else(session_load)
}

/// Resolve wallet_id: CLI arg → `NYKS_WALLET_ID` env var → None.
fn resolve_wallet_id(wallet_id: Option<String>) -> Option<String> {
    wallet_id.or_else(|| std::env::var("NYKS_WALLET_ID").ok())
}

/// Resolve an `OrderWallet` – load from DB using wallet_id (arg or env), or create fresh.
///
/// Priority: CLI arg → `NYKS_WALLET_ID` env var → create a new ephemeral wallet.
/// Password priority: CLI arg → `NYKS_WALLET_PASSPHRASE` env var.
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
async fn resolve_order_wallet(
    wallet_id: Option<String>,
    password: Option<String>,
) -> Result<OrderWallet, String> {
    let wid = resolve_wallet_id(wallet_id);
    let pwd = resolve_password(password);
    if let Some(wid) = wid {
        load_order_wallet_from_db(&wid, pwd, None)
    } else {
        OrderWallet::new(None).map_err(|e| e.to_string())
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

    let result = match cli.command {
        Commands::Wallet(cmd) => handle_wallet(cmd).await,
        Commands::Order(cmd) => handle_order(cmd).await,
        Commands::Market(cmd) => handle_market(cmd).await,
        Commands::History(cmd) => handle_history(cmd).await,
        Commands::Portfolio(cmd) => handle_portfolio(cmd).await,
    };

    if let Err(e) = result {
        error!("{}", e);
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Wallet handlers
// ---------------------------------------------------------------------------

async fn handle_wallet(cmd: WalletCmd) -> Result<(), String> {
    match cmd {
        WalletCmd::Create {
            wallet_id,
            password,
            with_db,
        } => {
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;
            println!("Wallet created successfully");
            println!("  Address: {}", ow.wallet.twilightaddress);
            println!("  BTC address: {}", ow.wallet.btc_address);

            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            if with_db {
                let pwd = resolve_password(password).map(|p| SecretString::new(p.into()));
                ow.with_db(pwd, wallet_id.clone())?;
                println!(
                    "  Database persistence enabled (wallet_id: {})",
                    wallet_id.unwrap_or_else(|| ow.wallet.twilightaddress.clone())
                );
            }
            Ok(())
        }

        WalletCmd::Import {
            mnemonic,
            wallet_id,
            password,
            with_db,
        } => {
            let mnemonic = mnemonic.trim().to_string();
            let mut ow =
                OrderWallet::import_from_mnemonic(&mnemonic, None).map_err(|e| e.to_string())?;
            println!("Wallet imported successfully");
            println!("  Address: {}", ow.wallet.twilightaddress);
            println!("  BTC address: {}", ow.wallet.btc_address);

            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            if with_db {
                let pwd = resolve_password(password).map(|p| SecretString::new(p.into()));
                ow.with_db(pwd, wallet_id.clone())?;
                println!(
                    "  Database persistence enabled (wallet_id: {})",
                    wallet_id.unwrap_or_else(|| ow.wallet.twilightaddress.clone())
                );
            }
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
        WalletCmd::Load { .. } => {
            Err("Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite".to_string())
        }

        WalletCmd::Balance {
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let balance = ow.wallet.update_balance().await.map_err(|e| e.to_string())?;
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
        WalletCmd::List { .. } => {
            Err("Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite".to_string())
        }

        WalletCmd::Export {
            output,
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            ow.wallet.export_to_json(&output).map_err(|e| e.to_string())?;
            println!("Wallet exported to {output}");
            Ok(())
        }

        WalletCmd::Accounts {
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let accounts = ow.zk_accounts.get_all_accounts();
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
        WalletCmd::Backup {
            output,
            wallet_id,
            password,
        } => {
            let ow = load_order_wallet_from_db(&wallet_id, password, None)?;
            let db_manager = ow.get_db_manager()
                .ok_or("Database not enabled on this wallet")?;
            db_manager.export_backup_to_file(&output)?;
            println!("Backup exported to {output}");
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::Backup { .. } => {
            Err("Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite".to_string())
        }

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::Restore {
            input,
            wallet_id,
            password,
            force,
        } => {
            let ow = load_order_wallet_from_db(&wallet_id, password, None)?;
            let db_manager = ow.get_db_manager()
                .ok_or("Database not enabled on this wallet")?;
            db_manager.import_backup_from_file(&input, force)?;
            println!("Backup restored from {input}");
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::Restore { .. } => {
            Err("Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite".to_string())
        }

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
            println!("  Released (pending reuse): {}", ow.nonce_manager.released_count());
            Ok(())
        }

        WalletCmd::Unlock => {
            // If a session is already active, ask before overwriting.
            if session_load().is_some() {
                eprintln!("A session password is already cached. Run `wallet lock` first to clear it.");
                return Err("session already active".to_string());
            }
            let password = rpassword::prompt_password("Wallet password: ")
                .map_err(|e| e.to_string())?;
            if password.is_empty() {
                return Err("password must not be empty".to_string());
            }
            session_save(&password)?;
            println!("Password cached for this terminal session.");
            println!("Run `wallet lock` to clear it, or just close the terminal.");
            Ok(())
        }

        WalletCmd::Lock => {
            session_clear();
            println!("Session password cleared.");
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Order handlers
// ---------------------------------------------------------------------------

async fn handle_order(cmd: OrderCmd) -> Result<(), String> {
    match cmd {
        OrderCmd::Fund {
            amount,
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            println!("Funding {amount} sats to new ZkOS trading account...");
            let (tx_result, account_index) = ow.funding_to_trading(amount).await?;
            println!("Funding successful");
            println!("  TX hash: {}", tx_result.tx_hash);
            println!("  TX code: {}", tx_result.code);
            println!("  Account index: {account_index}");
            Ok(())
        }

        OrderCmd::Withdraw {
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

        OrderCmd::Transfer {
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

        OrderCmd::Split {
            from,
            balances,
            wallet_id,
            password,
        } => {
            let balance_vec: Vec<u64> = balances
                .split(',')
                .map(|s| {
                    s.trim()
                        .parse::<u64>()
                        .map_err(|e| format!("Invalid balance '{}': {}", s.trim(), e))
                })
                .collect::<Result<Vec<u64>, String>>()?;

            if balance_vec.is_empty() {
                return Err("At least one balance is required".into());
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
    }
}

// ---------------------------------------------------------------------------
// History handlers
// ---------------------------------------------------------------------------

async fn handle_history(cmd: HistoryCmd) -> Result<(), String> {
    #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
    {
        let _ = cmd;
        return Err("Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite".to_string());
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
                .ok_or("wallet_id is required (pass --wallet-id or set NYKS_WALLET_ID)")?;
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
                .ok_or("wallet_id is required (pass --wallet-id or set NYKS_WALLET_ID)")?;
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

async fn handle_portfolio(cmd: PortfolioCmd) -> Result<(), String> {
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

            println!("Portfolio Summary");
            println!("{}", "=".repeat(50));
            println!("  On-chain balance:    {} sats", portfolio.wallet_balance_sats);
            println!("  Trading balance:     {} sats", portfolio.total_trading_balance);
            println!("  Margin used:         {:.2}", portfolio.total_margin_used);
            println!("  Unrealized PnL:      {:.2}", portfolio.unrealized_pnl);
            println!("  Margin utilization:  {:.2}%", portfolio.margin_utilization * 100.0);
            println!();
            println!("  Lend deposits:       {:.2}", portfolio.total_lend_deposits);
            println!("  Lend value:          {:.2}", portfolio.total_lend_value);
            println!("  Lend PnL:            {:.2}", portfolio.lend_pnl);
            println!();
            println!("  Total accounts:      {}", portfolio.total_accounts);
            println!("  On-chain accounts:   {}", portfolio.on_chain_accounts);

            if !portfolio.trader_positions.is_empty() {
                println!("\nTrader Positions");
                println!("{}", "-".repeat(115));
                println!(
                    "  {:<6} {:<6} {:>12} {:>12} {:>16} {:>6} {:>12} {:>14} {:>10}",
                    "ACCT", "SIDE", "ENTRY", "CURRENT", "SIZE", "LEV", "PnL", "LIQ PRICE", "FUNDING"
                );
                for p in &portfolio.trader_positions {
                    let funding_str = p
                        .funding_applied
                        .map(|v| format!("{:.4}", v))
                        .unwrap_or_else(|| "-".to_string());
                    println!(
                        "  {:<6} {:<6} {:>12.2} {:>12.2} {:>16.2} {:>5.0}x {:>12.2} {:>14.2} {:>10}",
                        p.account_index,
                        format!("{:?}", p.position_type),
                        p.entry_price,
                        p.current_price,
                        p.position_size,
                        p.leverage,
                        p.unrealized_pnl,
                        p.liquidation_price,
                        funding_str,
                    );
                }
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
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

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
                    println!(
                        "{:<8} {:<14} {:<10} {:<10}",
                        b.account_index,
                        b.balance,
                        format!("{:?}", b.io_type),
                        b.on_chain,
                    );
                    total += b.balance;
                }
                println!("{}", "-".repeat(46));
                println!("Total: {} sats across {} accounts", total, balances.len());
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

async fn handle_market(cmd: MarketCmd) -> Result<(), String> {
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
            let book = client.open_limit_orders().await.map_err(|e| e.to_string())?;
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
    }
    Ok(())
}
