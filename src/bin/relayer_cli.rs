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
        /// Load wallet from DB by wallet ID
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password
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

        /// Load wallet from DB by wallet ID
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password
        #[arg(long)]
        password: Option<String>,
    },

    /// List all ZkOS accounts for a wallet
    Accounts {
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
    /// Fund a new ZkOS trading account from the on-chain wallet
    Fund {
        /// Amount in satoshis to fund
        #[arg(long)]
        amount: u64,

        /// Load wallet from DB by wallet ID
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password
        #[arg(long)]
        password: Option<String>,
    },

    /// Withdraw from a ZkOS trading account back to the on-chain wallet
    Withdraw {
        /// ZkOS account index to withdraw from
        #[arg(long)]
        account_index: u64,

        /// Load wallet from DB by wallet ID
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password
        #[arg(long)]
        password: Option<String>,
    },

    /// Transfer between ZkOS trading accounts
    Transfer {
        /// Source account index
        #[arg(long)]
        from: u64,

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

        /// Load wallet from DB by wallet ID
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password
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

        /// Load wallet from DB by wallet ID
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password
        #[arg(long)]
        password: Option<String>,
    },

    /// Cancel a pending trader order
    CancelTrade {
        /// ZkOS account index
        #[arg(long)]
        account_index: u64,

        /// Load wallet from DB by wallet ID
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password
        #[arg(long)]
        password: Option<String>,
    },

    /// Query trader order status
    QueryTrade {
        /// ZkOS account index
        #[arg(long)]
        account_index: u64,

        /// Load wallet from DB by wallet ID
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password
        #[arg(long)]
        password: Option<String>,
    },

    /// Open a lend order
    OpenLend {
        /// ZkOS account index to lend from
        #[arg(long)]
        account_index: u64,

        /// Load wallet from DB by wallet ID
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password
        #[arg(long)]
        password: Option<String>,
    },

    /// Close a lend order
    CloseLend {
        /// ZkOS account index
        #[arg(long)]
        account_index: u64,

        /// Load wallet from DB by wallet ID
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password
        #[arg(long)]
        password: Option<String>,
    },

    /// Query lend order status
    QueryLend {
        /// ZkOS account index
        #[arg(long)]
        account_index: u64,

        /// Load wallet from DB by wallet ID
        #[arg(long)]
        wallet_id: Option<String>,

        /// Database encryption password
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

/// Build an `OrderWallet` either from DB or by creating a new one (caller decides).
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
fn load_order_wallet_from_db(
    wallet_id: &str,
    password: Option<String>,
    db_url: Option<String>,
) -> Result<OrderWallet, String> {
    let pwd = password.map(|p| SecretString::new(p.into()));
    OrderWallet::load_from_db(wallet_id.to_string(), pwd, db_url)
}

/// Resolve an `OrderWallet` – either load from DB (if wallet_id given) or create fresh.
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
async fn resolve_order_wallet(
    wallet_id: Option<String>,
    password: Option<String>,
) -> Result<OrderWallet, String> {
    if let Some(wid) = wallet_id {
        load_order_wallet_from_db(&wid, password, None)
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
                let pwd = password.map(|p| SecretString::new(p.into()));
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
            let mut ow =
                OrderWallet::import_from_mnemonic(&mnemonic, None).map_err(|e| e.to_string())?;
            println!("Wallet imported successfully");
            println!("  Address: {}", ow.wallet.twilightaddress);
            println!("  BTC address: {}", ow.wallet.btc_address);

            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            if with_db {
                let pwd = password.map(|p| SecretString::new(p.into()));
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

            let order = ow.query_trader_order(account_index).await?;
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

            let order = ow.query_lend_order(account_index).await?;
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
