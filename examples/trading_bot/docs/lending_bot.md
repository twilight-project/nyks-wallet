# 🏦 Automated Lending Bot Documentation

> **Disclaimer:** The code in this binary is for demonstration purposes only. It is intended to illustrate the usage of the `nyks-wallet` SDK and should not be considered a complete, production-ready trading strategy. The logic is simplified and may not perform as expected in a live trading environment.

This document provides a comprehensive overview of the Automated Lending Bot, its strategy, configuration, and operational procedures, ensuring compliance with the ZkOS protocol.

## 📜 Overview

The Automated Lending Bot is designed to provide liquidity to the Twilight Protocol's lending markets, generating yield for the user. It follows an automated strategy to manage lending positions, monitor market rates, and handle risk exposure.

## 🎯 Strategy Overview

The bot's core strategy includes:

- **Automated Liquidity Provision**: Automatically lends assets to the market.
- **Rate Monitoring**: Continuously monitors lending rates and adjusts its positions to optimize yield.
- **Risk Management**: Manages lending exposure based on user-defined limits.
- **Performance Tracking**: Tracks key performance indicators (KPIs) such as yield generation and APY.

## ⚙️ Usage

To run the lending bot, use the following command:

```bash
cargo run --bin lending_bot -- [OPTIONS]
```

### Example

```bash
cargo run --bin lending_bot -- --min-rate 0.05 --max-exposure 0.8 --lending-amount 10000 --initial-capital 50000 --paper-trading
```

## 🔧 Configuration Options

The bot can be configured via command-line arguments:

| Argument                      | Description                                                             | Default Value |
| ----------------------------- | ----------------------------------------------------------------------- | ------------- |
| `--min-rate`                  | Minimum acceptable annual lending rate (e.g., 0.05 for 5%).             | 0.05          |
| `--max-exposure`              | Maximum exposure as a percentage of capital (e.g., 0.8 for 80%).        | 0.8           |
| `-l`, `--lending-amount`      | Default lending amount per order in satoshis.                           | 10000         |
| `-c`, `--initial-capital`     | The total initial capital in satoshis to be used by the bot.            | 50000         |
| `-m`, `--monitoring-interval` | The interval in seconds for monitoring lending rates.                   | 5             |
| `-p`, `--paper-trading`       | Enable paper trading mode to simulate trading without using real funds. | false         |
| `--max-positions`             | The maximum number of concurrent lending positions.                     | 5             |
| `--auto-reinvest`             | If enabled, profits are automatically reinvested.                       | false         |

## 🔄 ZkOS Compliance and Account Lifecycle

The lending bot is fully compliant with ZkOS, particularly regarding account management to prevent errors like "Account is not on chain or not a coin account". This is achieved through a strict account lifecycle management process.

### Account Initialization

1.  A single **master trading account** is created with the full `initial_capital`.
2.  This master account is then **split** into multiple smaller accounts, determined by `--max-positions`. Each of these smaller accounts will be used for a single lending position.

### Lending Account Lifecycle Flow

The lifecycle of an account ensures that no account is ever reused for a new order without being "rotated".

1.  **Account Pool Creation**: A pool of fresh, ready-to-use accounts is created.
2.  **Position Opening**: An available account (in `IOType::Coin` state) is taken from the pool.
3.  **Order Placement**: A lend order is placed using the **full balance** of the account. The account's state transitions from `Coin` to `Memo`.
4.  **Position Filled**: The order status becomes `FILLED`, and the lending position is now active.
5.  **Position Monitoring**: The bot monitors the position and market rates to decide when to close.
6.  **Position Closing**: When criteria are met, a `close_lend_order` request is sent.
7.  **Position Settlement**: The order status becomes `SETTLED`, and interest is earned. The account state transitions back from `Memo` to `Coin`.
8.  **Account Rotation**: The settled account is "rotated" using `trading_to_trading`, which creates a new, fresh account with the returned principal and interest.
9.  **Return to Pool**: The new, fresh account is added back to the available accounts pool, ready for a new lending operation.

## 🎯 Lending-Specific Rules on Twilight Protocol

Lending operations have specific characteristics compared to trading:

- **Order Statuses**: Lending orders only have two statuses: `FILLED` and `SETTLED`. There is no `PENDING` status.
- **No Cancellation**: Once a lend order is placed, it cannot be canceled.
- **No Liquidation**: There is no risk of liquidation for lending positions.
- **Mandatory Rotation**: Every account _must_ be rotated after its lending position is settled before it can be used again.

This rigorous account management ensures the bot operates smoothly and reliably on the Twilight Protocol.
