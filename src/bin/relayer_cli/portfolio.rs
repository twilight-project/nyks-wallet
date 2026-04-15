use nyks_wallet::relayer_module::order_wallet::OrderWallet;
use nyks_wallet::relayer_module::relayer_types::OrderStatus;

use crate::commands::PortfolioCmd;
use crate::helpers::get_or_resolve_wallet;

pub(crate) async fn handle_portfolio(
    cmd: PortfolioCmd,
    json_output: bool,
    repl_wallet: Option<&mut OrderWallet>,
) -> Result<(), String> {
    match cmd {
        PortfolioCmd::Summary {
            wallet_id,
            password,
        } => {
            let mut ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

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
            println!("  Realised Lend PnL:   {:.2}", portfolio.realised_lend_pnl);
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

            if !portfolio.closed_lend_positions.is_empty() {
                println!("\nClosed Lend Positions (Settled & Unlocked)");
                println!("{}", "-".repeat(75));
                println!(
                    "  {:<6} {:>12} {:>12} {:>12} {:>16}",
                    "ACCT", "DEPOSIT", "VALUE", "PnL", "SHARES"
                );
                for p in &portfolio.closed_lend_positions {
                    println!(
                        "  {:<6} {:>12.2} {:>12.2} {:>12.2} {:>16.4}",
                        p.account_index,
                        p.deposit,
                        p.current_value,
                        p.pnl,
                        p.pool_share,
                    );
                }
                println!(
                    "\n  Total Realised Lend PnL: {:.2}",
                    portfolio.realised_lend_pnl
                );
            }
            Ok(())
        }

        PortfolioCmd::Balances {
            wallet_id,
            password,
            unit,
        } => {
            let ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

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
            let mut ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

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
