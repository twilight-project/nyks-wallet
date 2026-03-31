# 🎲 Random Trades Bot Documentation

> **Disclaimer:** The code in this binary is for demonstration purposes only. It is intended to illustrate the usage of the `nyks-wallet` SDK and should not be considered a complete, production-ready trading strategy. The logic is simplified and may not perform as expected in a live trading environment.

This document provides a comprehensive guide to the Random Trades Bot, which demonstrates automated trading with random order generation and proper ZkOS account management.

## 📜 Overview

The Random Trades Bot is an automated trading bot that places random trader and lend orders at regular intervals. It serves as a comprehensive example of proper ZkOS account lifecycle management, order monitoring, and automated trading operations.

## 🎯 Strategy Overview

The bot's strategy includes:

- **Random Order Generation**: Places both trader orders (LONG/SHORT) and lend orders with random parameters
- **Account Management**: Uses proper ZkOS account lifecycle with funding → trading → account splitting
- **Order Monitoring**: Continuously monitors order status and handles fills, cancellations, and settlements
- **Account Rotation**: Properly rotates accounts using `trading_to_trading` after order settlement
- **Real-time Price Updates**: Fetches current market prices from the relayer API

## ⚙️ Usage

To run the random trades bot, use the following command:

```bash
cargo run --bin random_trades
```

### Example

```bash
cargo run --bin random_trades
```

## 🔧 Configuration Options

The bot can be configured through the `RandomOrderBotConfig` struct:

| Parameter                | Description                                  | Default Value |
| ------------------------ | -------------------------------------------- | ------------- |
| `initial_capital`        | Initial capital in satoshis                  | 100,000       |
| `initial_trader_orders`  | Number of trader orders to place initially   | 15            |
| `initial_lend_orders`    | Number of lend orders to place initially     | 15            |
| `order_interval_seconds` | Interval between order operations in seconds | 10            |
| `min_leverage`           | Minimum leverage for trader orders           | 1             |
| `max_leverage`           | Maximum leverage for trader orders           | 10            |
| `price_variation_pct`    | Price variation percentage for limit orders  | 0.02 (2%)     |

## 🔄 ZkOS Compliance and Account Lifecycle

The Random Trades Bot follows proper ZkOS account management patterns:

### Account Initialization

1. Creates a master trading account using `funding_to_trading()`
2. Splits the master account into multiple smaller accounts for order placement
3. Fetches current market price from relayer API

### Order Lifecycle Management

The bot handles all order statuses properly:

- **PENDING**: Order is waiting to be filled
- **FILLED**: Order is filled, ready to be closed
- **SETTLED**: Order is closed and settled
- **CANCELLED**: Order was cancelled, account can be reused

### Account Rotation

After an order is settled, the account is rotated using `trading_to_trading()`:

- Creates a new fresh account with the proceeds
- Returns the new account to the available pool
- Ensures proper ZkOS account management

## 🎲 Random Order Generation

### Trader Orders

- **Order Type**: 70% MARKET, 30% LIMIT
- **Position**: 50% LONG, 50% SHORT
- **Leverage**: Random between min/max range
- **Price**: Market price for MARKET orders, varied price for LIMIT orders

### Lend Orders

- Uses full account balance for lending
- Places lend orders on available accounts

## 📊 Statistics Tracking

The bot tracks comprehensive statistics:

- Orders placed/closed/cancelled
- Accounts rotated
- Uptime and performance metrics
- Active order counts

## 🛡️ Safety Features

- **Leverage Limits**: Configurable min/max leverage (1-50x)
- **Price Validation**: Ensures valid prices for limit orders
- **Account State Checking**: Validates account state before operations
- **Error Recovery**: Handles API failures gracefully

## 📋 Example Output

```
[INFO] Initializing random order bot accounts...
[INFO] Created master trading account 123 with 200000 sats (tx: abc123...)
[INFO] Splitting master account into 20 accounts with 10000 sats each
[INFO] Successfully created 20 trading accounts, ready for random trading
[INFO] Market price updated: 0 -> 65000
[INFO] Placing initial orders...
[INFO] Placing MARKET LONG trader order: account=124, leverage=3, price=65000
[INFO] Placing lend order: account=125, balance=10000
[INFO] Starting main trading loop with 10 second intervals
[INFO] Bot Stats - Uptime: 10s, Active Orders: 20, Available Accounts: 0, Orders Placed: 20, Orders Closed: 0, Accounts Rotated: 0
```

This bot provides a comprehensive example of automated trading with proper account management, making it suitable for both learning and understanding ZkOS trading patterns.
