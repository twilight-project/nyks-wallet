//! # Market Data API Test
//!
//! This is a simple test utility to demonstrate the relayer API market data fetching
//! capabilities without running a full trading bot.
//!
//! ## Usage
//! ```bash
//! cargo run --bin test_market_data
//! ```

use anyhow::Result;
use log::info;
use nyks_wallet::relayer_module::order_wallet::OrderWallet;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    // Load environment variables
    dotenv::dotenv().ok();

    info!("Testing Relayer API Market Data Integration");
    info!("============================================");

    // Create OrderWallet to get relayer client
    let order_wallet = OrderWallet::new(None)?;
    let relayer_client = &order_wallet.relayer_api_client;

    // Test 1: Basic BTC/USD Price
    info!("\n1. Testing BTC/USD Price Fetching...");
    match relayer_client.btc_usd_price().await {
        Ok(price) => {
            info!(
                "✅ Successfully fetched BTC price: ${:.2} at {}",
                price.price, price.timestamp
            );
        }
        Err(e) => {
            info!("❌ Failed to fetch BTC price: {}", e);
        }
    }

    // Test 2: Order Book
    info!("\n2. Testing Order Book Fetching...");
    match relayer_client.open_limit_orders().await {
        Ok(order_book) => {
            info!("✅ Successfully fetched order book:");

            if let Some(best_bid) = order_book.bid.first() {
                info!(
                    "   Best Bid: ${:.2} (size: {:.2})",
                    best_bid.price, best_bid.positionsize
                );
            }

            if let Some(best_ask) = order_book.ask.first() {
                info!(
                    "   Best Ask: ${:.2} (size: {:.2})",
                    best_ask.price, best_ask.positionsize
                );

                // Calculate spread
                if let Some(best_bid) = order_book.bid.first() {
                    let spread = best_ask.price - best_bid.price;
                    let spread_pct = (spread / best_bid.price) * 100.0;
                    info!("   Spread: ${:.2} ({:.3}%)", spread, spread_pct);
                }
            }

            info!(
                "   Total Bids: {}, Total Asks: {}",
                order_book.bid.len(),
                order_book.ask.len()
            );
        }
        Err(e) => {
            info!("❌ Failed to fetch order book: {}", e);
        }
    }

    // Test 3: Recent Trades
    info!("\n3. Testing Recent Trades Fetching...");
    match relayer_client.recent_trade_orders().await {
        Ok(recent_trades) => {
            info!("✅ Successfully fetched recent trades:");
            info!("   Total recent orders: {}", recent_trades.orders.len());

            if !recent_trades.orders.is_empty() {
                let total_volume: f64 = recent_trades
                    .orders
                    .iter()
                    .map(|order| order.positionsize)
                    .sum();

                let avg_price: f64 = recent_trades
                    .orders
                    .iter()
                    .map(|order| order.price)
                    .sum::<f64>()
                    / recent_trades.orders.len() as f64;

                info!("   Total volume: {:.2} BTC", total_volume);
                info!("   Average price: ${:.2}", avg_price);

                // Show last few trades
                info!("   Recent trades:");
                for (i, trade) in recent_trades.orders.iter().take(3).enumerate() {
                    info!(
                        "     {}. {:?} {:.4} BTC @ ${:.2} ({})",
                        i + 1,
                        trade.side,
                        trade.positionsize,
                        trade.price,
                        trade.timestamp
                    );
                }
            }
        }
        Err(e) => {
            info!("❌ Failed to fetch recent trades: {}", e);
        }
    }

    // Test 4: Server Time
    info!("\n4. Testing Server Time...");
    match relayer_client.server_time().await {
        Ok(server_time) => {
            let local_time = chrono::Utc::now();
            let time_diff = (local_time - server_time).num_milliseconds();
            info!("✅ Server time: {}", server_time);
            info!("   Local time:  {}", local_time);
            info!("   Time diff:   {}ms", time_diff);
        }
        Err(e) => {
            info!("❌ Failed to fetch server time: {}", e);
        }
    }

    // Test 5: Position Size
    info!("\n5. Testing Position Size Data...");
    match relayer_client.position_size().await {
        Ok(position_size) => {
            info!("✅ Successfully fetched position size data:");
            info!(
                "   Total Long:  {:.2}",
                position_size.total_long_position_size
            );
            info!(
                "   Total Short: {:.2}",
                position_size.total_short_position_size
            );
            info!("   Total:       {:.2}", position_size.total_position_size);

            let long_pct = if position_size.total_position_size > 0.0 {
                (position_size.total_long_position_size / position_size.total_position_size) * 100.0
            } else {
                0.0
            };
            info!(
                "   Long/Short ratio: {:.1}% / {:.1}%",
                long_pct,
                100.0 - long_pct
            );
        }
        Err(e) => {
            info!("❌ Failed to fetch position size: {}", e);
        }
    }

    // Test 6: Funding Rate
    info!("\n6. Testing Funding Rate...");
    match relayer_client.get_funding_rate().await {
        Ok(funding_rate) => {
            info!("✅ Successfully fetched funding rate:");
            info!(
                "   Rate: {:.6}% (at BTC price: ${:.2})",
                funding_rate.rate * 100.0,
                funding_rate.btc_price
            );
            info!("   Timestamp: {}", funding_rate.timestamp);
        }
        Err(e) => {
            info!("❌ Failed to fetch funding rate: {}", e);
        }
    }

    // Test 7: Pool Information
    info!("\n7. Testing Pool Information...");
    match relayer_client.lend_pool_info().await {
        Ok(pool_info) => {
            info!("✅ Successfully fetched pool information:");
            info!("   Pool Info: {:?}", pool_info);
        }
        Err(e) => {
            info!("❌ Failed to fetch pool info: {}", e);
        }
    }

    info!("\n============================================");
    info!("Market Data API Test Complete");
    info!("Note: Some endpoints may not be available depending on the relayer setup.");

    Ok(())
}
