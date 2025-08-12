# OrderWallet – Updated Relayer Module Guide

This document explains all features and function usage of the OrderWallet in the Twilight Protocol relayer module. It is an updated and expanded version of the original guide, aligned with the current API and implementation.

---

## 📑 Index

1. [Overview](#1--overview)
2. [Key Features](#2--key-features)
3. [Prerequisites](#3--prerequisites)
4. [Getting Started](#4--getting-started)
5. [Core API](#5--core-api)
6. [Trading Operations](#6--trading-operations)
7. [Lending Operations](#7--lending-operations)
8. [Account Management](#8--account-management)
9. [Database Persistence (optional)](#9--database-persistence-optional)
10. [Environment Configuration](#10--environment-configuration)
11. [Error Handling](#11--error-handling)
12. [End-to-End Examples](#12--end-to-end-examples)

---

## 1 • Overview

OrderWallet is a specialized wallet interface for automated trading and relayer operations on Twilight. It integrates ZkOS accounts for privacy and provides end-to-end order lifecycle management for trading and lending.

- Leveraged derivatives (LONG/SHORT)
- Lend/close-lend flows
- ZkOS account creation, transfer, and UTXO state tracking
- Request/transaction status querying with retries

---

## 2 • Key Features

### 2.1 Trading

- Market and Limit trader orders
- Leverage 1–50x with validation
- Open, query, close, cancel

### 2.2 Lending

- Open lend orders using account balance
- Query lend status and accrued amount
- Close lend and settle back to Coin state

### 2.3 Privacy & Security

- ZkOS-backed accounts per strategy
- Account IO types: Coin (spendable) vs Memo (locked in order)
- Deterministic key derivation from wallet seed

### 2.4 Integration & Reliability

- Uses relayer JSON-RPC client for order APIs
- UTXO and TX hash polling with exponential retry delays
- Optional encrypted DB persistence for wallet, Zk accounts, UTXO details, and request IDs

---

## 3 • Prerequisites

### 3.1 Cargo features

Enable database support only if you need persistence:

```toml
[features]
default = ["sqlite"] # or leave empty if you don’t want DB
sqlite = []
postgresql = []
```

### 3.2 Environment

See [Environment Configuration](#10--environment-configuration) for full list. Minimal usage works with defaults.

---

## 4 • Getting Started

> Important: Mnemonic display and security
>
> - Calling `OrderWallet::new` or `Wallet::new` generates a NEW mnemonic.
> - The mnemonic is printed ONCE directly to the terminal (TTY) using `print_secret_to_tty` and is not logged to stdout/stderr or persisted.
> - You must securely save the mnemonic when it is displayed; it cannot be retrieved later from the wallet or database.

### 4.1 Create a new OrderWallet with defaults

```rust
use nyks_wallet::relayer_module::order_wallet::OrderWallet;

fn main() -> Result<(), String> {
    env_logger::init();
    // Uses EndpointConfig::default() under the hood
    let mut order_wallet = OrderWallet::new(None)?;


    println!("Chain: {}", order_wallet.chain_id);
    Ok(())
}
```

### 4.2 Import using a mnemonic

```rust
use nyks_wallet::relayer_module::order_wallet::OrderWallet;

fn main() -> Result<(), String> {
    let mnemonic = "<your 24-word mnemonic>";
    let mut order_wallet = OrderWallet::import_from_mnemonic(mnemonic, None)?;
    Ok(())
}
```

### 4.3 Fund a ZK trading account from the on-chain wallet

```rust
#[tokio::main]
async fn main() -> Result<(), String> {
    let mut order_wallet = OrderWallet::new(None)?;
    info!("Getting test tokens from faucet");
    // get_test_tokens only availble for testnet
    match get_test_tokens(&mut wallet).await {
        Ok(_) => info!("Tokens received successfully"),
        Err(e) => return Err(e.to_string()),
    }
    // Mint 10_000 sats into a new ZK trading account
    let (tx_result, account_index) = order_wallet.funding_to_trading(10_000).await?;
    assert_eq!(tx_result.code, 0);
    println!("Funded account index: {}", account_index);
    Ok(())
}
```

---

## 5 • Core API

### 5.1 Structure

```rust
pub struct OrderWallet {
    pub wallet: Wallet,
    pub zk_accounts: ZkAccountDB,
    pub chain_id: String,
    // seed is kept secret and used to derive child ZkOS keys
    pub utxo_details: HashMap<AccountIndex, UtxoDetailResponse>,
    pub request_ids: HashMap<AccountIndex, String>,
    pub relayer_endpoint_config: RelayerEndPointConfig,
    // Optional DB fields behind sqlite/postgresql features
}
```

### 5.2 Constructors and loading

- `OrderWallet::new(endpoint_config: Option<EndpointConfig>) -> Result<Self, WalletError>`
- `OrderWallet::import_from_mnemonic(mnemonic: &str, endpoint_config: Option<EndpointConfig>) -> Result<Self, String>`
- With DB features: `OrderWallet::with_db(&mut self, wallet_password: Option<secrecy::SecretString>, wallet_id: Option<String>) -> Result<Self, String>`
- With DB features: `OrderWallet::load_from_db(wallet_id, password, db_url) -> Result<Self, String>`
- With DB features: `OrderWallet::get_wallet_list_from_db(db_url: Option<String>) -> Result<Vec<WalletList>, String>`

### 5.3 Utility methods

- `get_secret_key(index) -> RistrettoSecretKey`
- `request_id(index) -> Result<&str, String>`
- `ensure_coin_onchain(index) -> Result<(), String>`

### 5.4 Funding and transfers

- `funding_to_trading(amount) -> Result<(TxResult, u64), String>`
  - Mints trading BTC to a new ZK account. On success, account transitions to on-chain Coin state and is tracked in `utxo_details`.
- `trading_to_trading(index) -> Result<u64, String>`
  - Spends full balance of a Coin account into a newly created Coin account (strategy isolation or key rotation). Updates both accounts’ on-chain flags and UTXO tracking.

---

## 6 • Trading Operations

### 6.1 Open trader order

```rust
use twilight_client_sdk::relayer_types::{OrderType, PositionType};

#[tokio::main]
async fn main() -> Result<(), String> {
    let mut order_wallet = OrderWallet::new(None)?;
    let (_, account_index) = order_wallet.funding_to_trading(6_000).await?;

    // Example: LONG market with 10x leverage
    let request_id = order_wallet
        .open_trader_order(
            account_index,
            OrderType::MARKET,
            PositionType::LONG,
            50_000, // entry price (must be > 0)
            10,     // leverage 1..=50
        )
        .await?;
    println!("Order submitted: {}", request_id);
    Ok(())
}
```

Validations:

- `leverage` must be 1–50
- `entry_price` must be > 0
- Account must be on-chain in Coin state

Effects:

- On success, IO type set to `Memo` (funds locked in order)
- For market orders, Memo UTXO lookup is performed and stored
- `request_ids[index]` is updated

### 6.2 Query trader order

```rust
let order = order_wallet.query_trader_order(account_index).await?;
println!("status: {:?}", order.order_status);
```

Statuses include: `PENDING`, `FILLED`, `SETTLED`, `CANCELLED`.

### 6.3 Close trader order

```rust
// Market close (execution_price = 0.0)
let close_id = order_wallet
    .close_trader_order(account_index, OrderType::MARKET, 0.0)
    .await?;
```

Requirements and effects:

- The existing order must be `FILLED`
- After settlement, account IO type becomes `Coin`
- Balance is refreshed from the returned `available_margin`
- UTXO details are updated to Coin state

### 6.4 Cancel trader order

```rust
let cancel_id = order_wallet.cancel_trader_order(account_index).await?;
```

- Only allowed when order is `PENDING`
- After cancel, IO type becomes `Coin`; balance restored from `available_margin`

---

## 7 • Lending Operations

### 7.1 Open lend order

```rust
let request_id = order_wallet.open_lend_order(account_index).await?;
```

Effects:

- Uses entire account balance as lend amount
- IO type set to `Memo`
- UTXO details recorded as Memo

### 7.2 Query lend order

```rust
let lend = order_wallet.query_lend_order(account_index).await?;
println!("status: {:?}, amount: {}", lend.order_status, lend.new_lend_state_amount);
```

### 7.3 Close lend order

```rust
let close_id = order_wallet.close_lend_order(account_index).await?;
```

- Requires current status `FILLED`
- On success: status `SETTLED`, IO type becomes `Coin`, balance set to `new_lend_state_amount`

---

## 8 • Account Management

Each ZK account maintains:

- Balance in sats
- On-chain existence flag
- IO type: `Coin` (spendable) or `Memo` (locked)

Typical transitions:

```text
funding_to_trading → Coin(on-chain)
open_(trader|lend) → Memo(locked)
close/cancel       → Coin(on-chain)
trading_to_trading → Coin(old off-chain), Coin(new on-chain)
```

---

## 9 • Database Persistence (optional)

These APIs are compiled in only when `sqlite` or `postgresql` feature is enabled.

### 9.1 Enable persistence and save state

```rust
use secrecy::SecretString;

// Requires feature flag. If password is None, resolution is env → prompt
let mut order_wallet = OrderWallet::new(None)?;

// Option A: Provide password and a custom wallet_id explicitly
let order_wallet = order_wallet
    .with_db(Some(SecretString::new("strong passphrase".into())), Some("my_trading_wallet".into()))?;

// Option B: Resolve password via env/prompt and derive wallet_id from Twilight address
// let order_wallet = order_wallet.with_db(None, None)?;

// Persist OrderWallet config, encrypted wallet, zk accounts, utxo details, and request_ids
order_wallet.save_order_wallet_to_db()?;
```

#### 9.1.1 Password resolution order

When enabling DB persistence via `with_db(password, wallet_id)` (or when loading via `load_from_db`), the password is resolved in this order:

- Provided explicitly as function argument: `Some(SecretString)`
- Environment variable `NYKS_WALLET_PASSPHRASE`
- Interactive prompt (terminal input)

This means calling `with_db(None, ..)` or `load_from_db(wallet_id, None, ..)` will first look for `NYKS_WALLET_PASSPHRASE`. If it is not set, a prompt will appear to enter the passphrase.

#### 9.1.2 Wallet ID selection

- If you pass `Some(wallet_id)`, that value is used as the database key.
- If you pass `None`, the wallet ID defaults to the wallet's Twilight address (public identifier).
- The wallet ID must be unique; attempting to enable persistence with an existing wallet ID will return an error.

Behavior:

- Encrypted wallet is stored using AES-GCM with key derived from passphrase
- Zk accounts are upserted on create/update and during Drop
- UTXO details and request IDs are synced on updates and during Drop

### 9.2 Load from DB

```rust
use secrecy::SecretString;

let wallet_id = "<twilight_address>".to_string();
let password = Some(SecretString::new("strong passphrase".into()));
let mut order_wallet = OrderWallet::load_from_db(wallet_id, password, None)?;
```

You can also omit the password to use the same resolution order (env → prompt):

```rust
let wallet_id = "<twilight_address>".to_string();
// Will try NYKS_WALLET_PASSPHRASE first; if not set, will prompt
let mut order_wallet = OrderWallet::load_from_db(wallet_id, None, None)?;
```

### 9.3 List stored wallets

```rust
let list = OrderWallet::get_wallet_list_from_db(None)?;
for w in list { println!("{} {}", w.wallet_id, w.created_at); }
```

Use this when you need to discover which wallet IDs are available in the database before calling `load_from_db`. The function returns a `Vec<WalletList>`, where each item contains:

- wallet_id: the unique identifier (Twilight address) used to load a wallet
- created_at: the timestamp when the encrypted wallet was first stored

Example: Select a wallet and load it

```rust
use secrecy::SecretString;

let wallets = OrderWallet::get_wallet_list_from_db(None)?;
if let Some(first) = wallets.first() {
    println!("Loading wallet {} (created {})", first.wallet_id, first.created_at);
    // Provide a password or use None to resolve via env → prompt
    let mut order_wallet = OrderWallet::load_from_db(first.wallet_id.clone(), None, None)?;
}
```

---

## 10 • Environment Configuration

Required or useful variables (defaults exist for local dev):

| Variable                    | Description                                |
| --------------------------- | ------------------------------------------ |
| `NYKS_RPC_BASE_URL`         | Cosmos RPC endpoint                        |
| `NYKS_LCD_BASE_URL`         | Cosmos LCD endpoint                        |
| `FAUCET_BASE_URL`           | Faucet endpoint for test tokens            |
| `RELAYER_PROGRAM_JSON_PATH` | Path to relayer program config JSON        |
| `PUBLIC_API_RPC_SERVER_URL` | Public data API base URL (relayer)         |
| `RELAYER_RPC_SERVER_URL`    | Client order API base URL (relayer)        |
| `RUST_LOG`                  | logging level (e.g., `info`, `debug`)      |
| `NYKS_WALLET_PASSPHRASE`    | Passphrase for DB encryption (if using DB) |

Example production setup:

```bash
export NYKS_RPC_BASE_URL="https://rpc.twilight.rest"
export NYKS_LCD_BASE_URL="https://lcd.twilight.rest"
export FAUCET_BASE_URL="https://faucet-rpc.twilight.rest"
export PUBLIC_API_RPC_SERVER_URL="https://relayer.twilight.rest/api"
export RELAYER_RPC_SERVER_URL="https://relayer.twilight.rest/clientapi"
export RELAYER_PROGRAM_JSON_PATH="/path/to/relayerprogram.json"
export RUST_LOG=info
```

---

## 11 • Error Handling

Common errors and resolutions:

- "Insufficient balance" → top up wallet or reduce size
- "Account is not on chain or not a coin account" → wait for `funding_to_trading` confirmation
- "Leverage must be greater than 0 and less than 50" → fix parameter
- "Entry price must be greater than 0" → fix parameter
- "Order is not filled" (on close) → wait for fill or cancel
- "Order is not pending" (on cancel) → only pending orders can be cancelled
- UTXO/TxHash fetch failures → network hiccups; automatic retries are included

Helpers (internals):

- `fetch_utxo_details_with_retry(address, io_type)`
- `fetch_tx_hash_with_retry(request_id, client)`
- `send_tx_to_chain(signed_tx, rpc_endpoint)`

---

## 12 • End-to-End Examples

### 12.1 Complete trading cycle

```rust
#[tokio::test]
async fn test_complete_trading_cycle() -> Result<(), String> {
    env_logger::init();
    let mut order_wallet = OrderWallet::new(None)?;

    // 1. Fund trading account
    let (tx, idx) = order_wallet.funding_to_trading(6_000).await?;
    assert_eq!(tx.code, 0);

    // 2. Open position
    let req = order_wallet
        .open_trader_order(idx, OrderType::MARKET, PositionType::LONG, 50_000, 5)
        .await?;

    // 3. Verify order filled
    let o = order_wallet.query_trader_order(idx).await?;
    assert_eq!(o.order_status, OrderStatus::FILLED);

    // 4. Close position
    let close = order_wallet
        .close_trader_order(idx, OrderType::MARKET, 0.0)
        .await?;

    // 5. Verify settlement and account state
    let final_o = order_wallet.query_trader_order(idx).await?;
    assert_eq!(final_o.order_status, OrderStatus::SETTLED);
    Ok(())
}
```

### 12.2 Lend cycle

```rust
#[tokio::test]
async fn test_lend_cycle() -> Result<(), String> {
    let mut order_wallet = OrderWallet::new(None)?;
    let (_, idx) = order_wallet.funding_to_trading(6_000).await?;

    let req = order_wallet.open_lend_order(idx).await?;
    let lend = order_wallet.query_lend_order(idx).await?;
    assert_eq!(lend.order_status, OrderStatus::FILLED);

    let close = order_wallet.close_lend_order(idx).await?;
    let after = order_wallet.query_lend_order(idx).await?;
    assert_eq!(after.order_status, OrderStatus::SETTLED);
    Ok(())
}
```

### 12.3 Account rotation (trading_to_trading)

```rust
#[tokio::test]
async fn test_rotation() -> Result<(), String> {
    let mut order_wallet = OrderWallet::new(None)?;
    let (_, sender) = order_wallet.funding_to_trading(6_000).await?;
    let receiver = order_wallet.trading_to_trading(sender).await?;
    assert_ne!(sender, receiver);
    Ok(())
}
```

---

## Further Reading

- Main README – project overview
- Quick Start – basic wallet setup and funding
- Deployment guide – production deployment tips
- Twilight Client SDK – ZkOS and QuisQuis primitives
- Relayer Core – matching engine details

---

License: Apache-2.0 (see LICENSE)
