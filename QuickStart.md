# Quick Start – Nyks Wallet SDK

This guide gets you from **zero to a funded test wallet in ~60 seconds**. For a deeper dive, see `README.md` and `Deployment.md`.

---

## 1 • Prerequisites

| Tool           | Min. version                            | Install                                                     |
| -------------- | --------------------------------------- | ----------------------------------------------------------- | ------------------------------------ |
| Rust toolchain | 2024-edition nightly (or stable ≥ 1.75) | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs` |
| protoc         | ≥ 3.0                                   | Ubuntu `sudo apt install protobuf-compiler` &nbsp;          | &nbsp; macOS `brew install protobuf` |
| git            | any                                     | Ubuntu `sudo apt install git`                               |

> Tip – On Debian/Ubuntu add OpenSSL headers: `sudo apt install pkg-config libssl-dev`.

---

## 2 • Clone & build

```bash
# grab the source
$ git clone https://github.com/your-org/nyks-wallet.git
$ cd nyks-wallet

# compile everything in release mode (≈ 40 s on a modern laptop)
$ cargo build --release
```

This produces the library **and** the sample binary `relayer_init` in `target/release/`.

---

## 3 • Configure endpoints

Nyks Wallet talks to the public Twilight test-net. Endpoints are read from environment variables and will **panic if missing**.

```bash
cat <<'EOF' > .env
LCD_BASE_URL=https://lcd.twilight.rest
FAUCET_BASE_URL=https://faucet-rpc.twilight.rest
ZKOS_SERVER_URL=https://nykschain.twilight.rest/zkos
RUST_LOG=info
EOF

# load them in your current shell
source .env
```

---

## 4 • Run the one-liner demo

```bash
# generates a fresh wallet, funds it, deploys relayer contract
$ cargo run --bin relayer_init
```

On success you will see `relayer_deployer.json` in the working directory plus log output similar to:

```
Successfully wrote relayer data to relayer_deployer.json
```

---

## 5 • Use the SDK in your own project

Add a path (or Git) dependency:

```toml
# Cargo.toml
[dependencies]
nyks-wallet = { path = "../nyks-wallet" }           # or github = "twilight-project/nyks-wallet"
reqwest      = { version = "0.12", default-features = false, features = ["rustls-tls"] }
```

Minimal example:

```rust
use nyks_wallet::wallet::{Wallet, get_test_tokens};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1️⃣ create a random Cosmos+BTC wallet
    let mut wallet = Wallet::create_new_with_random_btc_address()?;

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
• Check the [Deployment guide](DEPLOYMENT.md) for Docker usage and advanced relayer setup.  
• Join the discussion on Discord: https://discord.gg/twilight-protocol

Happy building! 🚀
