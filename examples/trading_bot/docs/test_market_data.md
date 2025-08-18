# 📊 Market Data API Test Utility Documentation

> **Disclaimer:** The code in this binary is for demonstration purposes only. It is intended to illustrate the usage of the `nyks-wallet` SDK and should not be considered a complete, production-ready trading strategy. The logic is simplified and may not perform as expected in a live trading environment.

This document provides an overview of the Market Data API Test Utility, a tool for demonstrating and verifying the market data fetching capabilities of the Relayer API.

## 📜 Overview

The `test_market_data` binary is a simple command-line utility designed to test and showcase the various market data endpoints available through the Relayer API. It allows developers to quickly check the API's status and the data it provides without needing to run a full trading bot.

## ✨ Features

The utility tests the following Relayer API endpoints:

- **BTC/USD Price**: Fetches the current price of BTC in USD.
- **Order Book**: Retrieves the current open limit orders (bids and asks).
- **Recent Trades**: Gets a list of recent trade orders.
- **Server Time**: Fetches the current server time for synchronization.
- **Position Size**: Gathers data on the total long and short position sizes.
- **Funding Rate**: Retrieves the current funding rate.
- **Pool Information**: Fetches information about the lending pool.

## ⚙️ Usage

To run the market data test utility, simply execute the following command:

```bash
cargo run --bin test_market_data
```

## 📋 Expected Output

When you run the utility, it will sequentially call each of the tested API endpoints and print the results to the console.

### Example Output

```
INFO  [test_market_data] Testing Relayer API Market Data Integration
INFO  [test_market_data] ============================================

INFO  [test_market_data]
1. Testing BTC/USD Price Fetching...
INFO  [test_market_data] ✅ Successfully fetched BTC price: $50123.45 at 2024-01-15 10:30:00 UTC

INFO  [test_market_data]
2. Testing Order Book Fetching...
INFO  [test_market_data] ✅ Successfully fetched order book:
INFO  [test_market_data]    Best Bid: $50120.00 (size: 1.50)
INFO  [test_market_data]    Best Ask: $50125.50 (size: 2.30)
INFO  [test_market_data]    Spread: $5.50 (0.011%)
INFO  [test_market_data]    Total Bids: 15, Total Asks: 20

INFO  [test_market_data]
3. Testing Recent Trades Fetching...
INFO  [test_market_data] ✅ Successfully fetched recent trades:
INFO  [test_market_data]    Total recent orders: 50
INFO  [test_market_data]    Total volume: 12.34 BTC
INFO  [test_market_data]    Average price: $50122.80
INFO  [test_market_data]    Recent trades:
INFO  [test_market_data]      1. LONG 0.5000 BTC @ $50124.00 (2024-01-15 10:29:55 UTC)
INFO  [test_market_data]      2. SHORT 0.2500 BTC @ $50123.50 (2024-01-15 10:29:50 UTC)
INFO  [test_market_data]      3. LONG 1.1000 BTC @ $50123.00 (2024-01-15 10:29:45 UTC)

... and so on for the other endpoints.
```

If any of the API calls fail, the utility will print an error message indicating the issue. This is useful for debugging problems with the Relayer API or your connection to it.
