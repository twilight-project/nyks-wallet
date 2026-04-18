# Quick Start – Nyks Wallet SDK

This guide gets you from **zero to a funded test wallet in ~60 seconds**. For a deeper dive, see [`README.md`](README.md) and [`DEPLOYMENT.md`](DEPLOYMENT.md).

---

## 1 • Prerequisites

| Tool           | Min. version                            | Install                                                     | Additional Install             |
| -------------- | --------------------------------------- | ----------------------------------------------------------- | ------------------------------ |
| Rust toolchain | 2024-edition nightly (or stable ≥ 1.75) | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs` |                                |
| protoc         | ≥ 3.0                                   | Ubuntu: `sudo apt install protobuf-compiler`                | macOS: `brew install protobuf` |
| git            | any                                     | Ubuntu: `sudo apt install git`                              |                                |

> Tip – On Debian/Ubuntu add OpenSSL headers: `sudo apt install pkg-config libssl-dev`.

---

## 2 • Clone & build

```bash
# grab the source
$ git clone https://github.com/twilight-project/nyks-wallet.git
$ cd nyks-wallet

# compile everything in release mode (≈ 40 s on a modern laptop)
$ cargo build --release
```

This produces the library **and** two binaries in `target/release/`: `relayer-init` (bootstrap) and `relayer-cli` (interactive CLI).

---

## 3 • Configure endpoints

Endpoints come from environment variables; every variable has a built-in default chosen by `NETWORK_TYPE` (mainnet vs testnet), so nothing is required — override only when you want to point at a local full-node.

```bash
cat <<'EOF' > .env
# Select the network. Mainnet is the default if unset.
NETWORK_TYPE=testnet
# nyks chain only supports BTC mainnet — keep this as mainnet even on testnet.
BTC_NETWORK_TYPE=mainnet
CHAIN_ID=nyks

# The following testnet endpoints are the built-in defaults for NETWORK_TYPE=testnet.
# Only set them explicitly if you need to override.
# NYKS_LCD_BASE_URL=https://lcd.twilight.rest
# NYKS_RPC_BASE_URL=https://rpc.twilight.rest
# FAUCET_BASE_URL=https://faucet-rpc.twilight.rest
# ZKOS_SERVER_URL=https://nykschain.twilight.rest/zkos
# RELAYER_API_RPC_SERVER_URL=https://relayer.twilight.rest/api

RUST_LOG=info
EOF

# load them in your current shell
set -a; source .env; set +a
```

> See [`.env.example`](.env.example) for the full list and [README.md §7](README.md#7--environment-variables) for the mainnet/testnet defaults.

---

## 4 • Run the one-liner demo

```bash
# generates a fresh wallet, funds it on testnet, writes relayer_deployer.json
$ cargo run --bin relayer-init
```

On success you will see `relayer_deployer.json` in the working directory plus log output similar to:

```
Successfully wrote relayer data to relayer_deployer.json
```

Try the interactive CLI next:

```bash
$ cargo run --bin relayer-cli -- --help
```

---

## 5 • Use the SDK in your own project

Add a path (or Git) dependency:

```toml
# Cargo.toml
[dependencies]
nyks-wallet = { path = "../nyks-wallet" }           # or github = "twilight-project/nyks-wallet"
```

Minimal example:

```rust
use nyks_wallet::wallet::{Wallet, get_test_tokens};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1️⃣ create a random Cosmos+BTC wallet
    let mut wallet = Wallet::create_new_with_random_btc_address().await?;

    // 2️⃣ request test-net tokens (10 000 nyks & 50 000 sats)
    get_test_tokens(&mut wallet).await?;

    println!("Twilight address: {}", wallet.twilightaddress);
    println!("Balance: {} nyks, {} sats", wallet.balance_nyks, wallet.balance_sats);
    Ok(())
}
```

Compile & run:

```bash
cargo run -q
```

---

## 6 • Next steps

• Explore the full API surface in [`README.md`](README.md).
• Read the [OrderWallet guide](OrderWallet.md) for trading and lending.
• See the [`relayer-cli` reference](docs/relayer-cli.md) for the interactive CLI.
• Check the [Deployment guide](DEPLOYMENT.md) for Docker usage and advanced relayer setup.
• Join the discussion on Discord: https://discord.gg/twilight-protocol

Happy building! 🚀
