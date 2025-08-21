# 📈 Simple Market Maker Bot Documentation

> **Disclaimer:** The code in this binary is for demonstration purposes only. It is intended to illustrate the usage of the `nyks-wallet` SDK and should not be considered a complete, production-ready trading strategy. The logic is simplified and may not perform as expected in a live trading environment.

This document provides a detailed guide to the Simple Market Maker Bot, including its strategy, configuration, and ZkOS-compliant operational flow.

## 📜 Overview

The Simple Market Maker Bot is an automated trading bot that provides liquidity to the market by placing both buy (LONG) and sell (SHORT) orders around the current market price. Its goal is to profit from the bid-ask spread.

## 🎯 Strategy Overview

The bot's market-making strategy is as follows:

- **Dual Order Placement**: Places both a buy (LONG) and a sell (SHORT) limit order around an estimated market price.
- **Spread Management**: The distance between the buy and sell orders is determined by the `--spread` parameter.
- **Dynamic Adjustments**: The bot periodically updates its estimate of the market price and adjusts its orders accordingly.
- **Inventory Management**: It monitors its net position (inventory) and adjusts its trading to avoid accumulating excessive long or short exposure. If inventory limits are breached, it may place a hedge order to reduce risk.
- **Risk Management**: Basic risk management is implemented through inventory limits and order size constraints.

## ⚙️ Usage

To run the market maker bot, use the following command:

```bash
$ cargo run --bin simple_market_maker -- --help
```

```bash
cargo run --bin simple_market_maker -- [OPTIONS]
```

### Example

```bash
cargo run --bin simple_market_maker -- --spread 0.002 --order-size 1000 --max-inventory 10000 --initial-capital 50000 --paper-trading
```

### Production Configuration Example

For a more realistic, production-like setup, you might use the following configuration:

```bash
cargo run --bin simple_market_maker -- \
  --enhanced-market-data \
  --spread 0.0015 \
  --order-size 5000 \
  --max-inventory 20000 \
  --max-leverage 2 \
  --refresh-interval 45
```

## 🔧 Configuration Options

The bot can be configured using the following command-line arguments:

| Argument                   | Description                                                                                      | Default Value |
| -------------------------- | ------------------------------------------------------------------------------------------------ | ------------- |
| `-s`, `--spread`           | The spread percentage between buy and sell orders (e.g., 0.002 for 0.2%).                        | 0.002         |
| `-o`, `--order-size`       | The size of each order in satoshis.                                                              | 1000          |
| `-m`, `--max-inventory`    | The maximum inventory (net position) in satoshis.                                                | 10000         |
| `-c`, `--initial-capital`  | The total initial capital in satoshis for the bot.                                               | 50000         |
| `-r`, `--refresh-interval` | The interval in seconds at which the bot refreshes its orders.                                   | 60            |
| `-p`, `--paper-trading`    | Enable paper trading mode for simulation.                                                        | false         |
| `--max-leverage`           | The maximum leverage to use for hedging orders.                                                  | 3             |
| `--enhanced-market-data`   | Use enhanced market data, including the order book and recent trades, for more accurate pricing. | false         |

## 📈 Performance Metrics

The market maker bot offers two modes for fetching market data, each with different performance characteristics:

### Basic Mode

- **API Calls**: 1 per update cycle.
- **Latency**: Typically ~50-100ms.
- **Best For**: High-frequency updates (e.g., 15-30 second intervals).

### Enhanced Mode (`--enhanced-market-data`)

- **API Calls**: 3 concurrent calls per update cycle.
- **Latency**: Typically ~200-300ms.
- **Best For**: Medium-frequency updates (e.g., 60+ second intervals) where higher data quality is desired.

## 🔄 ZkOS Compliance and Account Lifecycle

The Simple Market Maker Bot is designed to be fully compliant with ZkOS, especially concerning account and order validation.

### Account Initialization

1.  A **master trading account** is funded with the `initial_capital`.
2.  This master account is then **split** into multiple smaller trading accounts (currently fixed at 6). This allows the bot to have multiple orders open simultaneously and rotate accounts as they are used.

### Trading Account Lifecycle Flow

The bot adheres to a strict lifecycle for each trading account to ensure ZkOS compliance:

1.  **Account Selection**: For a new order, the bot selects an account from its pool of available accounts. It will only select an account that is in the `IOType::Coin` state and has a balance greater than zero.
2.  **Order Placement**: A limit order is placed using the **full balance** of the selected account. The account's state transitions from `Coin` to `Memo`.
3.  **Order Filled**: When an order is filled, the bot immediately places a `MARKET` order to close the position.
4.  **Position Settlement**: Once the closing order is settled, the account's state transitions back to `Coin`.
5.  **Account Rotation**: The settled account is then "rotated" via `trading_to_trading` to generate a new, fresh account. This new account, with its updated balance, is returned to the pool of available accounts.
6.  **Order Cancellation**: If a limit order is canceled (e.g., if it's too old), the account does **not** need to be rotated. It can be immediately returned to the available accounts pool for reuse.

### Order Validation

Before placing any order, the bot performs several validation checks to prevent `Invalid order params` errors from the relayer:

- **Account State**: Confirms the account is `IOType::Coin` and has a positive balance.
- **Price**: Ensures the order price is greater than 0.
- **Leverage**: Validates that leverage is within the acceptable range (1 to 50).

This robust account and order management system ensures the bot operates reliably and avoids common pitfalls when interacting with the ZkOS protocol.
