# 🚀 Momentum Trader Bot Documentation

> **Disclaimer:** The code in this binary is for demonstration purposes only. It is intended to illustrate the usage of the `nyks-wallet` SDK and should not be considered a complete, production-ready trading strategy. The logic is simplified and may not perform as expected in a live trading environment.

This document provides a comprehensive guide to the Momentum Trader Bot, detailing its trading strategy, technical indicators, configuration, and ZkOS-compliant operational procedures.

## 📜 Overview

The Momentum Trader Bot is an automated trading strategy that aims to capitalize on market trends. It uses a combination of technical indicators to identify the direction and strength of market momentum and executes trades accordingly.

## 🎯 Strategy Overview

The bot's strategy is based on the following principles:

- **Trend Identification**: It uses a combination of Fast and Slow Moving Averages (MA) and the Relative Strength Index (RSI) to determine the market trend (Bullish, Bearish, or Sideways).
- **Signal Strength**: It calculates a "signal strength" to quantify the confidence in a trading signal. A trade is only initiated if the signal strength exceeds a configurable threshold.
- **Dynamic Leverage**: The leverage for a position is dynamically adjusted based on the signal strength—stronger signals result in higher leverage, up to a defined maximum.
- **Risk Management**: Each position is protected by a predefined stop-loss and take-profit level, which are calculated as a percentage of the entry price.
- **Position Management**: The bot will only hold one position at a time. It continuously monitors the market to determine the optimal time to exit a position, either due to a stop-loss, take-profit, or a reversal in the market signal.

## ⚙️ Usage

To run the momentum trader bot, use the following command:

```bash
cargo run --bin momentum_trader -- [OPTIONS]
```

### Example

```bash
cargo run --bin momentum_trader -- --fast-ma 10 --slow-ma 30 --rsi-period 14 --position-size 5000 --paper-trading
```

## 🔧 Configuration Options

The bot's behavior can be customized with these command-line arguments:

| Argument                    | Description                                                           | Default Value |
| --------------------------- | --------------------------------------------------------------------- | ------------- |
| `--fast-ma`                 | The period for the fast moving average.                               | 10            |
| `--slow-ma`                 | The period for the slow moving average.                               | 30            |
| `--rsi-period`              | The period for the Relative Strength Index (RSI) calculation.         | 14            |
| `--position-size`           | The size of each position in satoshis.                                | 5000          |
| `-l`, `--max-leverage`      | The maximum leverage to be used for a position.                       | 5             |
| `-c`, `--initial-capital`   | The total initial capital in satoshis for the bot.                    | 50000         |
| `-a`, `--analysis-interval` | The interval in seconds for market analysis.                          | 60            |
| `--stop-loss`               | The stop-loss percentage (e.g., 0.05 for 5%).                         | 0.05          |
| `--take-profit`             | The take-profit percentage (e.g., 0.15 for 15%).                      | 0.15          |
| `-p`, `--paper-trading`     | Enable paper trading mode for simulation.                             | false         |
| `--min-signal-strength`     | The minimum signal strength required to open a position (0.0 to 1.0). | 0.7           |

## 🔄 ZkOS Compliance and Account Lifecycle

The Momentum Trader Bot is fully compliant with ZkOS, particularly in its management of trading accounts to prevent common errors.

### Account Initialization

1.  A **master trading account** is created with the full `initial_capital`.
2.  The master account is then **split** into multiple smaller accounts (currently fixed at 3) to facilitate position rotation.

### Trading Account Lifecycle Flow

The bot follows a strict account lifecycle:

1.  **Account Selection**: To open a new position, the bot selects an available account that is in the `IOType::Coin` state and has a positive balance.
2.  **Order Placement**: A `MARKET` order is placed using the **full balance** of the chosen account. The account's state transitions from `Coin` to `Memo`.
3.  **Position Monitoring**: Once the order is `FILLED`, the bot monitors the position for exit conditions (stop-loss, take-profit, or signal reversal).
4.  **Position Closing**: When an exit condition is met, a `MARKET` order is sent to close the position.
5.  **Position Settlement**: After the closing order is `SETTLED`, the account is ready for rotation.
6.  **Account Rotation**: The settled account is "rotated" using `trading_to_trading`, which creates a new, fresh account containing the proceeds from the trade. This new account is then returned to the pool of available accounts.
7.  **Order Cancellation**: If an order is `CANCELLED`, the account does not need rotation and is immediately returned to the available accounts pool.

This process ensures that a fresh account is used for every new position, adhering to ZkOS's account management rules.
