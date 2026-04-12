use nyks_wallet::relayer_module::order_wallet::OrderWallet;
// use nyks_wallet::relayer_module::relayer_types::OrderStatus;

use crate::commands::OrderCmd;
use crate::helpers::{get_or_resolve_wallet, parse_datetime, parse_order_status, parse_order_type, parse_position_type};

pub(crate) async fn handle_order(
    cmd: OrderCmd,
    json_output: bool,
    repl_wallet: Option<&mut OrderWallet>,
) -> Result<(), String> {
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

            let mut ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

            if !json_output {
                println!(
                    "Opening {side} {order_type} order on account {account_index} (price={entry_price}, leverage={leverage}x)..."
                );
            }
            let request_id = match ow
                .open_trader_order(account_index, ot, ps, entry_price, leverage)
                .await
            {
                Ok(id) => id,
                Err(e) if e.contains("Value Witness Verification Failed") => {
                    return Err(format!(
                        "{e}\n\nHint: If a previous order on this account was closed, you need to \
                         create a transfer first before placing a new order.\n\
                         An order cannot be placed with the same account address twice.\n\
                         Use: relayer-cli zkaccount transfer --account-index {account_index}"
                    ));
                }
                Err(e) => return Err(e),
            };
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
            let mut ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

            if !json_output {
                println!("Closing trader order on account {account_index}...");
            }
            if order_type == "LIMIT" && (stop_loss.is_some() || take_profit.is_some()) {
                return Err(
                    "Cannot combine --order-type LIMIT with --stop-loss or --take-profit. \
                     Use LIMIT for a limit close at --execution-price, \
                     or use MARKET (default) with --stop-loss / --take-profit for SLTP triggers."
                        .to_string(),
                );
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
                if order_type == "LIMIT" {
                    if execution_price == 0.0 {
                        return Err("Execution price is required for limit orders".to_string());
                    }
                }
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
            stop_loss,
            take_profit,
            wallet_id,
            password,
        } => {
            let mut ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

            if !json_output {
                println!("Cancelling trader order on account {account_index}...");
            }

            let request_id = if stop_loss || take_profit {
                ow.cancel_trader_order_sltp(account_index, stop_loss, take_profit)
                    .await?
            } else {
                ow.cancel_trader_order(account_index).await?
            };

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
            let mut ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

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
            let mut ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

            if !json_output {
                println!("Opening lend order on account {account_index}...");
            }
            let request_id = match ow.open_lend_order(account_index).await {
                Ok(id) => id,
                Err(e) if e.contains("Value Witness Verification Failed") => {
                    return Err(format!(
                        "{e}\n\nHint: If the account was previously used for an open/closed order, \
                         you must transfer the account first before placing a new order.\n\
                         An order cannot be placed with the same account address twice.\n\
                         Use: relayer-cli zkaccount transfer --account-index {account_index}"
                    ));
                }
                Err(e) => return Err(e),
            };
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
            let mut ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

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

        OrderCmd::UnlockCloseOrder {
            account_index,
            wallet_id,
            password,
        } => {
            let mut ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

            let tx_type = ow.zk_accounts.get_account(&account_index)?.tx_type.clone();

            let result = match tx_type {
                Some(twilight_client_sdk::relayer_types::TXType::LENDTX) => {
                    if !json_output {
                        println!("Unlocking settled lend order on account {account_index}...");
                    }
                    ow.unlock_lend_order(account_index).await
                }
                Some(twilight_client_sdk::relayer_types::TXType::ORDERTX) | None => {
                    if !json_output {
                        println!("Unlocking settled trader order on account {account_index}...");
                    }
                    ow.unlock_trader_order(account_index).await
                }
            };

            match result {
                Ok((order_status, request_id)) => {
                    if json_output {
                        println!(
                            "{}",
                            serde_json::json!({"account_index": account_index, "order_status": format!("{:?}", order_status), "request_id": request_id})
                        );
                    } else {
                        println!(
                            "Account {} unlocked successfully, order status: {:?}. Request ID: {}",
                            account_index, order_status, request_id
                        );
                    }
                }
                Err(e) => {
                    if json_output {
                        println!(
                            "{}",
                            serde_json::json!({"account_index": account_index, "error": e})
                        );
                    } else {
                        println!("Error: {}", e);
                    }
                }
            }
            Ok(())
        }

        OrderCmd::UnlockFailedOrder {
            account_index,
            wallet_id,
            password,
        } => {
            let mut ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

            if !json_output {
                println!("Unlocking failed order on account {account_index}...");
            }
            match ow.unlock_failed_order(account_index).await {
                Ok(()) => {
                    if json_output {
                        println!(
                            "{}",
                            serde_json::json!({"account_index": account_index, "status": "unlocked"})
                        );
                    } else {
                        println!(
                            "Account {} unlocked successfully (failed order cleared)",
                            account_index
                        );
                    }
                }
                Err(e) => {
                    if json_output {
                        println!(
                            "{}",
                            serde_json::json!({"account_index": account_index, "error": e})
                        );
                    } else {
                        println!("Error: {}", e);
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
            let mut ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

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
            let mut ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

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
            let mut ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

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
            let mut ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

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
            let ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

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
            reason,
        } => {
            use nyks_wallet::relayer_module::relayer_api::RelayerJsonRpcClient;
            use nyks_wallet::relayer_module::relayer_types::TransactionHashArgs;

            let endpoint = nyks_wallet::config::RELAYER_API_RPC_SERVER_URL.to_string();
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
                // Group hashes by order_id
                use std::collections::BTreeMap;
                let mut grouped: BTreeMap<String, Vec<_>> = BTreeMap::new();
                for h in &hashes {
                    grouped.entry(h.order_id.to_string()).or_default().push(h);
                }

                // Show account ID above all tables (same for all records)
                println!("Account: {}", hashes[0].account_id);
                println!();

                for (order_id, entries) in &grouped {
                    println!("Order ID: {}", order_id);
                    if reason {
                        println!("{}", "-".repeat(180));
                        println!(
                            "  {:<20} {:<10} {:<20} {:>10} {:>10} {:<64} {}",
                            "STATUS", "TYPE", "DATE", "OLD PRICE", "NEW PRICE", "TX HASH", "REASON"
                        );
                    } else {
                        println!("{}", "-".repeat(140));
                        println!(
                            "  {:<20} {:<10} {:<20} {:>10} {:>10} {:<64}",
                            "STATUS", "TYPE", "DATE", "OLD PRICE", "NEW PRICE", "TX HASH"
                        );
                    }
                    for h in entries {
                        if reason {
                            println!(
                                "  {:<20} {:<10} {:<20} {:>10} {:>10} {:<64} {}",
                                format!("{:?}", h.order_status),
                                format!("{:?}", h.order_type),
                                &h.datetime[..std::cmp::min(20, h.datetime.len())],
                                h.old_price.map_or("-".to_string(), |p| format!("{:.2}", p)),
                                h.new_price.map_or("-".to_string(), |p| format!("{:.2}", p)),
                                h.tx_hash,
                                h.reason.as_deref().unwrap_or("-"),
                            );
                        } else {
                            println!(
                                "  {:<20} {:<10} {:<20} {:>10} {:>10} {:<64}",
                                format!("{:?}", h.order_status),
                                format!("{:?}", h.order_type),
                                &h.datetime[..std::cmp::min(20, h.datetime.len())],
                                h.old_price.map_or("-".to_string(), |p| format!("{:.2}", p)),
                                h.new_price.map_or("-".to_string(), |p| format!("{:.2}", p)),
                                h.tx_hash,
                            );
                        }
                    }
                    println!("  Total: {} hash(es)\n", entries.len());
                }
            }
            Ok(())
        }

        OrderCmd::RequestHistory {
            account_index,
            wallet_id,
            password,
            status,
            limit,
            offset,
            reason,
        } => {
            use nyks_wallet::relayer_module::relayer_api::RelayerJsonRpcClient;
            use nyks_wallet::relayer_module::relayer_types::TransactionHashArgs;

            let ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

            let account_address = ow.zk_accounts.get_account_address(&account_index)?;

            let endpoint = nyks_wallet::config::RELAYER_API_RPC_SERVER_URL.to_string();
            let client = RelayerJsonRpcClient::new(&endpoint).map_err(|e| e.to_string())?;

            let status = status.map(|s| parse_order_status(&s)).transpose()?;
            let params = TransactionHashArgs::AccountId {
                id: account_address.clone(),
                status,
                limit,
                offset,
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
                println!("No transaction hashes found for account index {account_index}");
            } else {
                use std::collections::BTreeMap;
                let mut grouped: BTreeMap<String, Vec<_>> = BTreeMap::new();
                for h in &hashes {
                    grouped.entry(h.order_id.to_string()).or_default().push(h);
                }

                println!("Account: {}", account_address);
                println!();

                for (order_id, entries) in &grouped {
                    println!("Order ID: {}", order_id);
                    if reason {
                        println!("{}", "-".repeat(180));
                        println!(
                            "  {:<20} {:<10} {:<20} {:>10} {:>10} {:<64} {}",
                            "STATUS", "TYPE", "DATE", "OLD PRICE", "NEW PRICE", "TX HASH", "REASON"
                        );
                    } else {
                        println!("{}", "-".repeat(140));
                        println!(
                            "  {:<20} {:<10} {:<20} {:>10} {:>10} {:<64}",
                            "STATUS", "TYPE", "DATE", "OLD PRICE", "NEW PRICE", "TX HASH"
                        );
                    }
                    for h in entries {
                        if reason {
                            println!(
                                "  {:<20} {:<10} {:<20} {:>10} {:>10} {:<64} {}",
                                format!("{:?}", h.order_status),
                                format!("{:?}", h.order_type),
                                &h.datetime[..std::cmp::min(20, h.datetime.len())],
                                h.old_price.map_or("-".to_string(), |p| format!("{:.2}", p)),
                                h.new_price.map_or("-".to_string(), |p| format!("{:.2}", p)),
                                h.tx_hash,
                                h.reason.as_deref().unwrap_or("-"),
                            );
                        } else {
                            println!(
                                "  {:<20} {:<10} {:<20} {:>10} {:>10} {:<64}",
                                format!("{:?}", h.order_status),
                                format!("{:?}", h.order_type),
                                &h.datetime[..std::cmp::min(20, h.datetime.len())],
                                h.old_price.map_or("-".to_string(), |p| format!("{:.2}", p)),
                                h.new_price.map_or("-".to_string(), |p| format!("{:.2}", p)),
                                h.tx_hash,
                            );
                        }
                    }
                    println!("  Total: {} hash(es)\n", entries.len());
                }
            }
            Ok(())
        }
    }
}
