# Nyks Wallet SDK — Database Features Branch

## Overview

This document provides a high-level overview of the Nyks Wallet SDK on the `database-features` branch. Nyks Wallet is a **pure Rust SDK** that wraps the low-level gRPC/REST and cryptographic plumbing required to interact with the Twilight (Nyks) blockchain. It allows you to create or import Cosmos-compatible accounts, request Nyks tokens from the public faucet, register Bitcoin deposit addresses and mint/burn assets on-chain, work with ZkOS/QuisQuis shielded accounts, and craft, sign and broadcast custom protobuf messages asynchronously.

The `database-features` branch extends the SDK by adding optional persistence using [Diesel](https://diesel.rs/) and `r2d2` connection pooling. When enabled, the SDK can store shielded accounts, encrypted wallet seeds, order wallet information, UTXO details and relayer request IDs in a SQLite or PostgreSQL database. The default build uses SQLite; to use PostgreSQL, enable the `postgresql` feature at compile time.

## Package Structure and Functionality

The SDK is structured as a set of modules. The following sections summarise the purpose of each package/module.

### `wallet`

Located under `src/wallet/`, this module contains the `Wallet` struct and helpers for managing Cosmos + BTC keys, balances and transactions. Key capabilities include:

- **Lifecycle** – create a new wallet with a random BTC address (`create_new_with_random_btc_address`), import an existing wallet from a mnemonic or private key, and serialize/deserialize to JSON for long-term storage.
- **Balance management** – query the on-chain balance (`check_balance`) or refresh the embedded `balance_nyks` and `balance_sats` fields with `update_balance`.
- **Faucet helpers** – request Nyks tokens and test satoshis via `get_nyks`/`mint_sats`/`mint_sats_5btc`.
- **BTC deposit registration & trading** – sign and broadcast a `MsgRegisterBtcDepositAddress` for user deposits and craft mint/burn messages through `nyks_fn::create_funiding_to_trading_tx_msg` and `nyks_fn::send_tx`.
- **Seed signer** – derive deterministic Ristretto seeds from ADR-036 signatures via the `seed_signer` submodule.

### `nyks_rpc`

This module implements a minimal JSON-RPC client and message encoder. It defines an exhaustive `Method` enum of Tendermint RPC calls and helpers such as `MethodTypeURL::sign_msg` for packing payloads, configuring fees and signing secp256k1 messages. Applications rarely need to use `nyks_rpc` directly; the higher-level `wallet` functions wrap these calls.

### `zkos_accounts`

Utilities for working with QuisQuis/ZkOS shielded accounts. The `ZkAccountDB` type manages an in-memory collection of shielded accounts and supports generating new accounts, deriving them from signatures and serialising/deserialising the resulting structures. The `EncryptedAccount` helpers provide encoding/decoding, key-pair verification and balance decryption. In the database branch, shielded accounts can be persisted via the `zk_accounts` table (see the **Database Module** below).

### `config`

Defines `EndpointConfig`, `WalletEndPointConfig` and `RelayerEndPointConfig` for configuring RPC, LCD, faucet and relayer endpoints. Default values are derived from environment variables such as `NYKS_LCD_BASE_URL`, `NYKS_RPC_BASE_URL`, `FAUCET_BASE_URL`, `ZKOS_SERVER_URL`, `RELAYER_API_RPC_SERVER_URL` and `CHAIN_ID`. Helper methods allow reading these values from the environment (`from_env`), constructing new configurations, or converting between wallet and relayer endpoint formats.

### `relayer_module`

High-level trading and lending interface for the Twilight relayer. The `OrderWallet` type orchestrates ZkOS account funding, balance transfers, and order management. It wraps a base `Wallet`, an in-memory `ZkAccountDB` and a relayer JSON-RPC client, and exposes methods to:

- **Fund ZkOS accounts** from the on-chain wallet and move balances across accounts (single or multiple receivers).
- **Open, close or cancel trader and lend orders** via relayer endpoints, with retry helpers for fetching transaction hashes and UTXO details.
- **Persist runtime state** – when the `sqlite` or `postgresql` feature is enabled and `with_db()` is called, `OrderWallet` persists wallet seeds, account data, UTXO details and request IDs to the configured database. The mnemonic is securely printed once and never written to disk.

This module also defines `relayer_api`, `relayer_order` and `relayer_types` submodules for interfacing with the relayer RPC and defining order types, statuses and serialization.

### `database` (feature-gated)

The `database` module is only compiled when either the `sqlite` or `postgresql` feature flag is enabled. It consists of several submodules:

- **`connection`** – sets up a Diesel connection pool using `r2d2` and reads the database URL from environment variables. By default, SQLite is used and the database file is stored at `./wallet_data.db`. When the `postgresql` feature is enabled (and `sqlite` disabled), the module switches to a PostgreSQL connection. Helper functions `init_pool` and `get_conn` create the pool and acquire connections, while `run_migrations_once` ensures that migrations are executed exactly once per process.

- **`models`** – defines Diesel structs representing the persistent tables. Tables include `zk_accounts` (shielded accounts), `encrypted_wallets` (AES-GCM-encrypted wallet seeds), `order_wallets` (relayer order configuration), `utxo_details` (serialized UTXO responses) and `request_ids` (relayer request IDs). Each model includes helper functions to convert between in-memory types (e.g. `ZkAccount` or `UtxoDetailResponse`) and database structs.

- **`operations`** – exposes a `DatabaseManager` struct which wraps a connection pool and provides CRUD operations. Key methods include saving, updating and removing ZkOS accounts (`save_zk_account`, `update_zk_account`, `remove_zk_account`); saving and loading encrypted wallets and order wallets; saving and loading UTXO details; and managing relayer request IDs. A static `get_wallet_list` helper returns all wallet IDs that have encrypted data stored.

- **`schema`** – contains Diesel table definitions (generated via `diesel print-schema`) corresponding to the models. Migrations in `connection::run_migrations` create the tables if they do not exist for either SQLite or PostgreSQL, including setting WAL mode and timeouts for SQLite.

### `security`

This module provides utilities for secure password handling. The `SecurePassword` type derives encryption keys from passphrases using a salted key derivation function, and AES-GCM helpers perform authenticated encryption and decryption of sensitive data. `EncryptedWallet` and `DbOrderWallet` use these helpers to encrypt wallet seeds before writing them to the database.

### `validator_wallet` (optional)

When compiled with the `validator-wallet` feature flag, the SDK exposes helpers for creating and managing validator wallets. This feature is disabled by default and is only intended for validator operators.

## Database Features and Constraints

The following constraints apply when using the database functionality:

1. **Mutually exclusive features** – at compile time, enable exactly one of `sqlite` or `postgresql`. If both are enabled, the build defaults to SQLite and emits a deprecation warning.
2. **Environment variables** – set `DATABASE_URL_SQLITE` to the path of the SQLite database file (defaults to `./wallet_data.db`) or `DATABASE_URL_POSTGRESQL` to a PostgreSQL connection URI. The SDK reads these variables when initializing the connection pool.
3. **Migrations** – call `run_migrations_once(pool)` after creating the pool to ensure that all necessary tables are created. This function is idempotent and safe to call from multiple threads.
4. **Concurrency** – the default pool size is 15 connections with a minimum idle of 2 and an 8-second connection timeout for both SQLite and PostgreSQL. Adjust these parameters in `init_pool` if your workload demands different sizing.
5. **Secure storage** – wallet seeds are encrypted with a user-provided passphrase using AES-256-GCM before being stored in the `encrypted_wallets` table. The SDK never writes plain mnemonics to disk.

## Quick Start

Follow these steps to build, configure and run the SDK with database support. For a faster getting-started path without persistence, you can omit the database features and jump directly to step 4.

### 1. Prerequisites

| Tool                | Minimum version / notes                          |
| ------------------- | ------------------------------------------------ |
| Rust toolchain      | Nightly 2024-edition or stable ≥ 1.75            |
| Protobuf (`protoc`) | ≥ 3.0 – install via your package manager         |
| `git`               | Any recent version                               |
| OpenSSL headers     | On Debian/Ubuntu install `pkg-config libssl-dev` |

### 2. Clone and build

```bash
# Grab the source and switch to the database branch
git clone https://github.com/twilight-project/nyks-wallet.git
cd nyks-wallet
git checkout database-features

# Compile everything in release mode
cargo build --release --features sqlite
# To use PostgreSQL instead of SQLite, compile with:
# cargo build --release --no-default-features --features postgresql
```

The build produces the library and a sample binary `relayer_init` in `target/release/`.

### 3. Configure endpoints and database

Create a `.env` file (or set environment variables in your shell) containing the endpoints. At a minimum you should set:

```bash
NYKS_LCD_BASE_URL=https://lcd.twilight.rest
NYKS_RPC_BASE_URL=https://rpc.twilight.rest
FAUCET_BASE_URL=https://faucet-rpc.twilight.rest
ZKOS_SERVER_URL=https://nykschain.twilight.rest/zkos
RUST_LOG=info

# Optional database URLs
DATABASE_URL_SQLITE=./wallet_data.db            # path to SQLite DB
# or
DATABASE_URL_POSTGRESQL=postgres://user:pass@host:5432/dbname
```

Load the variables into your shell with `source .env` before running any binaries. The `config` module will automatically pick up these values if they are present.

### 4. Run the one-liner demo

After building, you can generate a fresh wallet, fund it with test-net tokens and deploy the relayer contract in a single command:

```bash
cargo run --bin relayer_init --features sqlite
```

On success, the program writes `relayer_deployer.json` to the working directory and prints log messages indicating that the wallet was funded and the relayer contract deployed.

### 5. Use the SDK in your own project

Add a dependency on `nyks-wallet` in your `Cargo.toml` and optionally select database features:

```toml
[dependencies]
nyks-wallet = { git = "https://github.com/twilight-project/nyks-wallet.git", branch = "database-features", features = ["sqlite"] }
```

Minimal example:

```rust
use nyks_wallet::wallet::{Wallet, get_test_tokens};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1️⃣ create a random Cosmos + BTC wallet
    let mut wallet = Wallet::create_new_with_random_btc_address()?;
    // 2️⃣ request test-net tokens (10 000 nyks & 50 000 sats)
    get_test_tokens(&mut wallet).await?;
    println!("Twilight address: {}", wallet.twilightaddress);
    println!("Balance: {} nyks, {} sats", wallet.balance_nyks, wallet.balance_sats);
    Ok(())
}
```

Compile and run with `cargo run -q`.

### 6. Enabling persistence

To persist wallet data, ZkOS accounts and UTXO details, call `with_db()` on an `OrderWallet` or create a `DatabaseManager` directly. Provide a passphrase to encrypt your seed – the SDK will derive an AES-256-GCM key from the passphrase and store the encrypted seed along with a salt and nonce in the `encrypted_wallets` table. After enabling persistence, you can:

- Insert or update shielded accounts via `save_zk_account` or `update_zk_account`.
- Save UTXO details and request IDs for later retrieval (`save_utxo_detail`, `save_request_id`).
- Deactivate order wallets when they are no longer needed (`deactivate_order_wallet`).

## Environment Variables

The SDK reads many endpoints and configuration values from the environment. The following variables are relevant across modules:

| Variable                     | Default                   | Purpose                                                                    |
| ---------------------------- | ------------------------- | -------------------------------------------------------------------------- |
| `NYKS_LCD_BASE_URL`          | `http://0.0.0.0:1317`     | Cosmos SDK LCD REST endpoint                                               |
| `NYKS_RPC_BASE_URL`          | `http://0.0.0.0:26657`    | Cosmos SDK Tendermint RPC endpoint                                         |
| `FAUCET_BASE_URL`            | `http://0.0.0.0:6969`     | Faucet and mint endpoints                                                  |
| `ZKOS_SERVER_URL`            | `http://0.0.0.0:3030`     | ZkOS/QuisQuis JSON-RPC endpoint                                            |
| `RUST_LOG`                   | `info`                    | Logging level; use `debug` or `warn` as needed                             |
| `RELAYER_API_RPC_SERVER_URL` | `http://0.0.0.0:8088/api` | Relayer JSON-RPC endpoint                                                  |
| `RELAYER_PROGRAM_JSON_PATH`  | `./relayerprogram.json`   | Path to the relayer program JSON file                                      |
| `VALIDATOR_WALLET_PATH`      | `validator.mnemonic`      | Default path for a validator mnemonic                                      |
| `CHAIN_ID`                   | `nyks`                    | Chain identifier                                                           |
| `DATABASE_URL_SQLITE`        | `./wallet_data.db`        | SQLite database path                                                       |
| `DATABASE_URL_POSTGRESQL`    | _none_                    | PostgreSQL connection URI; must be set when using the `postgresql` feature |

## Further Reading

- **README.md** – detailed architecture, use cases and API listings.
- **QuickStart.md** – step-by-step instructions for building, configuring and funding a test wallet.
- **DEPLOYMENT.md** – instructions for containerised deployment and advanced relayer setup.
- **twilight-client-sdk** – Rust primitives for QuisQuis and ZkOS.
