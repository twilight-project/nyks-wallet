# OrderWallet – Relayer Module Trading Interface

This document explains how to use the **OrderWallet** for automated trading operations in the Twilight Protocol relayer system. OrderWallet provides a comprehensive interface for leveraged derivatives trading, lending operations, ZkOS account management, and optional database persistence.

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
9. [Database Persistence (optional)](#9--database-persistence-optional)
10. [Environment Configuration](#10--environment-configuration)
11. [Error Handling](#11--error-handling)
12. [Testing Examples](#12--testing-examples)

---

## 1 • Overview

**OrderWallet** is a specialized wallet interface designed for automated trading bots and relayer operations on the Twilight Protocol. It integrates ZkOS-backed accounts and provides:

- **Leveraged derivatives trading** with LONG/SHORT positions
- **Lending operations** for yield generation
- **ZkOS shielded account management** for privacy-preserving transactions
- **Order lifecycle management** (open, query, close, cancel)
- **Cross-account transfers** (single and multiple receivers) within the ZK privacy layer

OrderWallet integrates seamlessly with Twilight relayer infrastructure for high-frequency strategies while maintaining zero-knowledge privacy guarantees.

---

## 2 • Key Features

### 2.1 Trading Capabilities

- **Market & Limit Orders** – instant execution or conditional fills
- **Leveraged Positions** – leverage bounded at runtime by the relayer risk engine (`params.max_leverage` from `get_market_stats`)
- **Position Management** – programmatic open/close/cancel, including stop-loss / take-profit variants
- **Pre-submission Risk Validation** – `validate_open_order` replicates server-side checks (market status, max leverage, min position size, per-position cap, directional headroom)
- **Real-time Order Status** – query pending, filled, settled, and v1 enhanced views (settle_limit, stop_loss, take_profit, funding_applied)
- **History** – historical trader/lend orders and funding payment history

### 2.2 Lending Operations

- **Liquidity Provision** – earn yield by lending to pool
- **Automated Lending** – programmatic lend order management
  – open lend orders using full account balance
  – query lend status and accrued amount
- **Interest Collection** – close lend and settle back to Coin state and balance updates

### 2.3 Privacy & Security

- **ZkOS Integration** – all trading operations are privacy-preserving
- **Account Isolation** – Account IO types: `Coin` (spendable) vs `Memo` (locked in order)
- **Deterministic Key Derivation** – Deterministic key derivation from wallet seed

### 2.4 Integration & Reliability

- **Relayer Connectivity** – layer JSON-RPC client for order APIs
- **UTXO Management** – automatic tracking of on-chain state
- **Retry Logic** – robust error handling with exponential backoff
- **Data Persistency** - Optional encrypted DB persistence for wallet, ZK accounts, UTXO details, and request IDs

---

## 3 • Prerequisites

### 3.1 Environment Setup

Follow the `.env.example` file. See [Environment Configuration](#10--environment-configuration) for variables.

### 3.2 Dependencies

> Types like `IOType`, `OrderStatus`, `OrderType`, and `PositionType` are re-exported by this crate. Import them from `nyks_wallet::relayer_module::relayer_types` — no extra dependency is required.

Relayer only (no DB persistence):

```toml
[dependencies]
nyks-wallet = { path = "../nyks-wallet", default-features = false, features = ["order-wallet"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
log = "0.4"
env_logger = "0.11"
```

With SQLite persistence (default features):

```toml
[dependencies]
nyks-wallet = { path = "../nyks-wallet" } # default = ["sqlite", "order-wallet"]
secrecy = "0.8"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
log = "0.4"
env_logger = "0.11"
```

With PostgreSQL persistence:

```toml
[dependencies]
nyks-wallet = { path = "../nyks-wallet", default-features = false, features = ["postgresql"] }
secrecy = "0.8"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
log = "0.4"
env_logger = "0.11"
```

### 3.3 Cargo features

From this crate’s `Cargo.toml`:

```toml
[features]
# Default build uses SQLite and enables OrderWallet APIs
default = ["sqlite", "order-wallet"]

# Exactly one of these DB backends should be enabled at a time
sqlite = [
    "diesel/sqlite",
    "diesel_migrations",
    "libsqlite3-sys/bundled",
    "r2d2",
    "order-wallet",
]

postgresql = [
    "diesel/postgres",
    "diesel_migrations",
    "pq-sys",
    "r2d2",
    "order-wallet",
]

# Only enable this if you want to build the validator wallet
validator-wallet = []

# Enables relayer OrderWallet APIs
order-wallet = ["dep:twilight-client-sdk", "curve25519-dalek"]
```

Usage tips:

- Disable DB by turning off defaults and enabling only `order-wallet`:
  - `nyks-wallet = { ..., default-features = false, features = ["order-wallet"] }`
- Use SQLite (default) without extra flags, or explicitly set `features = ["sqlite"]`.
- For PostgreSQL, disable defaults and enable `features = ["postgresql"]`.

### 3.4 Relayer Program Configuration

Ensure you have a valid `relayerprogram.json` file containing the relayer smart contract configuration (path configured via `RELAYER_PROGRAM_JSON_PATH`).

---

## 4 • Getting Started

> Important: Mnemonic display and security
>
> - Creating a new wallet prints a NEW mnemonic ONCE directly to the terminal (TTY). It is not logged or persisted.
> - Save the mnemonic securely when it is displayed; it cannot be retrieved later.

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

    // Optional: get test tokens on testnet
    // use nyks_wallet::get_test_tokens;
    // get_test_tokens(&mut order_wallet.wallet).await.ok();

    // Mint 10_000 sats into a new ZK trading account
    let (tx_result, account_index) = order_wallet.funding_to_trading(10_000).await?;
    assert_eq!(tx_result.code, 0);
    println!("Funded account index: {}", account_index);
    Ok(())
}
```

---

## 5 • Core Functionality

### 5.1 OrderWallet Structure

```rust
pub struct OrderWallet {
    pub wallet: Wallet,                                          // Base wallet for chain operations
    pub zk_accounts: ZkAccountDB,                                // ZK account database
    pub chain_id: String,                                        // Target chain identifier
    seed: SecretString,                                          // Seed for ZK key derivation (private)
    pub utxo_details: HashMap<AccountIndex, UtxoDetailResponse>, // UTXO tracking
    pub request_ids: HashMap<AccountIndex, RequestId>,           // Order request tracking
    pub relayer_api_client: RelayerJsonRpcClient,                // Relayer RPC client
    pub relayer_endpoint_config: RelayerEndPointConfig,          // Relayer endpoints/config
    pub nonce_manager: Arc<NonceManager>,                        // Sequence/account_number manager
    // Database fields are only present under `sqlite` or `postgresql` features.
}
```

### 5.2 Constructors and loading

- `OrderWallet::new(endpoint_config: Option<EndpointConfig>) -> Result<Self, WalletError>`
- `OrderWallet::import_from_mnemonic(mnemonic: &str, endpoint_config: Option<EndpointConfig>) -> Result<Self, String>`
- With DB features: `with_db(&mut self, password: Option<SecretString>, wallet_id: Option<String>) -> Result<Self, String>`
- With DB features: `load_from_db(wallet_id: String, password: Option<SecretString>, db_url: Option<String>) -> Result<OrderWallet, String>`
- With DB features: `get_wallet_list_from_db(db_url: Option<String>) -> Result<Vec<WalletList>, String>`
- With DB features: `get_wallet_id_from_db(wallet_id: &str, db_url: Option<String>) -> Result<bool, String>`
- With DB features: `get_db_manager(&self) -> Option<&DatabaseManager>`
- With DB features: `get_wallet_password(&self) -> Option<&SecretString>`

### 5.3 Utility methods

- `get_secret_key(index) -> RistrettoSecretKey` – derive a child key for an account index
- `request_id(index) -> Result<&str, String>` – last stored request ID for an account
- `ensure_coin_onchain(index) -> Result<(), String>` – check on-chain Coin state + non-zero balance
- `ensure_zk_account_onchain(&ZkAccount) -> Result<(), String>` – same check given an account reference
- `sync_nonce(&self) -> Result<(), String>` – re-anchor the local sequence counter from chain; call before transaction batches or periodically
- `sync_account_state(&mut self, index) -> Result<(), String>` – refresh the on-chain UTXO state for an account; use this to complete a deferred sync after a `--no-wait` open/close

### 5.4 Funding and transfers

- `funding_to_trading(amount) -> Result<(TxResult, u64), String>`
  - Mints trading BTC to a new ZK account. On success, account transitions to on-chain Coin state and is tracked in `utxo_details`.
- `trading_to_trading(index) -> Result<u64, String>`
  - Spends full balance of a Coin account into a newly created Coin account. Updates both accounts’ on-chain flags and UTXO tracking.
- `trading_to_trading_multiple_accounts(sender_index, balances: Vec<u64>) -> Result<Vec<(u64, u64)>, String>`
  - Splits one Coin account into multiple new Coin accounts, each funded with the specified amount.
- `trading_to_funding(index) -> Result<(), String>`
  - Burns ZK Coin back to the on-chain wallet.

#### 5.4.1 Multi-account transfer usage

```rust
#[tokio::main]
async fn main() -> Result<(), String> {
    let mut order_wallet = OrderWallet::new(None)?;
    // Prepare a sender account with enough Coin balance
    let (tx, sender_idx) = order_wallet.funding_to_trading(40_000).await?;
    assert_eq!(tx.code, 0);

    // Create multiple new accounts with specified balances
    let balances = vec![5_000, 1_000, 8_000, 600];
    let new_accounts = order_wallet
        .trading_to_trading_multiple_accounts(sender_idx, balances)
        .await?;

    println!("created accounts: {:?}", new_accounts); // Vec<(account_index, balance)>
    Ok(())
}
```

Requirements and effects:

- Sender must be on-chain in Coin state and have sufficient balance for the sum of `balances`
- `balances` must be non-empty; recommended `balances.len() <= 8` due to tx size limits
- Each created account is set on-chain, balance recorded, and UTXO tracked
- Sender’s balance and on-chain flag are updated accordingly (may become off-chain if fully spent)

---

## 6 • Trading Operations

### 6.1 Opening Trading Positions

```rust
use nyks_wallet::relayer_module::relayer_types::{OrderType, PositionType};

#[tokio::main]
async fn main() -> Result<(), String> {
    let mut order_wallet = OrderWallet::new(None)?;
    let (_, account_index) = order_wallet.funding_to_trading(6_000).await?;

    // Open a LONG position with 10x leverage at market price
    let request_id = order_wallet
        .open_trader_order(
            account_index,
            OrderType::MARKET,
            PositionType::LONG,
            50_000, // entry price (> 0)
            10,     // leverage: must be > 0; upper bound enforced by risk engine
        )
        .await?;

    println!("Order submitted: {}", request_id);
    Ok(())
}
```

Validations:

- `leverage` must be greater than 0 (upper bound is enforced dynamically by the relayer risk engine against `params.max_leverage` in `get_market_stats`)
- Account must be on-chain in Coin state
- Pre-submission pipeline (via `validate_open_order`) mirrors the server-side risk engine and rejects the call before any RPC if it would fail:
  1. Market status (HALT / CLOSE_ONLY)
  2. Max leverage (`params.max_leverage`)
  3. Min position size (`params.min_position_btc`)
  4. Per-position cap (`params.max_position_pct * pool_equity`)
  5. Directional headroom (`max_long_btc` / `max_short_btc`)

Effects:

- On success, IO type set to `Memo` (funds locked in order)
- `request_ids[index]` is updated
- Account state is synced before submission

#### 6.1.1 Pre-flight market / risk checks

Call these directly if you want to probe the relayer before deciding to submit:

```rust
order_wallet.validate_market_not_halted().await?;
order_wallet
    .validate_open_order(&PositionType::LONG, initial_margin, leverage)
    .await?;
```

### 6.2 Querying Orders

```rust
use nyks_wallet::relayer_module::relayer_types::OrderStatus;
let order = order_wallet.query_trader_order(account_index).await?;
println!("status: {:?}", order.order_status);
```

Statuses include: `PENDING`, `FILLED`, `SETTLED`, `CANCELLED`, `LIQUIDATE`.

Enhanced and historical views:

- `query_trader_order_v1(index)` – returns `TraderOrderV1` with `settle_limit`, `stop_loss`, `take_profit`, `funding_applied`
- `query_lend_order_v1(index)` – returns `LendOrderV1` with unrealised profit and APR
- `historical_trader_order(index) -> Vec<TraderOrder>`
- `historical_lend_order(index) -> Vec<LendOrder>`
- `order_funding_history(index) -> Vec<FundingHistoryEntry>`

If the query fails and the underlying tx status is terminal-but-not-viable (not PENDING/FILLED/LIQUIDATE), `query_trader_order` auto-unlocks the account back to `Coin` via `unlock_failed_order` and returns an error with the reason.

### 6.3 Closing Positions

```rust
use nyks_wallet::relayer_module::relayer_types::OrderType;
// Market close (execution_price = 0.0)
let close_id = order_wallet
    .close_trader_order(account_index, OrderType::MARKET, 0.0)
    .await?;
```

Requirements and effects:

- If the order is already `SETTLED` or `LIQUIDATE`, `close_trader_order` short-circuits to `unlock_trader_order` and just settles the account locally
- Otherwise the current order must be `FILLED`
- After settlement, account IO type becomes `Coin`
- Balance is refreshed from the returned `available_margin`
- UTXO details are updated to Coin state

#### 6.3.1 Close with stop-loss / take-profit

```rust
let request_id = order_wallet
    .close_trader_order_sltp(
        account_index,
        OrderType::LIMIT,
        execution_price,   // 0.0 for MARKET
        Some(stop_loss),   // or None
        Some(take_profit), // or None
    )
    .await?;
```

- Requires the order to be `FILLED`
- Same auto-unlock behavior as `close_trader_order` if the order is already `SETTLED`/`LIQUIDATE`

### 6.4 Canceling Orders

```rust
let cancel_id = order_wallet.cancel_trader_order(account_index).await?;
```

- Allowed when the order is `PENDING` (LIMIT open) or has an outstanding settle limit (close LIMIT)
- For pending LIMIT opens: on confirmed cancel, IO type becomes `Coin`; balance is unchanged (margin was never locked on the server)
- For close-limit cancels: the close request is withdrawn; the position remains `FILLED`

#### 6.4.1 Cancel stop-loss / take-profit

```rust
let request_id = order_wallet
    .cancel_trader_order_sltp(account_index, cancel_sl, cancel_tp)
    .await?;
```

- Requires the order to be `FILLED`
- Must have at least one of SL or TP attached; use the boolean flags to select which to cancel

#### 6.4.2 Manual unlock helpers

When using `--no-wait` style flows or recovering from a failed submission, these helpers reconcile local state with the chain:

- `unlock_trader_order(index) -> Result<(OrderStatus, String), String>` – verify a closed/liquidated trader order has settled on chain and return the account to `Coin`
- `unlock_lend_order(index) -> Result<(OrderStatus, String), String>` – same for a settled lend order
- `unlock_failed_order(index) -> Result<(), String>` – best-effort local recovery when a submission failed before the account could transition to `Memo` cleanly

### 6.5 Order Status Lifecycle

```
PENDING     →   FILLED    →     SETTLED
   ↓               ↓
CANCELLED      LIQUIDATE
```

---

## 7 • Lending Operations

### 7.1 Open Lend Orders

```rust
let request_id = order_wallet.open_lend_order(account_index).await?;
```

Effects:

- Uses entire account balance as lend amount
- IO type set to `Memo`
- UTXO details recorded as Memo

### 7.2 Query Lend Orders

```rust
let lend = order_wallet.query_lend_order(account_index).await?;
println!("status: {:?}, amount: {}", lend.order_status, lend.new_lend_state_amount);
```

### 7.3 Close Lend Orders

```rust
let close_id = order_wallet.close_lend_order(account_index).await?;
```

- If the lend order is already `SETTLED`, `close_lend_order` short-circuits to `unlock_lend_order`
- Otherwise requires current status `FILLED`
- On success: status `SETTLED`, IO type becomes `Coin`, balance set to `new_lend_state_amount`

---

## 8 • Account Management

Each ZK account tracks:

- **Balance**: Available satoshis
- **On-chain status**: Whether account exists on blockchain
- **IO Type**: `Coin` (spendable) or `Memo` (locked in order)

Typical transitions:

```text
funding_to_trading → Coin(on-chain)
open_(trader|lend) → Memo(locked)
close/cancel       → Coin(on-chain)
trading_to_trading → Coin(old off-chain), Coin(new on-chain)
```

Example:

```rust
use nyks_wallet::relayer_module::relayer_types::IOType;
// Move funds between ZK accounts for strategy isolation
let new_account = order_wallet.trading_to_trading(old_account_index).await?;

// Verify account states
let old_account = order_wallet.zk_accounts.get_account(&old_account_index)?;
let new_account_acc = order_wallet.zk_accounts.get_account(&new_account)?;

assert_eq!(old_account.on_chain, false); // Old account spent
assert_eq!(new_account_acc.on_chain, true);  // New account created
assert_eq!(new_account_acc.io_type, IOType::Coin);
```

---

## 9 • Database Persistence (optional)

These APIs are compiled only when `sqlite` or `postgresql` features are enabled.

### 9.1 Enable persistence and save state

```rust
use secrecy::SecretString;

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

- Function argument `Some(SecretString)`
- Environment variable `NYKS_WALLET_PASSPHRASE`
- Interactive prompt (terminal input)

#### 9.1.2 Wallet ID selection

- If provided, `wallet_id` is used as the database key
- If `None`, the wallet ID defaults to the wallet’s Twilight address
- Wallet ID must be unique; enabling persistence with an existing wallet ID errors

Behavior:

- Encrypted wallet stored using AES-GCM with key derived from passphrase
- ZK accounts upserted on create/update and during Drop
- UTXO details and request IDs synced on updates and during Drop

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
let mut order_wallet = OrderWallet::load_from_db(wallet_id, None, None)?;
```

### 9.3 List stored wallets

```rust
let list = OrderWallet::get_wallet_list_from_db(None)?;
for w in list { println!("{} {}", w.wallet_id, w.created_at); }
```

---

## 10 • Environment Configuration

Most endpoint defaults are selected by `NETWORK_TYPE` — the columns below show the **mainnet** and **testnet** defaults picked by `src/config.rs`. Override any value by exporting the variable explicitly.

| Variable                     | Default (mainnet)                       | Default (testnet)                      | Description                                                  |
| ---------------------------- | --------------------------------------- | -------------------------------------- | ------------------------------------------------------------ |
| `NETWORK_TYPE`               | `mainnet`                               | `mainnet`                              | `mainnet` or `testnet`; selects defaults for other endpoints |
| `BTC_NETWORK_TYPE`           | `mainnet`                               | `mainnet`                              | `mainnet` or `testnet` for BTC endpoints; nyks chain only supports BTC mainnet — keep this as `mainnet` even when `NETWORK_TYPE=testnet` |
| `CHAIN_ID`                   | `nyks`                                  | `nyks`                                 | Target blockchain chain ID                                   |
| `RUST_LOG`                   | –                                       | –                                      | Logging level (e.g. `info`, `debug`)                         |
| `RUST_BACKTRACE`             | –                                       | –                                      | Enable Rust backtrace (`1` or `full`)                        |
| `NYKS_RPC_BASE_URL`          | `https://rpc.twilight.org`              | `https://rpc.twilight.rest`            | Nyks chain Tendermint RPC endpoint                           |
| `NYKS_LCD_BASE_URL`          | `https://lcd.twilight.org`              | `https://lcd.twilight.rest`            | Nyks chain LCD REST endpoint                                 |
| `FAUCET_BASE_URL`            | *(empty)*                               | `https://faucet-rpc.twilight.rest`     | Faucet for test tokens (testnet only)                        |
| `RELAYER_API_RPC_SERVER_URL` | `https://api.ephemeral.fi/api`          | `https://relayer.twilight.rest/api`    | Relayer public JSON-RPC API (required for order-wallet)      |
| `ZKOS_SERVER_URL`            | `https://zkserver.twilight.org`         | `https://nykschain.twilight.rest/zkos` | ZkOS server endpoint                                         |
| `TWILIGHT_INDEXER_URL`       | `https://indexer.twilight.org`          | `https://indexer.twilight.rest`        | Twilight indexer endpoint                                    |
| `BTC_ESPLORA_PRIMARY_URL`    | `https://blockstream.info/api`          | `https://blockstream.info/testnet/api` | Primary Esplora API for BTC queries (driven by `BTC_NETWORK_TYPE`) |
| `BTC_ESPLORA_FALLBACK_URL`   | `https://mempool.space/api`             | `https://mempool.space/testnet/api`    | Fallback Esplora API (driven by `BTC_NETWORK_TYPE`)          |
| `RELAYER_PROGRAM_JSON_PATH`  | `./relayerprogram.json`                 | `./relayerprogram.json`                | Path to relayer program JSON                                 |
| `VALIDATOR_WALLET_PATH`      | `validator.mnemonic`                    | `validator.mnemonic`                   | Validator mnemonic file (validator-wallet feature)           |
| `NYKS_WALLET_PASSPHRASE`     | –                                       | –                                      | Wallet passphrase; leave unset to use interactive prompt     |
| `WALLET_ID`                  | –                                       | –                                      | Optional wallet ID (defaults to Twilight address if not set) |
| `DATABASE_URL_SQLITE`        | `./wallet_data.db`                      | `./wallet_data.db`                     | SQLite database file path (feature `sqlite`)                 |
| `DATABASE_URL_POSTGRESQL`    | –                                       | –                                      | PostgreSQL connection string (feature `postgresql`)          |

Example local development setup:

```bash
export RUST_LOG="info,debug"
export RUST_BACKTRACE=full
export NETWORK_TYPE=testnet                 # picks *.twilight.rest endpoints
export BTC_NETWORK_TYPE=mainnet              # nyks chain only supports BTC mainnet
export CHAIN_ID=nyks
# Override individual endpoints only if pointing to a local full-node:
# export NYKS_RPC_BASE_URL="http://0.0.0.0:26657"
# export NYKS_LCD_BASE_URL="http://0.0.0.0:1317"
# export FAUCET_BASE_URL="http://0.0.0.0:6969"
# export RELAYER_API_RPC_SERVER_URL="http://0.0.0.0:8088/api"
# export ZKOS_SERVER_URL="http://0.0.0.0:3030"
export RELAYER_PROGRAM_JSON_PATH="./relayerprogram.json"
export NYKS_WALLET_PASSPHRASE="test1_password"
export DATABASE_URL_SQLITE="./wallet_data.db"
```

Example production (mainnet) setup — defaults are already correct, typically only secrets and program path need to be set:

```bash
export RUST_LOG=info
export NETWORK_TYPE=mainnet
export BTC_NETWORK_TYPE=mainnet
export CHAIN_ID=nyks
export RELAYER_PROGRAM_JSON_PATH="/path/to/relayerprogram.json"
export NYKS_WALLET_PASSPHRASE="<secure-passphrase>"
```

---

## 11 • Error Handling

Common errors and resolutions:

- "Insufficient balance" → top up wallet or reduce size
- "Account does not exist on chain or has no balance" / "Account is locked, io type: …" → wait for `funding_to_trading` confirmation, or the account is currently in `Memo` state
- "Leverage must be greater than 0" → fix parameter (upper bound comes from risk-engine validation, surfaced as "Leverage X exceeds maximum allowed Y")
- "Market is halted: …" / "Market is in close-only mode: …" → relayer market guard; retry when market resumes
- "Position size … is below minimum …" / "… exceeds per-position cap …" / "… exceeds max available long/short capacity …" → risk-engine rejection from `validate_open_order`
- "Order is not filled, status: …" (on close) → wait for fill or cancel; if status is `SETTLED`/`LIQUIDATE`, `close_trader_order` will auto-unlock
- "Order is not pending or close limit, status: …" (on cancel) → only PENDING opens or outstanding close-limits can be cancelled
- UTXO/TxHash fetch failures → network hiccups; automatic retries are included

Robust retry example:

```rust
use nyks_wallet::relayer_module::relayer_types::{OrderType, PositionType};
use tokio::time::{sleep, Duration};

async fn robust_trading_operation(order_wallet: &mut OrderWallet, account_index: u64) -> Result<String, String> {
    let max_retries = 3;
    for attempt in 1..=max_retries {
        match order_wallet
            .open_trader_order(account_index, OrderType::MARKET, PositionType::LONG, 50_000, 10)
            .await
        {
            Ok(request_id) => return Ok(request_id),
            Err(e) if attempt < max_retries => {
                eprintln!("Attempt {} failed: {}", attempt, e);
                sleep(Duration::from_secs(2_u64.pow(attempt))).await;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

---

## 12 • Testing Examples

### 12.1 Complete Trading Flow

```rust
use nyks_wallet::relayer_module::relayer_types::{OrderStatus, OrderType, PositionType};

#[tokio::test]
async fn test_complete_trading_cycle() -> Result<(), String> {
    env_logger::init();
    let mut order_wallet = OrderWallet::new(None)?;

    // 1. Fund trading account
    let (tx, idx) = order_wallet.funding_to_trading(10_000).await?;
    assert_eq!(tx.code, 0);

    // 2. Open leveraged position
    let _req = order_wallet
        .open_trader_order(idx, OrderType::MARKET, PositionType::LONG, 50_000, 5)
        .await?;

    // 3. Verify order filled
    let order = order_wallet.query_trader_order(idx).await?;
    assert_eq!(order.order_status, OrderStatus::FILLED);

    // 4. Close position
    let _close_request = order_wallet
        .close_trader_order(idx, OrderType::MARKET, 0.0)
        .await?;

    // 5. Verify settlement
    let final_order = order_wallet.query_trader_order(idx).await?;
    assert_eq!(final_order.order_status, OrderStatus::SETTLED);

    Ok(())
}
```

### 12.2 Automated Market Making

```rust
use nyks_wallet::relayer_module::relayer_types::{OrderType, PositionType};

async fn market_making_strategy(
    order_wallet: &mut OrderWallet,
    base_price: u64,
    spread_pct: f64,
) -> Result<(), String> {
    // Create new account for each side
    let (_, long_idx) = order_wallet.funding_to_trading(10_000).await?;
    let (_, short_idx) = order_wallet.funding_to_trading(10_000).await?;

    let spread = (base_price as f64 * spread_pct / 100.0) as u64;

    // Open long position below market
    let _long_req = order_wallet
        .open_trader_order(long_idx, OrderType::LIMIT, PositionType::LONG, base_price - spread, 1)
        .await?;

    // Open short position above market
    let _short_req = order_wallet
        .open_trader_order(short_idx, OrderType::LIMIT, PositionType::SHORT, base_price + spread, 1)
        .await?;

    Ok(())
}
```

### 12.3 Lending Strategy

```rust
async fn lending_strategy(order_wallet: &mut OrderWallet, account_index: u64) -> Result<(), String> {
    // Open lending position
    let _lend_request = order_wallet.open_lend_order(account_index).await?;

    // Monitor lending status
    let _lend_order = order_wallet.query_lend_order(account_index).await?;

    // Close lending position
    let _close_request = order_wallet.close_lend_order(account_index).await?;
    Ok(())
}
```

---

## Further Reading

- Main README – Overview of nyks-wallet capabilities
- Quick Start – Basic wallet setup and funding
- Deployment Guide – Production deployment instructions
- Twilight Client SDK – ZkOS and QuisQuis primitives
- Relayer Core – High-performance matching engine

---

**License**: Released under the Apache License – see [LICENSE](LICENSE) for details.
