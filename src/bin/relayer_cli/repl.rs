use clap::{Parser, Subcommand};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::commands::*;
use crate::helpers::resolve_password;

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::helpers::load_order_wallet_from_db;

// ---------------------------------------------------------------------------
// REPL input parser — reuses the same Commands enum as the CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "",
    no_binary_name = true,
    disable_help_flag = true,
    disable_version_flag = true
)]
struct ReplInput {
    #[command(subcommand)]
    command: ReplCommands,

    /// Output results as JSON
    #[arg(long, global = true, default_value_t = false)]
    json: bool,
}

/// Wrapper enum that includes all CLI commands plus REPL built-ins.
#[derive(Subcommand)]
enum ReplCommands {
    #[command(subcommand)]
    Wallet(WalletCmd),
    #[command(subcommand)]
    Zkaccount(ZkaccountCmd),
    #[command(subcommand)]
    Order(OrderCmd),
    #[command(subcommand)]
    Market(MarketCmd),
    #[command(subcommand)]
    History(HistoryCmd),
    #[command(subcommand)]
    Portfolio(PortfolioCmd),
    #[command(subcommand)]
    BitcoinWallet(BitcoinWalletCmd),
    #[command(subcommand)]
    VerifyTest(VerifyTestCmd),
    /// Show help
    Help {
        command: Option<String>,
    },
    /// Reload the wallet from the database (pick up external changes)
    Reload,
    /// Exit the REPL
    Exit,
    /// Exit the REPL
    Quit,
}

// ---------------------------------------------------------------------------
// REPL entry point
// ---------------------------------------------------------------------------

pub(crate) async fn run_repl(
    wallet_id: Option<String>,
    password: Option<String>,
) -> Result<(), String> {
    // 1. Resolve wallet ID
    let wallet_id = match wallet_id {
        Some(id) => id,
        None => {
            // Try session cache, then env var, then prompt
            if let Some(id) = crate::helpers::resolve_wallet_id(None) {
                println!("Using wallet from session/env: {}", id);
                id
            } else {
                // List available wallets before prompting
                #[cfg(any(feature = "sqlite", feature = "postgresql"))]
                {
                    use nyks_wallet::relayer_module::order_wallet::OrderWallet;
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
                        _ => {}
                    }
                }
                let mut input = String::new();
                eprint!("Wallet ID: ");
                std::io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| e.to_string())?;
                let input = input.trim().to_string();
                if input.is_empty() {
                    return Err("wallet_id is required".to_string());
                }
                input
            }
        }
    };

    // 2. Resolve password
    let password = match password {
        Some(p) => p,
        None => match resolve_password(None) {
            Some(p) => {
                println!("Using password from session/env.");
                p
            }
            None => {
                let p =
                    rpassword::prompt_password("Wallet password: ").map_err(|e| e.to_string())?;
                if p.is_empty() {
                    return Err("password is required".to_string());
                }
                p
            }
        },
    };

    // 3. Load wallet from DB
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    let mut ow = load_order_wallet_from_db(&wallet_id, Some(password.clone()), None)
        .map_err(|e| format!("Failed to load wallet: {e}"))?;

    #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
    return Err(
        "REPL mode requires database features. Rebuild with --features sqlite".to_string(),
    );

    // 4. Print welcome
    #[cfg(any(feature = "sqlite", feature = "postgresql"))]
    {
        let short_addr = if ow.wallet.twilightaddress.len() > 20 {
            format!(
                "{}...{}",
                &ow.wallet.twilightaddress[..10],
                &ow.wallet.twilightaddress[ow.wallet.twilightaddress.len() - 6..]
            )
        } else {
            ow.wallet.twilightaddress.clone()
        };

        println!();
        println!("Relayer REPL — interactive mode");
        println!("  Wallet ID: {}", wallet_id);
        println!("  Address:   {}", ow.wallet.twilightaddress);
        println!("  Accounts:  {}", ow.zk_accounts.accounts.len());
        println!();
        println!("Type commands without the `relayer-cli` prefix.");
        println!("Examples: wallet balance, order query-trade --account-index 0, market price");
        println!("Type `help` for command list, `exit` or `quit` to leave.");
        println!();

        // 5. REPL loop
        let prompt = format!("{}> ", short_addr);
        let mut rl = DefaultEditor::new().map_err(|e| format!("Failed to init line editor: {e}"))?;

        // Try to load history from a file (ignore errors — file may not exist yet)
        let history_path = dirs_history_path();
        let _ = rl.load_history(&history_path);

        loop {
            // Read line with full editing support (arrows, history, etc.)
            let line = match rl.readline(&prompt) {
                Ok(line) => line,
                Err(ReadlineError::Interrupted) => {
                    // Ctrl+C — cancel current line, keep looping
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    // Ctrl+D
                    println!();
                    break;
                }
                Err(e) => {
                    eprintln!("Read error: {e}");
                    break;
                }
            };

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Add to history
            let _ = rl.add_history_entry(line);

            // Handle bare built-in aliases
            match line.to_lowercase().as_str() {
                "exit" | "quit" | "q" => break,
                "clear" | "cls" => {
                    // ANSI clear screen
                    print!("\x1b[2J\x1b[H");
                    continue;
                }
                _ => {}
            }

            // Parse the input line as a command
            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }

            let parsed = match ReplInput::try_parse_from(tokens.iter()) {
                Ok(p) => p,
                Err(e) => {
                    // Clap's error message already includes suggestions
                    eprintln!("{e}");
                    continue;
                }
            };

            let json_output = parsed.json;

            let result = match parsed.command {
                ReplCommands::Wallet(cmd) => {
                    crate::wallet::handle_wallet(cmd, Some(&mut ow)).await
                }
                ReplCommands::Zkaccount(cmd) => {
                    crate::zkaccount::handle_zkaccount(cmd, Some(&mut ow)).await
                }
                ReplCommands::Order(cmd) => {
                    crate::order::handle_order(cmd, json_output, Some(&mut ow)).await
                }
                ReplCommands::Market(cmd) => {
                    crate::market::handle_market(cmd, json_output).await
                }
                ReplCommands::History(cmd) => {
                    crate::history::handle_history(cmd, json_output, Some(&mut ow)).await
                }
                ReplCommands::Portfolio(cmd) => {
                    crate::portfolio::handle_portfolio(cmd, json_output, Some(&mut ow)).await
                }
                ReplCommands::BitcoinWallet(cmd) => {
                    crate::bitcoin_wallet::handle_bitcoin_wallet(cmd, Some(&mut ow)).await
                }
                ReplCommands::VerifyTest(cmd) => {
                    crate::verify_test::handle_verify_test(cmd).await
                }
                ReplCommands::Help { command } => {
                    match command {
                        Some(group) => crate::help::print_subcommand_help(&group),
                        None => print_repl_help(),
                    }
                    Ok(())
                }
                ReplCommands::Reload => {
                    match load_order_wallet_from_db(
                        &wallet_id,
                        Some(password.clone()),
                        None,
                    ) {
                        Ok(new_ow) => {
                            ow = new_ow;
                            println!("Wallet reloaded from database.");
                            println!("  Accounts: {}", ow.zk_accounts.accounts.len());
                        }
                        Err(e) => eprintln!("Error reloading wallet: {e}"),
                    }
                    Ok(())
                }
                ReplCommands::Exit | ReplCommands::Quit => break,
            };

            if let Err(e) = result {
                eprintln!("Error: {e}");
            }

            println!(); // blank line between commands
        }

        // Save history for next session
        let _ = rl.save_history(&history_path);

        println!("Goodbye.");
    }

    Ok(())
}

/// Returns the path to the REPL history file (~/.relayer_cli_history).
fn dirs_history_path() -> std::path::PathBuf {
    let mut path = dirs_home();
    path.push(".relayer_cli_history");
    path
}

fn dirs_home() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

// ---------------------------------------------------------------------------
// REPL-specific help
// ---------------------------------------------------------------------------

pub(crate) fn print_repl_help() {
    println!(
        r#"Relayer REPL — interactive mode

COMMANDS (same as CLI, without the `relayer-cli` prefix):
    wallet <sub>          Wallet management
    zkaccount <sub>       ZkOS account operations
    order <sub>           Trading and lending orders
    market <sub>          Market data queries
    history <sub>         Local DB history
    portfolio <sub>       Portfolio tracking
    bitcoin-wallet <sub>  On-chain BTC operations
    verify-test <sub>     Testnet verification

REPL COMMANDS:
    reload      Reload wallet from database
    help        Show this help (or `help <group>` for details)
    exit/quit   Leave the REPL
    clear       Clear the screen

NOTES:
    - Wallet ID and password are cached for the session — no need to pass
      --wallet-id or --password on each command.
    - The wallet stays loaded in memory, so there is no repeated DB/crypto
      overhead between commands.
    - Use `reload` if you modified the wallet from another process.
    - You can still pass --json on any command for JSON output."#
    );
}
