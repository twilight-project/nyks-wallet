# Relayer-Init Deployment Guide

This document explains how to build and run the `relayer-init` binary that ships with **nyks-wallet**.

---

## 1. Prerequisites

Make sure the following tools are available on your system:

| Tool                        | Minimum version      | Install command                                                               |
| --------------------------- | -------------------- | ----------------------------------------------------------------------------- |
| Rust toolchain              | 2024-edition nightly | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs`                   |
| `protoc` (Protocol Buffers) | ≥ 3.0                | Ubuntu `sudo apt install protobuf-compiler`<br/>macOS `brew install protobuf` |
| `git`                       | any                  | Ubuntu `sudo apt install git`                                                 |

> **Tip** – On Debian/Ubuntu you might also need the OpenSSL headers: `sudo apt install pkg-config libssl-dev`.

---

## 2. Clone & build

```bash
# clone the repository
$ git clone https://github.com/your-org/nyks-wallet.git
$ cd nyks-wallet

# (optional) update package lists and install protoc
$ sudo apt-get update
$ sudo apt-get install protobuf-compiler

# compile only relayer-init in release mode
$ cargo build --release --bin relayer-init
```

The resulting binary will be at `target/release/relayer-init`.

---

## 3. Configure endpoints

`relayer-init` talks to several Twilight test-net services. Endpoints are looked up via environment variables and **panic if they are not set**.

| Variable            | Default value                                                        | Notes                          |
| ------------------- | -------------------------------------------------------------------- | ------------------------------ |
| `NYKS_LCD_BASE_URL` | `http://0.0.0.0:1317` (public: https://lcd.twilight.rest)            | NYKS chain LCD (REST) endpoint |
| `NYKS_RPC_BASE_URL` | `http://0.0.0.0:26657` (public: https://rpc.twilight.rest)           | NYKS chain RPC endpoint        |
| `FAUCET_BASE_URL`   | `http://0.0.0.0:6969` (public: https://faucet-rpc.twilight.rest)     | Nyks / BTC faucet services     |
| `ZKOS_SERVER_URL`   | `http://0.0.0.0:3030` (public: https://nykschain.twilight.rest/zkos) | ZkOS JSON-RPC endpoint         |
| `RUST_LOG`          | `info`                                                               | Logging info                   |

### 3.1 Create a `.env` file (recommended)

```bash
cat <<'EOF' > .env
NYKS_LCD_BASE_URL=http://0.0.0.0:1317
NYKS_RPC_BASE_URL=http://0.0.0.0:26657
FAUCET_BASE_URL=http://0.0.0.0:6969
ZKOS_SERVER_URL=http://0.0.0.0:3030
RUST_LOG=info

EOF
```

Load it in your shell:

```bash
source .env
```

### 3.2 One-off override

You can also pass variables inline for a single execution:

```bash
ZKOS_SERVER_URL=https://nykschain.twilight.rest/zkos \
NYKS_LCD_BASE_URL=https://lcd.twilight.rest \
NYKS_RPC_BASE_URL=https://rpc.twilight.rest \
FAUCET_BASE_URL=https://faucet-rpc.twilight.rest \
RUST_LOG=info \
cargo run --bin relayer-init
```

---

## 4. Run `relayer-init`

With the environment configured you can start the program either via **cargo** (dev-friendly) or the compiled binary (faster start-up):

### 4.1 Cargo

```bash
cargo run --bin relayer-init  # uses the current directory’s source
```

### 4.2 Pre-built binary

```bash
./target/release/relayer-init
```

Both modes will:

1. Generate a fresh wallet (random mnemonic & BTC address).
2. Request test-net tokens from the faucet.
3. Deploy the initial relayer state contract.
4. Write details to `relayer_deployer.json`.

Logs are printed to stdout – watch for **“Successfully wrote relayer data to relayer_deployer.json”** to confirm success.

---

## 5. Troubleshooting

• **`missing environment variable …`** – double-check you exported all vars in step 3.  
• **`Failed to get utxo details`** – the ZkOS RPC may be syncing; retry in a few minutes.  
• **TLS errors** – if you are on Linux without CA certificates, install them: `sudo apt install ca-certificates`.

---

## 6. Cleanup / rebuild

```bash
# clean the workspace (optional)
cargo clean

# rebuild after changing code
autoenv | source .env && cargo run --bin relayer-init
```

---

Happy relayer-initialising! 🎉

## 7. Docker container (optional)

Running `relayer-init` inside Docker guarantees a reproducible build and keeps the generated files in your current directory.

### 7.1 Build the image

```bash
# From the repository root (where Dockerfile lives)
docker build -t relayer-init .
```

- `-t relayer-init` tags the image, so you can reference it by name later.
- The multi-stage Dockerfile compiles the binary with Rust **1.87** and copies it into a minimal Debian runtime layer.

### 7.2 Run the container

```bash
# current directory will receive relayer_deployer.json
docker run --rm \
  -e NYKS_LCD_BASE_URL=https://lcd.twilight.rest \
  -e NYKS_RPC_BASE_URL=https://rpc.twilight.rest \
  -e FAUCET_BASE_URL=https://faucet-rpc.twilight.rest \
  -e ZKOS_SERVER_URL=https://nykschain.twilight.rest/zkos \
  -e RUST_LOG=info \
  -v $(pwd):/home/relayer \
  relayer-init
```

Explanation of the flags:

| Flag                      | Purpose                                                                                       |
| ------------------------- | --------------------------------------------------------------------------------------------- |
| `--rm`                    | Remove the container after it exits (keeps things tidy).                                      |
| `-e VAR=value`            | Pass required environment variables. Add more `-e …` flags if your setup needs them.          |
| `-v $(pwd):/home/relayer` | Mount your **present working directory** into the container’s workdir so the JSON is exported |
| Image name `relayer-init` | Matches the tag set during build.                                                             |

When the program finishes you should see `relayer_deployer.json` appear in the directory where you ran the command.

> **Note:** If you change the Dockerfile or code, rebuild the image with `docker build -t relayer-init .` before running again.
