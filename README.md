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

- `Wallet::new(chain_config: Option<WalletEndPointConfig>)` – generate a random Cosmos key-pair along with a BIP-39 BTC wallet; prints the 24-word mnemonic once to the TTY.
- `Wallet::create_new_with_random_btc_address()` – async variant that does not print the mnemonic (used in automated flows).
- `Wallet::from_mnemonic(mnemonic, chain_config)` – import an existing 24-word mnemonic.
- `Wallet::from_private_key(private_key, btc_address, chain_config)` – import using a raw secp256k1 hex private key (no BTC wallet, just an address).
- `Wallet::from_mnemonic_file(path)` – read mnemonic from a file (used by the validator wallet).
- `Wallet::import_from_json(path)` / `Wallet::export_to_json(path)` – round-trip safe serialization for long-term storage.

> The BTC network (`mainnet` vs `testnet`) used to derive the BIP-86 Taproot address is controlled by `BTC_NETWORK_TYPE` — default `mainnet`. The nyks chain only supports BTC mainnet, so keep `BTC_NETWORK_TYPE=mainnet` even on nyks testnet.

### 4.2 Balance & account info

- `wallet::check_balance(addr, lcd_endpoint)` – one-shot REST query against the LCD endpoint.
- `Wallet::update_balance()` – refreshes the embedded `balance_nyks` & `balance_sats` fields.
- `Wallet::account_info()` / `Wallet::update_account_info()` – fetches the Cosmos auth account (sequence + account number).

### 4.3 Faucet helpers (testnet only)

- `faucet::get_nyks(addr, faucet_endpoint)` – requests **10 000 nyks**.
- `faucet::mint_sats(addr, faucet_endpoint)` – mints **50 000 test satoshis**.
- `faucet::mint_sats_5btc(addr, faucet_endpoint)` – special 5 BTC mint used by relayer wallets.
- `wallet::get_test_tokens(&mut wallet)` – one-shot helper that requests nyks, registers the BTC address, and mints sats. Errors on mainnet (`NETWORK_TYPE=mainnet`).

### 4.4 BTC deposit & withdrawal

- `Wallet::register_btc_deposit(..)` – signs and broadcasts `MsgRegisterBtcDepositAddress`.
- `Wallet::withdraw_btc(..)` – signs and broadcasts `MsgWithdrawBtcRequest`.
- `Wallet::fetch_deposit_status()` / `fetch_deposit_details()` – query current deposit state from the indexer.
- `Wallet::fetch_withdrawal_status(..)` – query withdrawal progress by ID.
- `Wallet::fetch_btc_reserves()` / `fetch_btc_proposed_reserve()` – read live BTC reserve state.
- `Wallet::fetch_registered_btc_by_address(..)` – verify whether a given address is registered on-chain.
- `Wallet::fetch_account_from_indexer()` – full indexer view of the account (deposits, withdrawals, balances).
- `Wallet::send_tokens(to_address, amount, denom)` – send `nyks` or `sats` to another Twilight address.
- `faucet::sign_and_send_reg_deposit_tx(..)` – lower-level primitive that signs a `MsgRegisterBtcDepositAddress`.
- `nyks_fn::create_funiding_to_trading_tx_msg(..)` – crafts a mint/burn trading message (note: the typo `funiding` is intentional in the current API surface).

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

Endpoint defaults are selected by `NETWORK_TYPE` (and `BTC_NETWORK_TYPE` for BTC-specific ones). Override any variable explicitly to point at a local full-node.

| Variable                     | Default (mainnet)                       | Default (testnet)                      | Description                                      |
| ---------------------------- | --------------------------------------- | -------------------------------------- | ------------------------------------------------ |
| `NETWORK_TYPE`               | `mainnet`                               | `mainnet`                              | Selects endpoint defaults (`mainnet` / `testnet`) |
| `BTC_NETWORK_TYPE`           | `mainnet`                               | `mainnet`                              | BTC network for Esplora endpoints (nyks only supports BTC mainnet) |
| `CHAIN_ID`                   | `nyks`                                  | `nyks`                                 | Chain identifier used in signed msgs             |
| `NYKS_LCD_BASE_URL`          | `https://lcd.twilight.org`              | `https://lcd.twilight.rest`            | Cosmos SDK LCD REST endpoint                     |
| `NYKS_RPC_BASE_URL`          | `https://rpc.twilight.org`              | `https://rpc.twilight.rest`            | Tendermint RPC endpoint                          |
| `FAUCET_BASE_URL`            | *(empty)*                               | `https://faucet-rpc.twilight.rest`     | Faucet & mint endpoints (testnet only)           |
| `ZKOS_SERVER_URL`            | `https://zkserver.twilight.org`         | `https://nykschain.twilight.rest/zkos` | ZkOS / QuisQuis JSON-RPC server                  |
| `RELAYER_API_RPC_SERVER_URL` | `https://api.ephemeral.fi/api`          | `https://relayer.twilight.rest/api`    | Relayer public JSON-RPC API (OrderWallet)        |
| `TWILIGHT_INDEXER_URL`       | `https://indexer.twilight.org`          | `https://indexer.twilight.rest`        | Twilight indexer endpoint                        |
| `BTC_ESPLORA_PRIMARY_URL`    | `https://blockstream.info/api`          | `https://blockstream.info/testnet/api` | Primary Esplora API (driven by `BTC_NETWORK_TYPE`) |
| `BTC_ESPLORA_FALLBACK_URL`   | `https://mempool.space/api`             | `https://mempool.space/testnet/api`    | Fallback Esplora API (driven by `BTC_NETWORK_TYPE`) |
| `RELAYER_PROGRAM_JSON_PATH`  | `./relayerprogram.json`                 | `./relayerprogram.json`                | Path to relayer program ABI/bytecode             |
| `VALIDATOR_WALLET_PATH`      | `validator.mnemonic`                    | `validator.mnemonic`                   | Path to validator mnemonic (validator-wallet feature); `.env.example` overrides to `validator-self.mnemonic` |
| `RUST_LOG`                   | –                                       | –                                      | Log level (`info`, `debug`, `trace`, …)          |
| `RUST_BACKTRACE`             | –                                       | –                                      | Enable Rust backtraces for debugging             |
| `NYKS_WALLET_PASSPHRASE`     | –                                       | –                                      | Passphrase used to encrypt wallet seed           |
| `WALLET_ID`                  | –                                       | –                                      | Override default wallet ID when using DB         |
| `DATABASE_URL_SQLITE`        | `./wallet_data.db`                      | `./wallet_data.db`                     | SQLite file used when feature = `sqlite`         |
| `DATABASE_URL_POSTGRESQL`    | –                                       | –                                      | PostgreSQL connection string (feature = `postgresql`) |

---

## 8 • Getting started in your own project

```toml
# Cargo.toml
[dependencies]
nyks-wallet = { git = "https://github.com/twilight-project/nyks-wallet", tag = "v0.1.2" }
# or: nyks-wallet = { path = "../nyks-wallet" }
```

Default features enable `sqlite` + `order-wallet`. Disable defaults and pick your own set if you don't need the DB:

```toml
nyks-wallet = { git = "...", default-features = false, features = ["order-wallet"] }
```

```rust
use nyks_wallet::wallet::{Wallet, get_test_tokens};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Import an existing 24-word mnemonic (chain_config=None → defaults from NETWORK_TYPE)
    let mut wallet = Wallet::from_mnemonic("your twenty-four words …", None)?;

    // Fund it on test-net (requires NETWORK_TYPE=testnet; errors on mainnet)
    get_test_tokens(&mut wallet).await?;

    println!("Twilight address: {}", wallet.twilightaddress);
    println!("Balance: {} nyks, {} sats", wallet.balance_nyks, wallet.balance_sats);
    Ok(())
}
```

### 8.1 Binaries shipped with the crate

The crate also builds two CLIs (both require the `order-wallet` feature, which is on by default):

- **`relayer-init`** – one-shot bootstrap that creates a wallet, funds it on testnet, and writes `relayer_deployer.json`. See [DEPLOYMENT.md](DEPLOYMENT.md).
- **`relayer-cli`** – interactive REPL + subcommand CLI for wallets, ZkOS accounts, orders, market data, portfolio, and self-update. See [`docs/relayer-cli.md`](docs/relayer-cli.md).

```bash
cargo build --release           # builds lib + both binaries with default features
cargo run --bin relayer-init    # run the bootstrap
cargo run --bin relayer-cli -- --help
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
