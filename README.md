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
├── wallet/          # Account handling & on-chain helpers
│   ├── wallet.rs    # Wallet struct & lifecycle helpers
│   ├── faucet.rs    # Faucet + BTC-deposit tx builders
│   ├── nyks_fn.rs   # Mint/Burn trading messages
│   └── seed_signer.rs
│
├── zkos_accounts/   # Shielded account (QuisQuis) utilities
├── nyks_rpc/        # Minimal JSON-RPC client & message encoder
├── proto/           # Upstream `.proto` definitions (compiled by build.rs)
└── src/lib.rs       # Re-exports + generated protobuf modules
```

All network calls run on **Tokio + Reqwest**, all crypto is handled via **k256**, **curve25519-dalek** and **twilight-client-sdk**.

---

## 4 • Core functionality

### 4.1 Wallet lifecycle

- `Wallet::create_new_with_random_btc_address()` – generate a random Cosmos key-pair _and_ deterministic testnet BTC address.
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

| Module            | Item                                       | Purpose                                      |
| ----------------- | ------------------------------------------ | -------------------------------------------- |
| `wallet`          | `Wallet` struct                            | Holds keys, balances, sequence & BTC address |
|                   | `create_and_export_randmon_wallet_account` | Generates a fresh wallet & persists as JSON  |
|                   | `update_balance`                           | Fetch latest on-chain balances               |
| `wallet::faucet`  | `get_nyks` / `mint_sats`                   | Test-net token faucets                       |
|                   | `sign_and_send_reg_deposit_tx`             | Register BTC deposit address                 |
| `wallet::nyks_fn` | `create_funiding_to_trading_tx_msg`        | Build mint/burn message                      |
|                   | `send_tx`                                  | Sign & broadcast raw transaction             |
| `zkos_accounts`   | `ZkAccountDB`                              | HD database for shielded accounts            |
|                   | `EncryptedAccount`                         | Compact on-chain representation              |
| `nyks_rpc`        | `Method`, `MethodTypeURL`                  | JSON-RPC method enum & protobuf encoder      |
| `seed_signer`     | `generate_seed`                            | Deterministic Ristretto seed derivation      |

---

## 7 • Environment variables

| Variable            | Default                                | Description                                   |
| ------------------- | -------------------------------------- | --------------------------------------------- |
| `NYKS_LCD_BASE_URL` | `https://lcd.twilight.rest`            | Cosmos SDK LCD REST endpoint `port:1317`      |
| `NYKS_RPC_BASE_URL` | `https://rpc.twilight.rest`            | Cosmos SDK RPC REST endpoint `port:26657`     |
| `FAUCET_BASE_URL`   | `https://faucet-rpc.twilight.rest`     | Faucet & mint endpoints `port:6969`           |
| `ZKOS_SERVER_URL`   | `https://nykschain.twilight.rest/zkos` | zkaccount json-rpc endpoint `port:3030`       |
| `RUST_LOG`          | `info`                                 | `info`, `debug` and `warn` are available tags |

Set them before running to point the SDK at a local full-node.

---

## 8 • Getting started in your own project

```bash
# Cargo.toml
[dependencies]
nyks-wallet = { path = "../nyks-wallet" }    # or github = "twilight-project/nyks-wallet"
reqwest      = { version = "0.12", default-features = false, features = ["rustls-tls"] }
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
- [Deployment guide](DEPLOYMENT.md) – detailed steps to build & run `relayer_init` (plus Docker).
- [`twilight-client-sdk`](https://github.com/twilight-project/twilight-client-sdk) – Rust primitives for QuisQuis & ZkOS.
- [`relayer-core`](https://github.com/twilight-project/relayer-core) - Twilight Relayer Core is an extremely high performance matching engine written in Rust.
- [ADR-036](https://github.com/cosmos/cosmos-adrs/blob/main/adr-036-arbitrary-data-signature.md) – Canonical signing of arbitrary data (used by seed signer).

---

## 10 • License

Released under the **Apache License** – see [`LICENSE`](LICENSE) for details.
