# Nyks Wallet — Agent Guide

This is the Twilight Protocol wallet SDK and CLI for trading inverse perpetuals on the Ephemeral exchange.

## Quick start

```bash
# Build
RUSTFLAGS="-L /opt/homebrew/opt/libpq/lib" cargo build --release --bin relayer-cli

# Configure (copy and edit .env.example)
cp .env.example .env

# Create wallet
./target/release/relayer-cli wallet create --wallet-id my-wallet --password secret
```

## Skills

- **`/twilight-trader`** — Full trading workflow: wallet management, funding ZkOS accounts, opening/closing leveraged positions, market data queries, and portfolio tracking.

## Key files

- `src/bin/relayer_cli.rs` — CLI entrypoint
- `src/relayer_module/order_wallet.rs` — OrderWallet trading logic
- `src/wallet/wallet.rs` — Wallet creation and key management
- `src/security/secure_tty.rs` — TTY-safe secret printing
- `.env.example` — Environment variable template
- `relayerprogram.json` — ZkOS circuit parameters (required at runtime)
