# Nyks Wallet SDK — Database Features Branch

## Overview

This document provides a high-level overview of the Nyks Wallet SDK on the `database-features` branch. Nyks Wallet is a **pure Rust SDK** that wraps the low-level gRPC/REST and cryptographic plumbing required to interact with the Twilight (Nyks) blockchain. It allows you to create or import Cosmos-compatible accounts, request Nyks tokens from the public faucet, register Bitcoin deposit addresses and mint/burn assets on-chain, work with ZkOS/QuisQuis shielded accounts, and craft, sign and broadcast custom protobuf messages asynchronously.

The `database-features` branch extends the SDK by adding optional persistence using [Diesel](https://diesel.rs/) and `r2d2` connection pooling. When enabled, the SDK can store shielded accounts, encrypted wallet seeds, order wallet information, UTXO details and relayer request IDs in a SQLite or PostgreSQL database. The default build uses SQLite; to use PostgreSQL, enable the `postgresql` feature at compile time.

## Package Structure and Functionality

- **See `README.md` for a full tour of _wallet_, _relayer_module_, _zkos_accounts_, _nyks_rpc_, etc.**  
  This document focuses **only on the extra components introduced by the `database-features` branch**.

### `database` (feature-gated)

| Sub-module   | Purpose                                                                                                                      |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------- |
| `connection` | Creates a Diesel connection pool (SQLite ⇄ PostgreSQL) and runs one-time migrations.                                         |
| `schema`     | Compile-time Diesel table declarations.                                                                                      |
| `models`     | Rust structs that mirror the DB tables (`zk_accounts`, `encrypted_wallets`, `order_wallets`, `utxo_details`, `request_ids`). |
| `operations` | High-level CRUD via `DatabaseManager` – save / update ZkOS accounts, encrypted seeds, UTXO data & relayer request IDs.       |

### `security`

Password & key-derivation helpers (AES-256-GCM, salted KDF) used to encrypt wallet seeds before they touch disk.

### Build Flags

Enable **exactly one** of the following when compiling:

```bash
# Default (SQLite)
cargo build --release --features sqlite

# PostgreSQL
a) export DATABASE_URL_POSTGRESQL=postgres://user:pass@host/db
b) cargo build --release --no-default-features --features postgresql
```

---

## Quick Start (Database)

```bash
# Add the library (SQLite example)
[dependencies]
nyks-wallet = { git = "https://github.com/twilight-project/nyks-wallet", features = ["sqlite"] }
```

```rust
use nyks_wallet::relayer_module::order_wallet::OrderWallet;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Securely persist everything with a passphrase
    let mut ow = OrderWallet::new(None)?
        .with_db(Some("💡 choose-a-strong-passphrase".into()), None)?;

    ow.save_order_wallet_to_db()?; //-> encrypted seed & config are now in SQLite
    Ok(())
}
```

---

## Database Environment Variables

Only two additional variables are required – all other endpoints are identical to the standard build:
| Variable | Example / Default | Description |
| ------------------------- | -------------------------------- | ------------------------------ |
| `DATABASE_URL_SQLITE` | `./wallet_data.db` (default) | Path to SQLite file |
| `DATABASE_URL_POSTGRESQL` | `postgres://user:pass@host/db` | PostgreSQL connection string |

---

## Further Reading

- **README.md** – complete architecture & non-DB modules.
- **QuickStart.md** – build & fund a test wallet in seconds.
- **DEPLOYMENT.md** – containerised deployment & advanced relayer setup.
- **Validator-Wallet.md** – validator key-management helpers.
- **twilight-client-sdk** – Rust primitives for QuisQuis & ZkOS.
