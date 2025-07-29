# Validator Wallet Feature

This document explains how to compile **nyks-wallet** with the optional `validator-wallet` feature and how to broadcast validator-side transactions programmatically.

---

## 1. Enabling the Feature in Your Project

Add **nyks-wallet** as a dependency and enable the feature:

```toml
[dependencies]
nyks-wallet = { git = "https://github.com/twilight-project/nyks-wallet.git", features = ["validator-wallet"] }
```

If you are working inside this repository, build with:

```bash
# Debug build
cargo build --features validator-wallet

# Release build
cargo build --release --features validator-wallet
```

---

## 2. Wallet Prerequisites

The validator wallet requires a mnemonic file to sign transactions.  
By **default** the crate looks for `validator.mnemonic` in the current directory.

_Override the path_ by setting an environment variable before runtime:

```bash
export VALIDATOR_WALLET_PATH=/absolute/path/to/your_validator.mnemonic
export NYKS_RPC_BASE_URL=http://your_chain_ip_or_domain:26657
```

---

## 3. Public API

Two asynchronous helper functions are exposed when the feature is turned on:

```rust
use nyks_wallet::validator_wallet::{transfer_tx, mint_burn_trading_btc_tx};
```

### 3.1 `transfer_tx`

```rust
pub async fn transfer_tx(
    tx_id: String,
    tx_byte_code: String,
    tx_fee: u64,
) -> Result<(String, u32), String>
```

Broadcasts a `MsgTransferTx`.

| Parameter      | Description                                       |
| -------------- | ------------------------------------------------- |
| `tx_id`        | ID string used to identify the transfer on-chain. |
| `tx_byte_code` | Hex/base64-encoded byte-code of the zk-proof.     |
| `tx_fee`       | Fee to be paid (denominated in `nyks`).           |

Returns the **transaction hash** and **Tendermint code** once the RPC node acknowledges the broadcast.

### 3.2 `mint_burn_trading_btc_tx`

```rust
pub async fn mint_burn_trading_btc_tx(
    mint_or_burn: bool,
    btc_value: u64,
    qq_account: String,
    encrypt_scalar: String,
) -> Result<(String, u32), String>
```

Broadcasts a `MsgMintBurnTradingBtc` transaction.

| Parameter        | Description                               |
| ---------------- | ----------------------------------------- |
| `mint_or_burn`   | `true` = mint, `false` = burn.            |
| `btc_value`      | Amount in satoshis.                       |
| `qq_account`     | QuisQuis‐encoded account string.          |
| `encrypt_scalar` | Scalar used in the encryption commitment. |

Like `transfer_tx`, it returns a tuple `(tx_hash, tx_code)`.

---

## 4. Quick Example

```rust
use nyks_wallet::validator_wallet::transfer_tx;

#[tokio::main]
async fn main() -> Result<(), String> {
    // make sure VALIDATOR_WALLET_PATH is set or validator.mnemonic exists
    let (hash, code) = transfer_tx(
        "my-tx-id".into(),
        "<byte-code-base64>".into(),
        1_000u64,
    ).await?;

    println!("Broadcasted: {} (code {})", hash, code);
    Ok(())
}
```

---

## 5. Logging & Environment

- `RUST_LOG` – control log level (e.g. `info`, `debug`).
- `.env` – you can keep RPC endpoints, etc., here; they are loaded by `dotenv`.

---

## 6. Error Handling

All helpers return `Result<_, String>` so they can be chained with `?`.  
Common errors include:

- Missing mnemonic file (`VALIDATOR_WALLET_PATH`).
- RPC connectivity issues.
- Invalid message parameters (bad base64, etc.).

---

## 7. Removing the Feature

If you do **not** need validator functionality, omit the feature flag;
none of the additional code or dependencies will be compiled.
