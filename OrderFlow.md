# Simple Market Maker – Order Flow with Nyks Wallet

> Pool‑based **inverse perpetual** on Twilight Protocol using ZkOS “trading (dark) accounts”.

This guide shows the minimal, end‑to‑end flow to place and manage trading and lending orders with the **Nyks Wallet** and `OrderWallet` helper, plus tips for bots that fan out to multiple accounts.

---

## Prerequisites

- Rust (stable), Tokio runtime
- Access to Twilight **testnet** endpoints
- Nyks Wallet / Relayer client crates in your project
- If you enable DB persistence, compile with `sqlite` or `postgresql` features

### Environment (Testnet)

follow the .env.example file

---

## High‑level Lifecycle

1. **Create** an `OrderWallet`
2. **Fund** the base wallet with **test sats** (50,000 sats)
3. **Fund trading (dark) account**: move sats → new ZkOS account (`IOType::Coin`)
4. **Open order** (trade or lend) — account transitions `Coin → Memo`
5. **Close order** (or **cancel** limit order) — account transitions `Memo → Coin`
6. **Rotate account** for re‑use (trade→trade) or **return to funding** (trade→funding)

> **Full‑amount rule:** opening an order uses **the entire balance** of the selected ZkOS account. Partial orders aren’t supported.

---

## Step‑by‑Step with Code

### 1) Create an `OrderWallet`

```rust
use nyks_wallet::relayer_module::order_wallet::OrderWallet;

// Defaults to Twilight testnet endpoints via EndpointConfig::default()
let mut order_wallet = OrderWallet::new(None)?;

// Optional: import from mnemonic
// let mut order_wallet = OrderWallet::import_from_mnemonic(mnemonic, None)?;

// Optional: enable DB persistence (prompts for passphrase if None)
// order_wallet.with_db(None, None)?;
```

### 2) Request test tokens (50,000 sats)

```rust
use nyks_wallet::wallet::get_test_tokens;

get_test_tokens(&mut order_wallet.wallet).await?; // mints ~50,000 test sats
```

### 3) Create a **trading (dark) account** and fund it

Transfers from the on‑chain funding account into a newly derived ZkOS account.

```rust
let deposit: u64 = 6_000; // sats to move into the new ZkOS account
let (tx_result, account_index) = order_wallet.funding_to_trading(deposit).await?;
assert_eq!(tx_result.code, 0, "chain tx failed");
// The new account at `account_index` is on‑chain with IOType::Coin
```

### 4) Open an order

#### 4a) Trader (perp) order

```rust
use twilight_client_sdk::relayer_types::{OrderType, PositionType};

let entry_price: u64 = /* e.g., oracle/price feed */ 65_000; // USD price in whole units
let leverage: u64 = 10; // 1..=50

let request_id = order_wallet
    .open_trader_order(
        account_index,            // ZkOS account to consume (full balance)
        OrderType::MARKET,        // or OrderType::LIMIT
        PositionType::LONG,       // or PositionType::SHORT
        entry_price,
        leverage,
    )
    .await?;

// For MARKET, the account immediately moves Coin -> Memo after fill.
// For LIMIT, status is PENDING until price is hit or you cancel.
```

##### Query / Cancel (LIMIT only)

```rust
// Cancel only works while order status is PENDING
let cancel_request_id = order_wallet.cancel_trader_order(account_index).await?;
// After a successful cancel, the account is Coin again and CAN be reused without rotation.
```

#### 4b) Lend order

```rust
let request_id = order_wallet.open_lend_order(account_index).await?;
// Account moves Coin -> Memo while the lend is active.
```

### 5) Close the order

#### Trader close (requires FILLED position)

```rust
use twilight_client_sdk::relayer_types::OrderType;

// Close at MARKET or LIMIT (execution_price ignored for MARKET)
let close_req_id = order_wallet
    .close_trader_order(
        account_index,
        OrderType::MARKET,   // or OrderType::LIMIT
        0.0,                 // execution_price if LIMIT close
    )
    .await?;

// On success (SETTLED):
// - Account transitions Memo -> Coin
// - Balance (available margin + PnL) is updated in ZkAccountDB
```

#### Lend close (requires FILLED lend)

```rust
let close_req_id = order_wallet.close_lend_order(account_index).await?;
// On success (SETTLED): account goes Memo -> Coin and balance is updated.
```

### 6) Account reuse rules (very important)

- **You cannot open a NEW order with an account that has already been used** (opened and then closed/settled or liquidated). Twilight enforces account freshness.
- You have two options:

  1. **Rotate trading account → trading account** (new index & keys)

     ```rust
     let new_index = order_wallet.trading_to_trading(account_index).await?;
     // old account: off‑chain; new account: on‑chain Coin with same balance
     ```

  2. **Trading → funding** (return balance to on‑chain funding wallet)

     ```rust
     // If your build exposes: order_wallet.trading_to_funding(index).await?
     // This deletes the ZkOS account and sends the balance back to the base wallet.
     ```

- **Exception:** If you **cancel a LIMIT order** (status becomes `CANCELLED`), you may reuse **the same account**; **no rotation** needed because it never settled on‑chain.

---

## Minimal “Hello Trade” Example

```rust
use nyks_wallet::relayer_module::order_wallet::OrderWallet;
use twilight_client_sdk::relayer_types::{OrderType, PositionType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let mut ow = OrderWallet::new(None)?;            // 1) create
    nyks_wallet::wallet::get_test_tokens(&mut ow.wallet).await?; // 2) faucet

    let (_tx, idx) = ow.funding_to_trading(6_000).await?;       // 3) fund ZkOS

    // 4) open MARKET long @ current price (example only)
    let price = 65_000u64;
    let rid = ow
        .open_trader_order(idx, OrderType::MARKET, PositionType::LONG, price, 10)
        .await?;

    // ... wait for fill (MARKET typically fills immediately). Optionally poll:
    let info = ow.query_trader_order(idx).await?;
    assert_eq!(info.order_status.to_string(), "FILLED");

    // 5) close position at MARKET
    let close_rid = ow.close_trader_order(idx, OrderType::MARKET, 0.0).await?;

    // 6) rotate for next trade (required for reused accounts)
    let next_idx = ow.trading_to_trading(idx).await?;

    println!("opened: {} closed: {} next_idx: {}", rid, close_rid, next_idx);
    Ok(())
}
```

---

## Bot‑friendly: Create multiple accounts in one shot

When building a simple market‑maker or grid bot, you can split one funded account into several child accounts, each receiving a target balance. Use **`trading_to_trading_multiple_accounts`**:

```rust
use nyks_wallet::relayer_module::order_wallet::AccountBalance; // (AccountIndex, Balance)

let sender = account_index; // funded ZkOS account (Coin, on‑chain)
let splits = vec![5_000, 1_000, 8_000, 600];
let accounts: Vec<AccountBalance> =
    order_wallet.trading_to_trading_multiple_accounts(sender, splits).await?;

for (idx, bal) in accounts {
    // place independent orders on each `idx` for your strategy
}
```

**Preconditions / tips**

- Sender must be **on‑chain**, `IOType::Coin`, with balance ≥ sum(splits)
- Keep the number of new accounts per call modest (e.g., ≤ 8) to respect tx size limits
- After the fan‑out, the sender may remain on‑chain (if remainder > 0) or become off‑chain

---

## Query helpers & statuses

- `query_trader_order(index)` → `TraderOrder`
- `query_lend_order(index)` → `LendOrder`
- Common statuses: `PENDING` → `FILLED` → `SETTLED` → (`CANCELLED` for limit cancels)
- `cancel_trader_order(index)` only allowed while **PENDING**

---

## Common errors & how to fix

- **Insufficient balance**: Ensure faucet succeeded and `funding_to_trading(amount)` ≤ wallet sats
- **Leverage/price validation**: `leverage` must be 1..=50; `entry_price` > 0
- **Wrong IOType**: Opening requires `Coin`; closing expects a previously **FILLED** order (`Memo` → `Coin`)
- **Order not FILLED/SETTLED**: The close path checks relayer status and will error if not ready
- **Missing env**: endpoints above must be set; faucet/relayer calls will fail otherwise
- **DB wallet_id exists**: choose a unique `wallet_id` when calling `with_db(..)` or persistence helpers

---

## Quick Reference (API surface)

| Method                                                              | Use for                         | Returns                                 |
| ------------------------------------------------------------------- | ------------------------------- | --------------------------------------- |
| `OrderWallet::new(None)`                                            | Create a fresh wallet           | `OrderWallet`                           |
| `get_test_tokens(&mut wallet)`                                      | Mint test sats                  | `()`                                    |
| `funding_to_trading(amount)`                                        | Create & fund a ZkOS account    | `(TxResult, AccountIndex)`              |
| `open_trader_order(index, order_type, side, entry_price, leverage)` | Open perp order (full balance)  | `RequestId`                             |
| `cancel_trader_order(index)`                                        | Cancel **PENDING** limit order  | `RequestId`                             |
| `close_trader_order(index, order_type, execution_price)`            | Close filled order              | `RequestId`                             |
| `open_lend_order(index)` / `close_lend_order(index)`                | Lend flow                       | `RequestId`                             |
| `query_trader_order(index)` / `query_lend_order(index)`             | Inspect order state             | `TraderOrder` / `LendOrder`             |
| `trading_to_trading(index)`                                         | Rotate used account → fresh one | `AccountIndex`                          |
| `trading_to_trading_multiple_accounts(sender, balances)`            | Fan‑out to many accounts        | `Vec<(AccountIndex, Balance)>`          |
| _(optional)_ `trading_to_funding(index)`                            | Return funds to base wallet     | `TxResult` _(implementation dependent)_ |

---

## Notes

- **Inverse perp math**: Margin is in sats; `position_value = margin × leverage`. `position_size` is computed against `entry_price`.
- **Account state transitions** are enforced on‑chain via ZkOS `IOType`: `Coin ↔ Memo`.
- After **cancelled** limit orders, reuse the same account (no rotation needed) because it never settled on‑chain.

---

## Troubleshooting Checklist

- ✅ Faucet succeeded and wallet `sats` ≥ deposit
- ✅ `funding_to_trading` returned `code == 0` and you see `IOType::Coin`
- ✅ For closing, prior status is `FILLED` (then expect `SETTLED`)
- ✅ Rotated (trade→trade) before attempting a brand‑new order with a used account
- ✅ Env vars match Twilight testnet endpoints

---

_Last updated: 2025‑08‑13 (Asia/Kolkata)._
