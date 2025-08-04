# OrderWallet – Relayer Module Trading Interface

This document explains how to use the **OrderWallet** for automated trading operations in the Twilight Protocol relayer system. OrderWallet provides a comprehensive interface for leveraged derivatives trading, lending operations, and ZkOS account management.

---

## 📑 Index

1. [Overview](#1--overview)
2. [Key Features](#2--key-features)
3. [Prerequisites](#3--prerequisites)
4. [Getting Started](#4--getting-started)
5. [Core Functionality](#5--core-functionality)
6. [Trading Operations](#6--trading-operations)
7. [Lending Operations](#7--lending-operations)
8. [Account Management](#8--account-management)
9. [Environment Configuration](#9--environment-configuration)
10. [Error Handling](#10--error-handling)
11. [Testing Examples](#11--testing-examples)

---

## 1 • Overview

**OrderWallet** is a specialized wallet interface designed for automated trading bots and relayer operations on the Twilight Protocol. It extends the base `Wallet` functionality with advanced features for:

- **Leveraged derivatives trading** with LONG/SHORT positions
- **Lending operations** for yield generation
- **ZkOS shielded account management** for privacy-preserving transactions
- **Order lifecycle management** (create, monitor, close, cancel)
- **Cross-account transfers** within the ZK privacy layer

The OrderWallet integrates seamlessly with the Twilight relayer infrastructure to enable high-frequency trading strategies while maintaining zero-knowledge privacy guarantees.

---

## 2 • Key Features

### 2.1 Trading Capabilities

- **Market & Limit Orders** – instant execution or conditional fills
- **Leveraged Positions** – configurable leverage up to protocol limits
- **Position Management** – programmatic open/close/cancel operations
- **Real-time Order Status** – query pending, filled, and settled states

### 2.2 Lending Operations

- **Liquidity Provision** – earn yield by lending to traders
- **Automated Lending** – programmatic lend order management
- **Interest Collection** – automatic settlement and balance updates

### 2.3 Privacy & Security

- **ZkOS Integration** – all trading operations are privacy-preserving
- **Account Isolation** – separate ZK accounts per trading strategy
- **Deterministic Key Derivation** – reproducible account generation from seed

### 2.4 Infrastructure Integration

- **Relayer Connectivity** – direct integration with Twilight relayer nodes
- **UTXO Management** – automatic tracking of on-chain state
- **Retry Logic** – robust error handling with exponential backoff

---

## 3 • Prerequisites

Before using OrderWallet, ensure you have:

### 3.1 Environment Setup

```bash
# Required environment variables
export NYKS_RPC_BASE_URL="https://rpc.twilight.rest"
export NYKS_LCD_BASE_URL="https://lcd.twilight.rest"
export FAUCET_BASE_URL="https://faucet-rpc.twilight.rest"
export RELAYER_PROGRAM_JSON_PATH="./relayerprogram.json"
```

### 3.2 Dependencies

```toml
[dependencies]
nyks-wallet = { path = "../nyks-wallet" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
log = "0.4"
env_logger = "0.11"
```

### 3.3 Relayer Program Configuration

Ensure you have a valid `relayerprogram.json` file containing the relayer smart contract configuration.

---

## 4 • Getting Started

### 4.1 Basic Setup

```rust
use nyks_wallet::{
    relayer_module::order_wallet::OrderWallet,
    wallet::Wallet,
    zkos_accounts::zkaccount::ZkAccountDB,
};

#[tokio::main]
async fn main() -> Result<(), String> {
    // Initialize logger
    env_logger::init();

    // Create or import wallet
    let wallet = Wallet::import_from_json("my_wallet.json")?;

    // Initialize ZK account database
    let zk_accounts = ZkAccountDB::new();

    // Create OrderWallet instance
    let mut order_wallet = OrderWallet::new(wallet, zk_accounts, "nyks")?;

    Ok(())
}
```

### 4.2 Fund Trading Account

```rust
// Transfer 10,000 sats from wallet to ZK trading account
let (tx_result, account_index) = order_wallet.funding_to_trading(10_000).await?;

if tx_result.code == 0 {
    println!("Successfully funded account {}: {}", account_index, tx_result.tx_hash);
} else {
    eprintln!("Funding failed: {}", tx_result.tx_hash);
}
```

---

## 5 • Core Functionality

### 5.1 OrderWallet Structure

```rust
pub struct OrderWallet {
    pub wallet: Wallet,                                    // Base wallet for chain operations
    pub zk_accounts: ZkAccountDB,                         // ZK account database
    pub chain_id: String,                                 // Target chain identifier
    seed: String,                                         // Seed for ZK key derivation
    pub utxo_details: HashMap<AccountIndex, UtxoDetailResponse>, // UTXO tracking
    pub request_ids: HashMap<AccountIndex, RequestId>,    // Order request tracking
}
```

### 5.2 Account Management

| Method                               | Purpose                              | Returns                           |
| ------------------------------------ | ------------------------------------ | --------------------------------- |
| `new(wallet, zk_accounts, chain_id)` | Create new OrderWallet instance      | `Result<OrderWallet, String>`     |
| `get_zk_account_seed(AccountIndex)`  | Derive ZK secret key for account     | `RistrettoSecretKey`              |
| `funding_to_trading(amount)`         | Move funds from wallet to ZK account | `Result<(TxResult, u64), String>` |
| `trading_to_trading(AccountIndex)`   | Transfer between ZK accounts         | `Result<AccountIndex, String>`    |

---

## 6 • Trading Operations

### 6.1 Opening Trading Positions

```rust
use twilight_client_sdk::relayer_types::{OrderType, PositionType};

// Open a LONG position with 10x leverage at market price
let request_id = order_wallet.open_trader_order(
    account_index,
    OrderType::MARKET,
    PositionType::LONG,
    entry_price,
    10, // leverage
).await?;

println!("Order submitted: {}", request_id);
```

### 6.2 Order Types & Parameters

| Parameter       | Type           | Description                                |
| --------------- | -------------- | ------------------------------------------ |
| `account_index` | `u64`          | ZK account to trade from                   |
| `order_type`    | `OrderType`    | `MARKET` or `LIMIT` and `LEND`             |
| `order_side`    | `PositionType` | `LONG` or `SHORT`                          |
| `entry_price`   | `u64`          | Entry price in satoshis (for LIMIT orders) |
| `leverage`      | `u64`          | Leverage multiplier (1-50x)                |

### 6.3 Position Management

```rust
// Query order status
let trader_order = order_wallet.query_trader_order(account_index).await?;
println!("Order status: {:?}", trader_order.order_status);

// Close position at market price
let close_request = order_wallet.close_trader_order(
    account_index,
    OrderType::MARKET,
    0.0, // execution_price (0.0 for market orders)
).await?;

// Cancel pending order
let cancel_request = order_wallet.cancel_trader_order(account_index).await?;
```

### 6.4 Order Status Lifecycle

```
PENDING     →   FILLED    →     SETTLED
   ↓               ↓
CANCELLED      LIQUIDATE
```

- **PENDING**: Order submitted, waiting for match
- **FILLED**: Order matched, position opened
- **SETTLED**: Position closed, funds returned
- **CANCELLED**: Order cancelled before fill
- **LIQUIDATE** : Liquidation price hit, Order Liquidated

---

## 7 • Lending Operations

### 7.1 Opening Lend Orders

```rust
// Provide liquidity for lending
let request_id = order_wallet.open_lend_order(account_index).await?;

// Query lending status
let lend_order = order_wallet.query_lend_order(account_index).await?;
println!("Lend amount: {}", lend_order.new_lend_state_amount);
```

### 7.2 Closing Lend Orders

```rust
// Close lending position and collect interest
let close_request = order_wallet.close_lend_order(account_index).await?;

// Check updated balance (principal + interest)
let account = order_wallet.zk_accounts.get_account(&account_index)?;
println!("Final balance: {}", account.balance);
```

---

## 8 • Account Management

### 8.1 Account States

Each ZK account tracks:

- **Balance**: Available satoshis
- **On-chain status**: Whether account exists on blockchain
- **IO Type**: `Coin` (tradeable) or `Memo` (locked in order)
- **Privacy**: All operations are zero-knowledge

### 8.2 Account Transitions

```rust
// Move funds between ZK accounts for strategy isolation
let new_account = order_wallet.trading_to_trading(old_account_index).await?;

// Verify account states
let old_account = order_wallet.zk_accounts.get_account(&old_account_index)?;
let new_account = order_wallet.zk_accounts.get_account(&new_account)?;

assert_eq!(old_account.on_chain, false); // Old account spent
assert_eq!(new_account.on_chain, true);  // New account created
```

---

## 9 • Environment Configuration

### 9.1 Required Variables

| Variable                    | Default                 | Description             |
| --------------------------- | ----------------------- | ----------------------- |
| `NYKS_RPC_BASE_URL`         | `http://0.0.0.0:26657`  | Cosmos RPC endpoint     |
| `NYKS_LCD_BASE_URL`         | `http://0.0.0.0:1317`   | Cosmos LCD endpoint     |
| `RELAYER_PROGRAM_JSON_PATH` | `./relayerprogram.json` | Relayer contract config |
| `RUST_LOG`                  | `info`                  | Logging level           |

### 9.2 Production Configuration

```bash
# Production endpoints
export NYKS_RPC_BASE_URL="https://rpc.twilight.rest"
export NYKS_LCD_BASE_URL="https://lcd.twilight.rest"
export FAUCET_BASE_URL="https://faucet-rpc.twilight.rest"

# Relayer integration
export RELAYER_PROGRAM_JSON_PATH="/path/to/relayerprogram.json"

# Enable debug logging
export RUST_LOG="debug"
```

---

## 10 • Error Handling

### 10.1 Common Error Scenarios

| Error Type                       | Cause                          | Resolution                        |
| -------------------------------- | ------------------------------ | --------------------------------- |
| `"Insufficient balance"`         | Not enough funds in wallet     | Fund wallet or reduce trade size  |
| `"Account is not on chain"`      | ZK account not yet confirmed   | Wait for transaction confirmation |
| `"Order is not filled"`          | Trying to close unfilled order | Wait for fill or cancel order     |
| `"Failed to fetch utxo details"` | Network connectivity issues    | Check RPC endpoints and retry     |

### 10.2 Robust Error Handling

```rust
use tokio::time::{sleep, Duration};

async fn robust_trading_operation(
    order_wallet: &mut OrderWallet,
    account_index: u64,
) -> Result<String, String> {
    let max_retries = 3;

    for attempt in 1..=max_retries {
        match order_wallet.open_trader_order(
            account_index,
            OrderType::MARKET,
            PositionType::LONG,
            50_000, // $50k BTC price
            10,     // 10x leverage
        ).await {
            Ok(request_id) => return Ok(request_id),
            Err(e) if attempt < max_retries => {
                eprintln!("Attempt {} failed: {}", attempt, e);
                sleep(Duration::from_secs(2_u64.pow(attempt))).await;
            },
            Err(e) => return Err(e),
        }
    }

    unreachable!()
}
```

---

## 11 • Testing Examples

### 11.1 Complete Trading Flow

```rust
#[tokio::test]
async fn test_complete_trading_cycle() -> Result<(), String> {
    let wallet = setup_test_wallet().await?;
    let zk_accounts = ZkAccountDB::new();
    let mut order_wallet = OrderWallet::new(wallet, zk_accounts, "nyks")?;

    // 1. Fund trading account
    let (tx_result, account_index) = order_wallet.funding_to_trading(10_000).await?;
    assert_eq!(tx_result.code, 0);

    // 2. Open leveraged position
    let request_id = order_wallet.open_trader_order(
        account_index,
        OrderType::MARKET,
        PositionType::LONG,
        50_000, // BTC price
        5,      // 5x leverage
    ).await?;

    // 3. Verify order filled
    let order = order_wallet.query_trader_order(account_index).await?;
    assert_eq!(order.order_status, OrderStatus::FILLED);

    // 4. Close position
    let close_request = order_wallet.close_trader_order(
        account_index,
        OrderType::MARKET,
        0.0,
    ).await?;

    // 5. Verify settlement
    let final_order = order_wallet.query_trader_order(account_index).await?;
    assert_eq!(final_order.order_status, OrderStatus::SETTLED);

    Ok(())
}
```

### 11.2 Automated Market Making

```rust
async fn market_making_strategy(
    order_wallet: &mut OrderWallet,
    account_index: u64,
    base_price: u64,
    spread_pct: f64,
) -> Result<(), String> {
    // Create new account for each side
    let long_account = order_wallet.trading_to_trading(account_index).await?;
    let short_account = order_wallet.trading_to_trading(account_index).await?;

    let spread = (base_price as f64 * spread_pct / 100.0) as u64;

    // Open long position below market
    let long_request = order_wallet.open_trader_order(
        long_account,
        OrderType::LIMIT,
        PositionType::LONG,
        base_price - spread,
        1, // 1x leverage for market making
    ).await?;

    // Open short position above market
    let short_request = order_wallet.open_trader_order(
        short_account,
        OrderType::LIMIT,
        PositionType::SHORT,
        base_price + spread,
        1, // 1x leverage
    ).await?;

    println!("Market making orders placed: {} {}", long_request, short_request);
    Ok(())
}
```

### 11.3 Lending Strategy

```rust
async fn lending_strategy(
    order_wallet: &mut OrderWallet,
    account_index: u64,
) -> Result<(), String> {
    // Open lending position
    let lend_request = order_wallet.open_lend_order(account_index).await?;

    // Monitor lending status
    // loop {
    //     let lend_order = order_wallet.query_lend_order(account_index).await?;

    //     match lend_order.order_status {
    //         OrderStatus::FILLED => {
    //             println!("Lending active, earning interest...");
    //             tokio::time::sleep(Duration::from_secs(3600)).await; // Check hourly
    //         },
    //         OrderStatus::SETTLED => {
    //             println!("Lending completed, collecting funds...");
    //             break;
    //         },
    //         _ => {
    //             tokio::time::sleep(Duration::from_secs(60)).await; // Check every minute
    //         }
    //     }
    // }

    // Close lending position
    let close_request = order_wallet.close_lend_order(account_index).await?;
    println!("Lending closed: {}", close_request);

    Ok(())
}
```

---

## Further Reading

- [Main README](README.md) – Overview of nyks-wallet capabilities
- [Quick Start Guide](QuickStart.md) – Basic wallet setup and funding
- [Deployment Guide](DEPLOYMENT.md) – Production deployment instructions
- [Twilight Client SDK](https://github.com/twilight-project/twilight-client-sdk) – ZkOS and QuisQuis primitives
- [Relayer Core](https://github.com/twilight-project/relayer-core) – High-performance matching engine

---

**License**: Released under the Apache License – see [LICENSE](LICENSE) for details.
