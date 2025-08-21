# Nyks Wallet – Twilight Protocol Rust SDK for NYKS chain

## 📑 Index

1. [Project summary](#1--project-summary)
2. [Why & where it is used](#2--why--where-it-is-used)
3. [High-level architecture](#3--high-level-architecture)
4. [Core functionality](#4--core-functionality)
5. [Typical use-cases](#5--typical-use-cases)
6. [Most important functions & structs](#6--most-important-functions--structs)
7. [Environment variables](#7--environment-variables)
8. [Getting started in your own project](#8--getting-started-in-your-own-project)
9. [Further reading](#9--further-reading)
10. [License](#10--license)

## 1 • Project summary

Nyks Wallet is a **pure-Rust SDK** that wraps all the low-level gRPC/REST and cryptographic plumbing required to interact with the _Twilight (Nyks)_ blockchain. It lets application-developers, bots and relayers:

- create / import Cosmos-compatible accounts (secp256k1)
- request **Nyks** tokens and test **satoshis** from the public faucet
- register **Bitcoin deposit addresses** and mint / burn assets on-chain
- work with **ZkOS / QuisQuis** shielded accounts
- craft, sign and broadcast custom protobuf messages – all fully async

The crate started as a market-maker client and grew into the de-facto wallet layer used by dApps, backend services and integration tests across the Twilight ecosystem.

---

## 2 • Why & where it is used

1. **dApps & frontends** – to display balances, generate deposit addresses and submit user transactions.
2. **Off-chain services** – relayers and market-maker bots build transactions with `wallet` + `nyks_rpc` and push them to the chain.
3. **Integration tests** – CI pipelines spin-up a fresh test-net, then use Nyks Wallet helpers to seed accounts and perform end-to-end flows.
4. **CLI tools / scripts** – operations teams rely on the faucet & balance utilities for quick troubleshooting.

> If your code needs to talk to Twilight, Nyks Wallet is the easiest starting point – no Tendermint RPC decoding or protobuf boilerplate required.

---

## 3 • High-level architecture

```
nyks-wallet
│
└── src/
    ├── wallet/           # Account handling & on-chain helpers
    ├── relayer_module/   # OrderWallet trading & lending interface
    ├── zkos_accounts/    # Shielded account (QuisQuis) utilities
    ├── database/         # Optional Diesel persistence (feature gated)
    ├── security/         # Password, encryption & keyring helpers
    ├── nyks_rpc/         # Minimal JSON-RPC client & message encoder
    ├── config.rs         # Endpoint configuration helpers
    ├── bin/              # CLI utilities (e.g. relayer_init.rs)
    ├── proto/            # Upstream `.proto` definitions (compiled by build.rs)
    └── lib.rs            # Crate root & public re-exports
```

**Deep-links to key modules & docs**

- **wallet/** – [`src/wallet`](src/wallet) • see [Core functionality](#4--core-functionality)
- **relayer_module/** – [`src/relayer_module`](src/relayer_module) • see [OrderWallet guide](OrderWallet.md)
- **zkos_accounts/** – [`src/zkos_accounts`](src/zkos_accounts)
- **database/** – [`src/database`](src/database) • see [Database features overview](Database.md)
- **security/** – [`src/security`](src/security)
- **nyks_rpc/** – [`src/nyks_rpc`](src/nyks_rpc)
- **config.rs** – [`src/config.rs`](src/config.rs)
- **bin/** – [`src/bin`](src/bin) sample CLI utilities

All network calls run on **Tokio + Reqwest**, all crypto is handled via **k256**, **curve25519-dalek** and **twilight-client-sdk**.

---

## 4 • Core functionality

### 4.1 Wallet lifecycle

- `Wallet::new(None)` – generate a random Cosmos key-pair _and_ deterministic testnet BTC address.
- `Wallet::from_mnemonic(..)` / `Wallet::from_private_key(..)` – import existing credentials.
- `Wallet::import_from_json(..)` / `export_to_json(..)` – round-trip safe serialization for long-term storage.

### 4.2 Balance management

- `wallet::check_balance(addr)` – one-shot REST query.
- `Wallet::update_balance()` – refreshes the embedded `balance_nyks` & `balance_sats` fields.

### 4.3 Faucet helpers

- `faucet::get_nyks(addr)` – requests **10 000 nyks**.
- `faucet::mint_sats(addr)` – mints **50 000 test satoshis**.
- `faucet::mint_sats_5btc(addr)` – special 5 BTC mint used by relayer wallets.

### 4.4 BTC deposit registration & trading

- `faucet::sign_and_send_reg_deposit_tx(..)` – signs a `MsgRegisterBtcDepositAddress` and broadcasts it.
- `nyks_fn::create_funiding_to_trading_tx_msg(..)` – crafts a **mint/burn** trading message.
- `nyks_fn::send_tx(msg)` – generic helper that signs & synchronously broadcasts any protobuf `Any`.

### 4.5 ZkOS / QuisQuis accounts

- `ZkAccountDB::generate_new_account(balance, seed)` – derive a shielded child account from a _Cosmos_ signature.
- `EncryptedAccount` utilities – encode / decode, verify key-pairs, decrypt balances.

### 4.6 Seed signer (ADR-036)

- `seed_signer::generate_seed(..)` – produce a deterministic Ristretto seed from a canonical `MsgSignData`.

### 4.7 Low-level JSON-RPC client

- `nyks_rpc::rpcclient::Method` – exhaustive enum of Tendermint RPC calls.
- `MethodTypeURL::sign_msg(..)` – payload builder that handles protobuf packing, fee setup and secp256k1 signing.

### 4.8 OrderWallet trading & lending

- `OrderWallet::new(endpoint_cfg)` – instantiate high-level trading orchestrator (wraps `Wallet` + `ZkAccountDB`).
- `funding_to_trading(amount)` – create a fresh ZkOS account and fund it from the on-chain wallet.
- `open_trader_order(..)` / `close_trader_order(..)` / `cancel_trader_order(..)` – manage leveraged LONG/SHORT positions.
- `open_lend_order(..)` / `close_lend_order(..)` – lend liquidity and settle back to Coin state.
- `trading_to_trading(..)` & `trading_to_trading_multiple_accounts(..)` – move / split balances between ZkOS accounts.
- `with_db(passphrase, wallet_id)` – enable optional SQLite/PostgreSQL persistence for seeds, accounts, UTXOs & request IDs.

---

## 5 • Typical use-cases

| Scenario                               | Relevant APIs                                                         |
| -------------------------------------- | --------------------------------------------------------------------- |
| **Bootstrap local test-net**           | `create_and_export_randmon_wallet_account`, `get_nyks`, `mint_sats`   |
| **User deposit flow** (BTC → Twilight) | `create_register_btc_deposit_message`, `sign_and_send_reg_deposit_tx` |
| **Market-maker mint/burn loop**        | `create_funiding_to_trading_tx_msg`, `send_tx`                        |
| **Shielded asset transfers**           | `ZkAccountDB`, `EncryptedAccount` helpers                             |
| **Automated integration tests / CI**   | Any of the above – everything is headless & async                     |

---

## 6 • Most important functions & structs

| Module           | Item                                   | Purpose                                      |
| ---------------- | -------------------------------------- | -------------------------------------------- |
| `wallet`         | `Wallet` struct                        | Holds keys, balances, sequence & BTC address |
|                  | `Wallet::new` / `new(None)`            | Create fresh wallet (prints mnemonic once)   |
|                  | `update_balance`                       | Refresh on-chain nyks & sats balances        |
| `wallet::faucet` | `get_nyks` / `mint_sats`               | Test-net token faucets                       |
|                  | `sign_and_send_reg_deposit_tx`         | Register BTC deposit address                 |
| `relayer_module` | `OrderWallet` struct                   | High-level trading & lending orchestrator    |
|                  | `funding_to_trading`                   | Fund a new ZkOS account from wallet          |
|                  | `open_trader_order` / `close_*`        | Manage leveraged LONG/SHORT positions        |
|                  | `open_lend_order` / `close_lend_order` | Lend liquidity & settle back                 |
| `zkos_accounts`  | `ZkAccountDB`                          | In-memory DB for shielded accounts           |
| `nyks_rpc`       | `Method`, `MethodTypeURL`              | Tendermint RPC wrappers & protobuf encoder   |
| `seed_signer`    | `generate_seed`                        | Deterministic Ristretto seed derivation      |

---

## 7 • Environment variables

| Variable                     | Default                   | Description                                           |
| ---------------------------- | ------------------------- | ----------------------------------------------------- |
| `NYKS_LCD_BASE_URL`          | `http://0.0.0.0:1317`     | Cosmos SDK LCD REST endpoint                          |
| `NYKS_RPC_BASE_URL`          | `http://0.0.0.0:26657`    | Tendermint RPC endpoint                               |
| `FAUCET_BASE_URL`            | `http://0.0.0.0:6969`     | Faucet & mint endpoints                               |
| `ZKOS_SERVER_URL`            | `http://0.0.0.0:3030`     | ZkOS / QuisQuis JSON-RPC server                       |
| `RELAYER_API_RPC_SERVER_URL` | `http://0.0.0.0:8088/api` | Relayer public JSON-RPC API (OrderWallet)             |
| `PUBLIC_API_RPC_SERVER_URL`  | `http://0.0.0.0:8088/api` | Public price-feed / order-book API                    |
| `RELAYER_RPC_SERVER_URL`     | `http://0.0.0.0:3032`     | Internal relayer client RPC                           |
| `RELAYER_PROGRAM_JSON_PATH`  | `./relayerprogram.json`   | Path to relayer program ABI/bytecode                  |
| `CHAIN_ID`                   | `nyks`                    | Chain identifier used in signed msgs                  |
| `RUST_LOG`                   | `info`                    | Log level (`info`, `debug`, `trace`, …)               |
| `RUST_BACKTRACE`             | `full`                    | Enable Rust backtraces for debugging                  |
| `NYKS_WALLET_PASSPHRASE`     | –                         | Passphrase used to encrypt wallet seed                |
| `WALLET_ID`                  | –                         | Override default wallet ID when using DB              |
| `VALIDATOR_WALLET_PATH`      | `validator-self.mnemonic` | Path to validator mnemonic (validator-wallet)         |
| `DATABASE_URL_SQLITE`        | `./wallet_data.db`        | SQLite file used when feature = `sqlite`              |
| `DATABASE_URL_POSTGRESQL`    | –                         | PostgreSQL connection string (feature = `postgresql`) |

Set them before running to point the SDK at a local full-node.

---

## 8 • Getting started in your own project

```bash
# Cargo.toml
[dependencies]
nyks-wallet = { path = "../nyks-wallet" }    # or github = "twilight-project/nyks-wallet"
```

```rust
use nyks_wallet::wallet::{Wallet, get_test_tokens};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Import an existing key (mnemonic, hex or JSON)
    let mut wallet = Wallet::from_mnemonic("your twelve words …")?;

    // Fund it on test-net
    get_test_tokens(&mut wallet).await?;

    println!("Twilight address: {}", wallet.twilightaddress);
    println!("Balance: {} nyks, {} sats", wallet.balance_nyks, wallet.balance_sats);
    Ok(())
}
```

---

## 9 • Further reading

- [Quick Start guide](QuickStart.md) – fastest path to build, configure & fund a test wallet.
- [OrderWallet guide](OrderWallet.md) – comprehensive reference for trading & lending APIs.
- [Order flow walk-through](OrderFlow.md) – step-by-step lifecycle of a leveraged position.
- [Validator wallet guide](Validator-Wallet.md) – validator node key management utilities.
- [Database features overview](Database.md) – optional SQLite/PostgreSQL persistence design.
- [Trading Bot docs](examples/trading_bot/docs) – reference end-to-end automated bot implementation.
- [Deployment guide](DEPLOYMENT.md) – build & run `relayer_init` (plus Docker containers).
- [`twilight-client-sdk`](https://github.com/twilight-project/twilight-client-sdk) – Rust primitives for QuisQuis & ZkOS.
- [`relayer-core`](https://github.com/twilight-project/relayer-core) – ultra-low-latency matching engine used by Twilight.

---

## 10 • License

Released under the **Apache License** – see [`LICENSE`](LICENSE) for details.
