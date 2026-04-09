use crate::commands::MarketCmd;
use crate::helpers::{parse_datetime, parse_interval};

pub(crate) async fn handle_market(cmd: MarketCmd, json_output: bool) -> Result<(), String> {
    use nyks_wallet::relayer_module::relayer_api::RelayerJsonRpcClient;

    let endpoint = nyks_wallet::config::RELAYER_API_RPC_SERVER_URL.to_string();
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
