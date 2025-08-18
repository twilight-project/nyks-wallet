# 🤖 Comprehensive Trading Bot Documentation

> **Disclaimer:** The code in this binary is for demonstration purposes only. It is intended to illustrate the usage of the `nyks-wallet` SDK and should not be considered a complete, production-ready trading strategy. The logic is simplified and may not perform as expected in a live trading environment.

This document provides an overview of the main executable for the Comprehensive Trading Bot, which serves as a framework for running various automated trading strategies.

## 📜 Overview

The main binary is the central entry point for the suite of trading bots. It is a sophisticated framework that supports multiple trading strategies, risk management features, and automated execution. It is designed to be a flexible platform for developing and deploying various trading algorithms on the Twilight Protocol.

## ✨ Features

The trading bot framework includes:

- **Multiple Trading Strategies**: It can run various trading strategies, such as Momentum, Mean Reversion, Arbitrage, and Market Making.
- **Risk Management**: Implements risk controls through position sizing, stop-losses, and leverage limits.
- **Automated Account Management**: Handles the creation and funding of trading accounts automatically.
- **Real-Time Order Execution**: Monitors and executes orders in real time.
- **Comprehensive Logging**: Provides detailed logs for monitoring and debugging.

## ⚙️ Usage

To run the main trading bot, you need to specify a strategy and can configure its parameters via the command line.

```bash
cargo run --bin main -- [OPTIONS]
```

### Example

```bash
cargo run --bin main -- --strategy momentum --initial-capital 100000 --max-leverage 10
```

## 🔧 Configuration Options

The bot's operation is controlled by the following command-line arguments:

| Argument                   | Description                                                                | Default Value |
| -------------------------- | -------------------------------------------------------------------------- | ------------- |
| `-s`, `--strategy`         | The trading strategy to use.                                               | `momentum`    |
| `-c`, `--initial-capital`  | The total initial capital in satoshis to be used by the bot.               | 100000        |
| `-l`, `--max-leverage`     | The maximum leverage to use for any position.                              | 10            |
| `-r`, `--risk-per-trade`   | The percentage of capital to risk on a single trade (e.g., 0.02 for 2%).   | 0.02          |
| `-t`, `--trading-interval` | The interval in seconds for the bot's trading loop.                        | 30            |
| `-p`, `--paper-trading`    | Enable paper trading mode for simulation without real funds.               | false         |
| `--max-trades`             | The maximum number of trades to execute before stopping (0 for unlimited). | 0             |

### Supported Strategies

You can choose from one of the following trading strategies:

- `momentum`
- `mean-reversion`
- `arbitrage`
- `market-making`

## 🔄 Account Initialization

The bot automates the setup of the necessary trading accounts:

1.  A **main trading account** is created and funded with 50% of the `initial_capital`.
2.  A **hedge account** is created and funded with 25% of the `initial_capital` for risk management purposes.

## 🌐 Real-Time Market Data Integration

The bot is capable of integrating real-time market data from the Relayer API, which is crucial for making informed trading decisions. This includes:

- **BTC/USD Price**: For accurate, real-time pricing.
- **Order Book**: To analyze market depth and spreads.
- **Recent Trades**: To gauge market activity and volume.

The system is designed with robust error handling, including fallbacks to simulated data if the live data feed is interrupted. For more details on the market data integration, refer to the `test_market_data` binary and its documentation.
