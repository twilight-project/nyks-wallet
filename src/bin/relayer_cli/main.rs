use clap::{Parser, Subcommand};

mod bitcoin_wallet;
mod commands;
mod help;
mod helpers;
mod history;
mod market;
mod order;
mod portfolio;
mod repl;
mod update;
mod verify_test;
mod wallet;
mod zkaccount;

use commands::*;

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

const VERSION_INFO: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("BUILD_DATE"),
    ")\nhttps://github.com/twilight-project/nyks-wallet/releases/tag/v",
    env!("CARGO_PKG_VERSION"),
    "-relayer-cli"
);

/// Twilight Relayer CLI — manage wallets and orders from the command line.
#[derive(Parser)]
#[command(name = "relayer-cli", version = VERSION_INFO, about, long_about = None, disable_help_subcommand = true)]
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

    /// Start interactive REPL mode — enter wallet ID and password once, then
    /// run commands without the `relayer-cli` prefix or repeated credentials.
    Repl {
        /// Wallet ID to use for the REPL session (prompts if omitted)
        #[arg(long)]
        wallet_id: Option<String>,

        /// Wallet password (prompts securely if omitted)
        #[arg(long)]
        password: Option<String>,
    },

    /// Check for updates and self-update the binary
    Update {
        /// Check only — show available version without downloading
        #[arg(long, default_value_t = false)]
        check: bool,
    },

    /// Show help for a command group (e.g. `help wallet`)
    Help {
        /// Command group to get help for (wallet, zkaccount, order, market, history, portfolio)
        command: Option<String>,
    },
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
        Commands::Wallet(cmd) => wallet::handle_wallet(cmd, None).await,
        Commands::Zkaccount(cmd) => zkaccount::handle_zkaccount(cmd, None).await,
        Commands::Order(cmd) => order::handle_order(cmd, json_output, None).await,
        Commands::Market(cmd) => market::handle_market(cmd, json_output).await,
        Commands::History(cmd) => history::handle_history(cmd, json_output, None).await,
        Commands::Portfolio(cmd) => portfolio::handle_portfolio(cmd, json_output, None).await,
        Commands::BitcoinWallet(cmd) => bitcoin_wallet::handle_bitcoin_wallet(cmd, None).await,
        Commands::VerifyTest(cmd) => verify_test::handle_verify_test(cmd).await,
        Commands::Update { check } => update::handle_update(check).await,
        Commands::Repl { wallet_id, password } => {
            repl::run_repl(wallet_id, password).await
        }
        Commands::Help { command } => {
            match command {
                Some(group) => help::print_subcommand_help(&group),
                None => help::print_global_help(),
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
