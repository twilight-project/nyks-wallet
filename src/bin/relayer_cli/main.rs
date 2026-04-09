use clap::{Parser, Subcommand};

mod bitcoin_wallet;
mod commands;
mod help;
mod helpers;
mod history;
mod market;
mod order;
mod portfolio;
mod verify_test;
mod wallet;
mod zkaccount;

use commands::*;

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
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    let cli = Cli::parse();

    let json_output = cli.json;

    let result = match cli.command {
        Commands::Wallet(cmd) => wallet::handle_wallet(cmd).await,
        Commands::Zkaccount(cmd) => zkaccount::handle_zkaccount(cmd).await,
        Commands::Order(cmd) => order::handle_order(cmd, json_output).await,
        Commands::Market(cmd) => market::handle_market(cmd, json_output).await,
        Commands::History(cmd) => history::handle_history(cmd, json_output).await,
        Commands::Portfolio(cmd) => portfolio::handle_portfolio(cmd, json_output).await,
        Commands::BitcoinWallet(cmd) => bitcoin_wallet::handle_bitcoin_wallet(cmd).await,
        Commands::VerifyTest(cmd) => verify_test::handle_verify_test(cmd).await,
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
