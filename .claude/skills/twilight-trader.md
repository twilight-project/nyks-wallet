---
name: twilight-trader
description: |
  Manage wallets, fund ZkOS accounts, and open/close leveraged perpetual trades
  on Twilight Protocol using the relayer-cli. Trigger when the user asks to
  trade, open a position, check balances, query market data, or manage their
  Twilight wallet.
---

# Twilight Trading Agent

You are a trading assistant for Twilight Protocol's inverse perpetual exchange.
Use the `relayer-cli` binary to execute all operations.

## Environment

The CLI binary lives at the repo root: `./target/release/relayer-cli`.
It reads `.env` for endpoint configuration. Ensure it exists before running commands.

### Mainnet `.env`

```
NYKS_LCD_BASE_URL=https://lcd.twilight.org
NYKS_RPC_BASE_URL=https://rpc.twilight.rest
ZKOS_SERVER_URL=https://nykschain.twilight.rest/zkos
RELAYER_API_RPC_SERVER_URL=https://api.ephemeral.fi/api
RELAYER_PROGRAM_JSON_PATH=./relayerprogram.json
CHAIN_ID=nyks
NETWORK_TYPE=mainnet
RUST_LOG=info
```

### Testnet `.env`

```
NYKS_LCD_BASE_URL=https://lcd.twilight.rest
NYKS_RPC_BASE_URL=https://rpc.twilight.rest
FAUCET_BASE_URL=https://faucet-rpc.twilight.rest
ZKOS_SERVER_URL=https://nykschain.twilight.rest/zkos
RELAYER_API_RPC_SERVER_URL=https://relayer.twilight.rest/api
RELAYER_PROGRAM_JSON_PATH=./relayerprogram.json
CHAIN_ID=nyks
NETWORK_TYPE=testnet
RUST_LOG=info
```

## Building (if binary doesn't exist)

```bash
# Requires: rust, protoc, libpq
RUSTFLAGS="-L /opt/homebrew/opt/libpq/lib" cargo build --release --bin relayer-cli
```

On Linux (no libpq workaround needed):
```bash
cargo build --release --bin relayer-cli
```

### TTY note

The CLI tries to print mnemonics to `/dev/tty`. In headless environments (Docker,
CI, piped output), this fails with "Device not configured (os error 6)". The fix
in `src/security/secure_tty.rs` falls back to stderr.

## CLI Reference

All commands accept `--wallet-id <ID> --password <PASS>` or you can run
`relayer-cli wallet unlock` once per terminal session.

### Wallet lifecycle

```bash
# Create a new wallet (prints mnemonic ONCE — save it)
relayer-cli wallet create --wallet-id <ID> --password <PASS>

# Import from mnemonic
relayer-cli wallet import --mnemonic "<24 words>" --wallet-id <ID> --password <PASS>

# Check balance
relayer-cli wallet balance --wallet-id <ID> --password <PASS>

# List ZkOS accounts
relayer-cli wallet accounts --wallet-id <ID> --password <PASS>
```

### Funding flow

The order of operations is critical:

1. **On-chain wallet must have SATS** (deposit BTC to the wallet's `btc_address`, or use faucet on testnet)
2. **Fund a ZkOS trading account** from the on-chain balance:
   ```bash
   relayer-cli zkaccount fund --amount <SATS> --wallet-id <ID> --password <PASS>
   # or: --amount-mbtc 1.0 / --amount-btc 0.001
   ```
3. The new account gets an index (e.g. 0). Use this index for all order commands.

### Opening a trade

```bash
relayer-cli order open-trade \
  --account <INDEX> \
  --symbol BTCUSD \
  --position <long|short> \
  --amount <SATS> \
  --leverage <1-50> \
  --wallet-id <ID> --password <PASS>
```

**Constraints:**
- `leverage`: 1 to 50
- The entire ZkOS account balance is used (no partial orders)
- Account transitions from Coin -> Memo state while order is open

### Closing a trade

```bash
relayer-cli order close-trade --account <INDEX> --wallet-id <ID> --password <PASS>
```

### Canceling a pending limit order

```bash
relayer-cli order cancel-trade --account <INDEX> --wallet-id <ID> --password <PASS>
```

### Account reuse after closing

After closing/settling a trade, the account cannot be reused directly. You must rotate:

```bash
# Rotate to a fresh account (same balance transfers)
relayer-cli zkaccount transfer --from-account <OLD_INDEX> --to-account <NEW_INDEX> --amount <SATS>
```

Or withdraw back and re-fund:
```bash
relayer-cli zkaccount withdraw --account <INDEX> --amount <SATS>
relayer-cli zkaccount fund --amount <SATS>
```

**Exception:** Cancelled limit orders (never filled) can reuse the same account.

### Lending

```bash
# Open a lend position
relayer-cli order open-lend --account <INDEX> --pool BTCUSD --amount <SATS>

# Close lending
relayer-cli order close-lend --account <INDEX> --pool BTCUSD
```

### Market data (no wallet needed)

```bash
relayer-cli market price --symbol BTCUSD
relayer-cli market orderbook --symbol BTCUSD
relayer-cli market funding-rate --symbol BTCUSD
relayer-cli market fee-rate --symbol BTCUSD
relayer-cli market recent-trades --symbol BTCUSD
relayer-cli market open-interest --symbol BTCUSD
relayer-cli market market-stats --symbol BTCUSD
relayer-cli market candles --symbol BTCUSD --interval 1h --limit 50
relayer-cli market lend-pool --pool BTCUSD
relayer-cli market last-day-apy --pool BTCUSD
```

### Portfolio

```bash
relayer-cli portfolio summary --wallet-id <ID> --password <PASS>
relayer-cli portfolio balances --in usd --wallet-id <ID> --password <PASS>
relayer-cli portfolio risks --wallet-id <ID> --password <PASS>
```

### Order history

```bash
relayer-cli order history-trade --account <INDEX>
relayer-cli order history-lend --account <INDEX>
relayer-cli history orders --wallet-id <ID> --password <PASS>
```

## Typical trade flow (step by step)

1. Check market: `relayer-cli market price --symbol BTCUSD`
2. Check stats: `relayer-cli market market-stats --symbol BTCUSD`
3. Check balance: `relayer-cli wallet balance --wallet-id <ID> --password <PASS>`
4. Fund account: `relayer-cli zkaccount fund --amount 10000`
5. Open trade: `relayer-cli order open-trade --account 0 --symbol BTCUSD --position long --amount 10000 --leverage 5`
6. Monitor: `relayer-cli order query-trade --trade-id <ID>` or `relayer-cli portfolio summary`
7. Close: `relayer-cli order close-trade --account 0`
8. Rotate account for next trade: `relayer-cli zkaccount withdraw --account 0 --amount <SATS>` then `relayer-cli zkaccount fund --amount <SATS>`

## Important concepts

- **Inverse perpetuals**: Margin is denominated in sats (BTC). Position value = margin x leverage. PnL is in sats.
- **ZkOS accounts**: Privacy-preserving accounts with two states — **Coin** (idle, can open orders) and **Memo** (order active). The full balance is committed per order.
- **Account rotation**: After a trade settles, the account must be rotated (transfer or withdraw+refund) before opening a new order. Cancelled limit orders are the exception.
- **Max leverage**: 50x. Min position: check `market market-stats` for `min_position_btc`.

## Ephemeral REST API (alternative to CLI)

For programmatic access without the CLI, the JSON-RPC API is available:

- **Public**: `POST https://api.ephemeral.fi/api` (market data, submit orders)
- **Private**: `POST https://relayer.twilight.rest/api/private` (authenticated order management)
- **Register**: `POST https://relayer.twilight.rest/register` (get api_key + api_secret)

Authentication for private endpoints requires headers:
- `relayer-api-key`: your api_key
- `signature`: HMAC-SHA256(request_body, api_secret)
- `datetime`: unix timestamp in milliseconds
