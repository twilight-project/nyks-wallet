# Nyks Wallet – Rust SDK for the Twilight Protocol

Nyks Wallet is a **pure-Rust library** that makes it simple to create and manage Twilight Protocol (Nyks) accounts, request test-tokens, register Bitcoin deposit addresses and inspect on-chain balances – all from comfortable, idiomatic Rust code.

The crate was originally started as an experimental “market-maker client”. Since then the focus has shifted to a **general-purpose wallet SDK**, so the README has been updated to match the current API and project layout.

---

## ✨ Highlights

- **Wallet lifecycle** – generate random secp256k1 keys or import from a BIP-39 mnemonic / raw 32-byte hex key.
- **Balance queries** – fetch `nyks` & `sats` balances over the Twilight LCD (REST) endpoint.
- **Faucet helpers** – one-liners to mint Twilight tokens and test satoshis on the public test-net.
- **BTC deposit registration** – sign & broadcast a `MsgRegisterBtcDepositAddress` transaction.
- **Protocol Buffers included** – the bridge module messages compile automatically through `build.rs`.
- **Fully async** – built on top of Tokio and Reqwest.

---

## 🚀 Quick start

### 1 — Install prerequisites

| Dependency              | Notes                                                                                                                    |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| **Rust (2024 edition)** | `rustup default nightly` is recommended until the 2024 edition is stabilised.                                            |
| **protoc**              | Required for compiling `.proto` files.<br>Ubuntu: `sudo apt install protobuf-compiler`<br>macOS: `brew install protobuf` |

### 2 — Clone & build

```bash
$ git clone https://github.com/your-org/nyks-wallet.git
$ cd nyks-wallet
$ sudo apt-get update
$ sudo apt-get install protobuf-compiler
$ cargo build
```

### 3 — Run the demo

The repo does not contain a binary target out-of-the-box. Create one (or just paste the snippet below into `examples/demo.rs`) to see everything working end-to-end:

```rust
use nyks_wallet::wallet::{
    create_and_export_randmon_wallet_account, get_test_tokens, Wallet,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Generate a new wallet & export it as <name>.json
    create_and_export_randmon_wallet_account("alice").await?;

    // 2. Re-import the wallet from JSON
    let mut wallet = Wallet::import_from_json("alice.json")?;

    // 3. Request Nyks & test satoshis from the faucet
    get_test_tokens(&mut wallet).await?;

    // 4. Refresh on-chain balances
    wallet.update_balance().await?;
    println!("💰 Current balance: {} nyks, {} sats", wallet.balance_nyks, wallet.balance_sats);

    Ok(())
}
```

Run it:

```bash
cargo run --example demo --features reqwest/rustls-tls
```

---

## 🛠️ Library overview

| Function / Type                                          | Purpose                                                                                                       |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `wallet::create_and_export_randmon_wallet_account(name)` | Generates a 24-word mnemonic, derives the first Cosmos address and writes `<name>.json` with all the details. |
| `wallet::Wallet::from_mnemonic(…) / from_private_key(…)` | Import an existing account.                                                                                   |
| `wallet::Wallet::import_from_json(path)`                 | Load the JSON written by the helper above.                                                                    |
| `wallet::Wallet::update_balance()`                       | Refreshes `balance_nyks` & `balance_sats` fields in-place and returns a `Balance` struct.                     |
| `faucet::get_nyks(addr)`                                 | Request 10 000 `nyks` on test-net.                                                                            |
| `faucet::mint_sats(addr)`                                | Mint 50 000 test satoshis on test-net.                                                                        |
| `faucet::sign_and_send_reg_deposit_tx(…)`                | Register a BTC deposit address on-chain.                                                                      |

All network calls are asynchronous – remember to `await` them inside a Tokio runtime.

---

## 🌐 Configuration

The endpoints default to Twilight’s public test-net. You can point the SDK at a local full-node by overriding the following environment variables:

| Variable          | Default                            | Description                  |
| ----------------- | ---------------------------------- | ---------------------------- |
| `LCD_BASE_URL`    | `https://lcd.twilight.rest`        | Cosmos SDK LCD REST endpoint |
| `FAUCET_BASE_URL` | `https://faucet-rpc.twilight.rest` | Faucet & mint endpoints      |

Example:

```bash
export LCD_BASE_URL=http://localhost:1317
export FAUCET_BASE_URL=http://localhost:8080
```

---

## 🧪 Running tests

```bash
cargo test -- --nocapture
```

The test-suite uses [`serial_test`](https://docs.rs/serial_test) to ensure faucet interactions run one-by-one.

---

## 📂 Project layout

```
.
├── src/
│   ├── lib.rs           # Re-exports & protobuf include!
│   ├── wallet.rs        # Wallet API implementation
│   ├── faucet.rs        # Faucet helpers & tx builders
│   └── test.rs          # Integration tests
├── proto/               # Upstream `.proto` files
│   └── nyks/module/v1/tx.proto
├── build.rs             # Compiles the protobuf at build-time
├── Cargo.toml
└── README.md            # You are here
```

---

## 🤝 Contributing

Pull requests, issues and feature requests are very welcome! Please open an issue first to discuss what you would like to change.

1. Fork the repo & create a new branch.
2. Make your changes (don’t forget `cargo fmt`).
3. Add tests where applicable.
4. Open a PR – GitHub Actions will run `cargo test`.

---

## 📜 License

Nyks Wallet is released under the MIT License. See the [LICENSE](LICENSE) file for full text.
