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
Use the `relayer-cli` binary to execute all operations. Refer to `docs/relayer-cli.md`
for the full CLI reference.

## Environment

The CLI binary lives at the repo root: `./target/release/relayer-cli`.
It reads `.env` for endpoint configuration. Ensure it exists before running commands.

### Mainnet `.env`

```
NYKS_LCD_BASE_URL=https://lcd.twilight.org
NYKS_RPC_BASE_URL=https://rpc.twilight.org
ZKOS_SERVER_URL=https://zkserver.twilight.org
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
# macOS (libpq from homebrew)
RUSTFLAGS="-L /opt/homebrew/opt/libpq/lib" cargo build --release --bin relayer-cli

# Linux
cargo build --release --bin relayer-cli

# PostgreSQL backend instead of SQLite
cargo build --release --bin relayer-cli --no-default-features --features postgresql
```

### TTY note

The CLI prints mnemonics to `/dev/tty`. In headless environments (Docker, CI),
this falls back to stderr after the fix in `src/security/secure_tty.rs`.

## Password & Wallet ID Resolution

Most commands accept `--wallet-id` and `--password`. When omitted:

- **Wallet ID**: `--wallet-id` → session cache (`wallet unlock`) → `NYKS_WALLET_ID` env → error
- **Password**: `--password` → session cache → `NYKS_WALLET_PASSPHRASE` env → none

Use `relayer-cli wallet unlock` to cache credentials for a terminal session.

## Wallet Commands

```bash
# Create a new wallet (prints mnemonic ONCE — save it)
relayer-cli wallet create --wallet-id <ID> --password <PASS>
relayer-cli wallet create --btc-address bc1q...  # use existing BTC address

# Import from mnemonic (prompts securely if --mnemonic omitted)
relayer-cli wallet import --mnemonic "<24 words>" --wallet-id <ID> --password <PASS>

# Check balance
relayer-cli wallet balance --wallet-id <ID> --password <PASS>

# List ZkOS accounts (--on-chain-only to hide off-chain)
relayer-cli wallet accounts --wallet-id <ID> --password <PASS>
relayer-cli wallet accounts --on-chain-only

# Wallet info (no chain calls)
relayer-cli wallet info --wallet-id <ID> --password <PASS>

# List all stored wallets
relayer-cli wallet list

# Lock/unlock session
relayer-cli wallet unlock          # cache wallet-id + password for session
relayer-cli wallet lock            # clear session cache

# Backup & restore
relayer-cli wallet backup --wallet-id <ID> --password <PASS> --output backup.json
relayer-cli wallet restore --wallet-id <ID> --password <PASS> --input backup.json

# Export wallet to JSON
relayer-cli wallet export --wallet-id <ID> --password <PASS> --output wallet.json

# Update BTC deposit address
relayer-cli wallet update-btc-address --btc-address bc1q... --wallet-id <ID>

# Sync nonce from chain
relayer-cli wallet sync-nonce --wallet-id <ID> --password <PASS>

# Change password (always prompts via TTY)
relayer-cli wallet change-password --wallet-id <ID>
```

## ZkOS Account Commands

Fund, withdraw, transfer, and split ZkOS trading accounts. All amounts accept
one of `--amount` (sats), `--amount-mbtc`, or `--amount-btc`.

```bash
# Fund a new ZkOS trading account from on-chain sats
relayer-cli zkaccount fund --amount 10000
relayer-cli zkaccount fund --amount-mbtc 1.0
relayer-cli zkaccount fund --amount-btc 0.001

# Withdraw back to on-chain wallet
relayer-cli zkaccount withdraw --account <INDEX> --amount 5000

# Transfer (rotate) to a fresh account
relayer-cli zkaccount transfer --from <INDEX>

# Split one account into multiple
relayer-cli zkaccount split --from <INDEX> --balances "2000,3000,5000"
relayer-cli zkaccount split --from <INDEX> --balances-mbtc "0.02,0.03"
```

## Order Commands

### Open a trade

```bash
relayer-cli order open-trade \
  --account-index <INDEX> \
  --side <long|short> \
  --entry-price <USD_INT> \
  --leverage <1-50> \
  --order-type MARKET \
  --no-wait              # optional: return immediately after relayer accepts

# Examples:
relayer-cli order open-trade --account-index 0 --side long --entry-price 66700 --leverage 5
relayer-cli order open-trade --account-index 0 --side short --entry-price 66700 --leverage 10 --no-wait
```

**Constraints:**
- `leverage`: 1 to 50
- The entire ZkOS account balance is used as margin (no partial orders)
- Account transitions Coin → Memo while order is open
- Check `market market-stats` for max position size (20% of pool equity)
- If position exceeds cap, split the account first with `zkaccount split`

### Close a trade

```bash
relayer-cli order close-trade --account-index <INDEX>
relayer-cli order close-trade --account-index <INDEX> --no-wait  # skip chain sync

# With stop-loss / take-profit
relayer-cli order close-trade --account-index <INDEX> --stop-loss 60000 --take-profit 70000
```

### Cancel a pending order

```bash
relayer-cli order cancel-trade --account-index <INDEX>
```

### Query orders

```bash
relayer-cli order query-trade --trade-id <TRADE_ID>
relayer-cli order query-lend --lend-id <LEND_ID>
relayer-cli order history-trade --account <INDEX>
relayer-cli order history-lend --account <INDEX>
relayer-cli order funding-history --symbol BTCUSD
relayer-cli order account-summary --account <INDEX>
relayer-cli order tx-hashes --account <INDEX>
```

### Lending

```bash
relayer-cli order open-lend --account <INDEX> --pool BTCUSD --amount 10000
relayer-cli order close-lend --account <INDEX> --pool BTCUSD
```

## Account Reuse After Closing

After closing/settling a trade, the account must be **rotated** before opening
a new order. Twilight enforces account freshness.

```bash
# Option A: Rotate to fresh account (recommended)
relayer-cli zkaccount transfer --from <OLD_INDEX>
# → Creates new account at next index with same balance

# Option B: Withdraw + re-fund
relayer-cli zkaccount withdraw --account <INDEX> --amount <SATS>
relayer-cli zkaccount fund --amount <SATS>
```

**Exception**: Cancelled limit orders (never filled) can reuse the same account.

## Market Data (no wallet needed)

```bash
relayer-cli market price
relayer-cli market orderbook
relayer-cli market funding-rate
relayer-cli market fee-rate
relayer-cli market recent-trades
relayer-cli market open-interest
relayer-cli market market-stats
relayer-cli market candles --interval 1h --limit 50
relayer-cli market server-time

# Lending pool
relayer-cli market lend-pool --pool BTCUSD
relayer-cli market pool-share-value --pool BTCUSD
relayer-cli market last-day-apy --pool BTCUSD
relayer-cli market apy-chart --pool BTCUSD --days 30

# Historical
relayer-cli market history-price --symbol BTCUSD --days 30
relayer-cli market history-funding --symbol BTCUSD
relayer-cli market history-fees --symbol BTCUSD
```

## Portfolio

```bash
relayer-cli portfolio summary
relayer-cli portfolio balances --in usd
relayer-cli portfolio balances --in btc
relayer-cli portfolio risks
```

## History (requires DB)

```bash
relayer-cli history orders --limit 20
relayer-cli history transfers --limit 10
```

## Typical Trade Flow (step by step)

```bash
# 1. Check market
relayer-cli market price
relayer-cli market market-stats

# 2. Check balance
relayer-cli wallet balance

# 3. Fund a ZkOS account
relayer-cli zkaccount fund --amount 5000

# 4. Open trade (check max position first from market-stats)
relayer-cli order open-trade --account-index 0 --side long --entry-price 66700 --leverage 2

# 5. Monitor
relayer-cli portfolio summary

# 6. Close
relayer-cli order close-trade --account-index 0

# 7. Rotate for next trade
relayer-cli zkaccount transfer --from 0
# Now use next index for the new trade
```

### Fast trade (--no-wait)

For bots or when you don't need chain confirmation:

```bash
# Open — returns in ~2.8s instead of ~5s
relayer-cli order open-trade --account-index 0 --side long --entry-price 66700 --leverage 5 --no-wait

# Close — returns PnL in ~5s, defers chain sync
relayer-cli order close-trade --account-index 0 --no-wait
```

## Important Concepts

- **Inverse perpetuals**: Margin is in sats (BTC). Position value = margin × leverage. PnL in sats.
- **ZkOS accounts**: Privacy-preserving with two states — **Coin** (idle) and **Memo** (order active). Full balance committed per order.
- **Account rotation**: Must rotate after settle. Use `zkaccount transfer --from <INDEX>`.
- **Max leverage**: 50x. Max position: 20% of pool equity (check `market market-stats`).
- **Fees**: 4% filled on market, 2% filled on limit, 4% settled on market, 2% settled on limit.
- **--no-wait**: Returns after relayer confirms, skips chain UTXO sync. Account syncs lazily on next use.
- **--json**: All commands support `--json` for scripting/bot integration.

## Ephemeral REST API (alternative to CLI)

For programmatic access without the CLI, the JSON-RPC API is available:

- **Public**: `POST https://api.ephemeral.fi/api` (market data, submit orders)
- **Private**: `POST https://relayer.twilight.rest/api/private` (authenticated order management)
- **Register**: `POST https://relayer.twilight.rest/register` (get api_key + api_secret)

Authentication for private endpoints requires headers:
- `relayer-api-key`: your api_key
- `signature`: HMAC-SHA256(request_body, api_secret)
- `datetime`: unix timestamp in milliseconds
