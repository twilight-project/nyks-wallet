use crate::commands::HistoryCmd;

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::helpers::{load_order_wallet_from_db, resolve_wallet_id};

pub(crate) async fn handle_history(cmd: HistoryCmd, json_output: bool) -> Result<(), String> {
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
