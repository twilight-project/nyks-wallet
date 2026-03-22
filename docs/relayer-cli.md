# Relayer CLI

Command-line interface for managing Twilight wallets, trading orders, lending, portfolio tracking, transaction history, and querying market data.

## Building

The CLI requires the `order-wallet` feature (enabled by default).

```bash
# Default build (SQLite backend)
cargo build --release --bin relayer-cli

# With PostgreSQL backend instead
cargo build --release --bin relayer-cli --no-default-features --features postgresql
```

The binary will be at `target/release/relayer-cli`.

## Environment Variables

| Variable                     | Description                                                          | Default                   |
| ---------------------------- | -------------------------------------------------------------------- | ------------------------- |
| `NYKS_WALLET_ID`             | Default wallet ID (fallback when `--wallet-id` is omitted)           | —                         |
| `NYKS_WALLET_PASSPHRASE`     | Database encryption password (fallback when `--password` is omitted) | —                         |
| `RELAYER_API_RPC_SERVER_URL` | Relayer JSON-RPC endpoint                                            | `http://0.0.0.0:8088/api` |
| `DATABASE_URL_SQLITE`        | SQLite database file path                                            | `./wallet_data.db`        |
| `DATABASE_URL_POSTGRESQL`    | PostgreSQL connection string (when using `postgresql` feature)       | —                         |

A `.env` file in the working directory is loaded automatically.

## Password and Wallet ID Resolution

Most commands accept `--wallet-id` and `--password` flags. When omitted, the CLI resolves them through a fallback chain:

**Wallet ID:** `--wallet-id` flag → `NYKS_WALLET_ID` env var → error.

**Password:** `--password` flag → `NYKS_WALLET_PASSPHRASE` env var → session cache (see `wallet unlock`) → none.

## Usage

```
relayer-cli <COMMAND>
```

Six top-level command groups:

- `wallet` — create, import, load, list, export, backup, restore, unlock/lock wallets
- `zkaccount` — fund, withdraw, transfer, and split ZkOS trading accounts
- `order` — open/close/cancel/query trader and lend orders
- `market` — query prices, orderbook, rates
- `history` — view order and transfer history (requires DB)
- `portfolio` — portfolio summary, balances, liquidation risks

---

## Wallet Commands

### `wallet create`

Create a new wallet. Prints the mnemonic phrase once — save it.

```bash
relayer-cli wallet create

# With database persistence
relayer-cli wallet create --with-db --wallet-id my-wallet --password s3cret
```

| Flag                | Description                                                   |
| ------------------- | ------------------------------------------------------------- |
| `--wallet-id <ID>`  | Optional ID for DB storage (defaults to the Twilight address) |
| `--password <PASS>` | DB encryption password                                        |
| `--with-db`         | Enable database persistence                                   |

### `wallet import`

Restore a wallet from a BIP-39 mnemonic.

```bash
relayer-cli wallet import --mnemonic "word1 word2 ... word24"

# With DB persistence
relayer-cli wallet import --mnemonic "..." --with-db --wallet-id restored_wallet_id --password s3cret
```

| Flag                  | Description                           |
| --------------------- | ------------------------------------- |
| `--mnemonic <PHRASE>` | **Required.** 24-word BIP-39 mnemonic |
| `--wallet-id <ID>`    | Optional DB wallet ID                 |
| `--password <PASS>`   | DB encryption password                |
| `--with-db`           | Enable database persistence           |

### `wallet load`

Load a wallet from the database. Requires `sqlite` or `postgresql` feature.

```bash
relayer-cli wallet load --wallet-id my-wallet --password s3cret
```

| Flag                | Description                             |
| ------------------- | --------------------------------------- |
| `--wallet-id <ID>`  | **Required.** Wallet ID in the database |
| `--password <PASS>` | DB encryption password                  |
| `--db-url <URL>`    | Override the default database URL       |

### `wallet balance`

Show NYKS and SATS balance for a wallet.

```bash
relayer-cli wallet balance
relayer-cli wallet balance --wallet-id my-wallet --password s3cret
```

### `wallet list`

List all wallets stored in the database. Requires `sqlite` or `postgresql` feature.

```bash
relayer-cli wallet list
relayer-cli wallet list --db-url "sqlite:///path/to/db"
```

### `wallet export`

Export wallet data to a JSON file.

```bash
relayer-cli wallet export --output wallet-backup.json
relayer-cli wallet export --wallet-id my-wallet --password s3cret
```

| Flag                | Description                               |
| ------------------- | ----------------------------------------- |
| `--output <PATH>`   | Output file path (default: `wallet.json`) |
| `--wallet-id <ID>`  | Load wallet from DB                       |
| `--password <PASS>` | DB encryption password                    |

### `wallet accounts`

List all ZkOS trading accounts associated with a wallet, sorted by account index.

```bash
relayer-cli wallet accounts
relayer-cli wallet accounts --wallet-id my-wallet --password s3cret

# Only show accounts that are on-chain (hide off-chain accounts)
relayer-cli wallet accounts --on-chain-only
```

| Flag                | Description                               |
| ------------------- | ----------------------------------------- |
| `--wallet-id <ID>`  | Load wallet from DB                       |
| `--password <PASS>` | DB encryption password                    |
| `--on-chain-only`   | Only show accounts where on-chain is true |

Output columns: `INDEX`, `BALANCE`, `ON-CHAIN`, `IO-TYPE`, `ACCOUNT`.

### `wallet backup`

Export a full database backup (all tables) to a JSON file. Includes ZkOS accounts, encrypted wallet data, order wallet config, UTXO details, request IDs, order history, and transfer history. Requires `sqlite` or `postgresql` feature.

```bash
relayer-cli wallet backup --wallet-id my-wallet --password s3cret
relayer-cli wallet backup --wallet-id my-wallet --password s3cret --output my-backup.json
```

| Flag                | Description                                      |
| ------------------- | ------------------------------------------------ |
| `--wallet-id <ID>`  | **Required.** Wallet ID to back up               |
| `--password <PASS>` | DB encryption password                           |
| `--output <PATH>`   | Output file path (default: `wallet_backup.json`) |

### `wallet restore`

Restore a wallet from a backup JSON file. Replaces all existing data for the target wallet ID. Requires `sqlite` or `postgresql` feature.

```bash
relayer-cli wallet restore --wallet-id my-wallet --password s3cret --input my-backup.json

# Force restore even if the backup's wallet_id doesn't match the target
relayer-cli wallet restore --wallet-id new-wallet --password s3cret --input my-backup.json --force
```

| Flag                | Description                                                   |
| ------------------- | ------------------------------------------------------------- |
| `--wallet-id <ID>`  | **Required.** Wallet ID to restore into                       |
| `--password <PASS>` | DB encryption password                                        |
| `--input <PATH>`    | **Required.** Backup file path                                |
| `--force`           | Allow restoring even if backup wallet_id doesn't match target |

### `wallet sync-nonce`

Sync the transaction nonce/sequence manager from on-chain state. Useful for debugging sequence issues or pre-warming before a batch of transactions.

```bash
relayer-cli wallet sync-nonce --wallet-id my-wallet --password s3cret
```

Output shows the next sequence number, cached account number, and count of released (reclaimable) sequences.

### `wallet unlock`

Prompt for the database password once and cache it for the current terminal session. Subsequent commands in the same shell will use the cached password automatically, so you don't need to pass `--password` each time.

The cache is scoped to the parent shell process — closing the terminal invalidates it. Only one session password can be active at a time; run `wallet lock` first to replace it.

```bash
relayer-cli wallet unlock
# Enter password at the secure prompt, then:
relayer-cli wallet balance --wallet-id my-wallet   # no --password needed
```

### `wallet lock`

Clear the cached session password immediately.

```bash
relayer-cli wallet lock
```

---

## ZkOS Account Commands

Manage ZkOS trading accounts — fund from on-chain, withdraw back, transfer between accounts, or split into multiple accounts.

All zkaccount commands require `--wallet-id` (or the `NYKS_WALLET_ID` env var) to identify which wallet to use. `--password` falls back to `NYKS_WALLET_PASSPHRASE` env var or the session cache set by `wallet unlock`.

### `zkaccount fund`

Fund a new ZkOS trading account from the on-chain wallet.

```bash
relayer-cli zkaccount fund --amount 100000
relayer-cli zkaccount fund --amount 100000 --wallet-id my-wallet --password s3cret
```

| Flag              | Description                              |
| ----------------- | ---------------------------------------- |
| `--amount <SATS>` | **Required.** Amount in satoshis to fund |

### `zkaccount withdraw`

Withdraw from a ZkOS trading account back to the on-chain wallet.

```bash
relayer-cli zkaccount withdraw --account-index 0
```

| Flag                  | Description                                       |
| --------------------- | ------------------------------------------------- |
| `--account-index <N>` | **Required.** ZkOS account index to withdraw from |

### `zkaccount transfer`

Transfer funds between ZkOS trading accounts (creates a new destination account).

```bash
relayer-cli zkaccount transfer --from 0
```

| Flag         | Description                       |
| ------------ | --------------------------------- |
| `--from <N>` | **Required.** Source account index |

### `zkaccount split`

Split a ZkOS trading account into multiple new accounts with specified balances.

```bash
relayer-cli zkaccount split --from 0 --balances "1000,2000,3000"
```

| Flag                | Description                                                                     |
| ------------------- | ------------------------------------------------------------------------------- |
| `--from <N>`        | **Required.** Source account index                                              |
| `--balances <LIST>` | **Required.** Comma-separated list of balances in satoshis for the new accounts |

---

## Order Commands

Trading and lending order commands. All order commands require `--wallet-id` (or the `NYKS_WALLET_ID` env var) to identify which wallet to use. `--password` falls back to `NYKS_WALLET_PASSPHRASE` env var or the session cache set by `wallet unlock`.

### `order open-trade`

Open a leveraged derivative position.

```bash
# Market long at $65000, 10x leverage
relayer-cli order open-trade --account-index 0 --side LONG --entry-price 65000 --leverage 10

# Limit short
relayer-cli order open-trade --account-index 1 --order-type LIMIT --side SHORT --entry-price 70000 --leverage 5
```

| Flag                   | Description                                |
| ---------------------- | ------------------------------------------ |
| `--account-index <N>`  | **Required.** ZkOS account index           |
| `--side <LONG\|SHORT>` | **Required.** Position direction           |
| `--entry-price <USD>`  | **Required.** Entry price in USD (integer) |
| `--leverage <1-50>`    | **Required.** Leverage multiplier          |
| `--order-type <TYPE>`  | `MARKET` (default) or `LIMIT`              |

### `order close-trade`

Close an open trader position.

```bash
# Market close
relayer-cli order close-trade --account-index 0

# Close with stop-loss / take-profit (uses SLTP order type automatically)
relayer-cli order close-trade --account-index 0 --stop-loss 60000 --take-profit 75000
```

| Flag                    | Description                                 |
| ----------------------- | ------------------------------------------- |
| `--account-index <N>`   | **Required.** ZkOS account index            |
| `--order-type <TYPE>`   | `MARKET` (default) or `LIMIT`               |
| `--execution-price <F>` | Execution price (default: `0.0` for market) |
| `--stop-loss <F>`       | Stop-loss price (triggers SLTP close)       |
| `--take-profit <F>`     | Take-profit price (triggers SLTP close)     |

### `order cancel-trade`

Cancel a pending trader order.

```bash
relayer-cli order cancel-trade --account-index 0
```

### `order query-trade`

Query the status of a trader order. Outputs JSON.

```bash
relayer-cli order query-trade --account-index 0
```

### `order unlock-trade`

Unlock a settled trader order. Use this to reclaim a ZkOS account after an SLTP (stop-loss/take-profit) order has been settled by the relayer. If the order is not yet settled, no changes are made.

```bash
relayer-cli order unlock-trade --account-index 0
relayer-cli order unlock-trade --account-index 0 --wallet-id my-wallet --password s3cret
```

| Flag                  | Description                            |
| --------------------- | -------------------------------------- |
| `--account-index <N>` | **Required.** ZkOS account index       |
| `--wallet-id <ID>`    | Load wallet from DB                    |
| `--password <PASS>`   | DB encryption password                 |

### `order open-lend`

Open a lending order on a ZkOS account.

```bash
relayer-cli order open-lend --account-index 0
```

### `order close-lend`

Close an active lending order.

```bash
relayer-cli order close-lend --account-index 0
```

### `order query-lend`

Query the status of a lending order. Outputs JSON.

```bash
relayer-cli order query-lend --account-index 0
```

---

## History Commands

View transaction history stored in the database. Requires `sqlite` or `postgresql` feature.

### `history orders`

Show order history (open, close, cancel events) with pagination.

```bash
relayer-cli history orders --wallet-id my-wallet --password s3cret

# Filter by account index
relayer-cli history orders --wallet-id my-wallet --password s3cret --account-index 3

# Pagination
relayer-cli history orders --wallet-id my-wallet --password s3cret --limit 20 --offset 40
```

| Flag                  | Description                        |
| --------------------- | ---------------------------------- |
| `--wallet-id <ID>`    | **Required.** Wallet ID            |
| `--password <PASS>`   | DB encryption password             |
| `--account-index <N>` | Filter to a specific account index |
| `--limit <N>`         | Max results (default: `50`)        |
| `--offset <N>`        | Pagination offset (default: `0`)   |

Output columns: `ACCT`, `ACTION`, `TYPE`, `SIDE`, `AMOUNT`, `PRICE`, `STATUS`, `CREATED`.

### `history transfers`

Show transfer history (fund-to-trade, trade-to-fund, trade-to-trade events) with pagination.

```bash
relayer-cli history transfers --wallet-id my-wallet --password s3cret
relayer-cli history transfers --wallet-id my-wallet --password s3cret --limit 10
```

| Flag                | Description                      |
| ------------------- | -------------------------------- |
| `--wallet-id <ID>`  | **Required.** Wallet ID          |
| `--password <PASS>` | DB encryption password           |
| `--limit <N>`       | Max results (default: `50`)      |
| `--offset <N>`      | Pagination offset (default: `0`) |

Output columns: `DIRECTION`, `FROM`, `TO`, `AMOUNT`, `CREATED`, `TX HASH`.

---

## Portfolio Commands

View portfolio state, per-account balances, and liquidation risk. These commands query live relayer data for open positions.

### `portfolio summary`

Show a full portfolio summary including on-chain balance, trading balances, margin usage, PnL, and all open trader and lend positions.

```bash
relayer-cli portfolio summary
relayer-cli portfolio summary --wallet-id my-wallet --password s3cret
```

Output includes:
- On-chain and trading balances
- Total margin used and utilization percentage
- Unrealized PnL across all positions
- Lend deposits, current value, and lend PnL
- Per-position table for trader orders (entry/current price, size, leverage, PnL, liquidation price, funding)
- Per-position table for lend orders (deposit, current value, PnL, unrealised PnL, APR, pool shares)

### `portfolio balances`

Show a per-account balance breakdown for all ZkOS accounts.

```bash
relayer-cli portfolio balances
relayer-cli portfolio balances --wallet-id my-wallet --password s3cret
```

Output columns: `INDEX`, `BALANCE`, `IO-TYPE`, `ON-CHAIN`, plus a total.

### `portfolio risks`

Show liquidation risk for all open trader positions. Queries live price data from the relayer.

```bash
relayer-cli portfolio risks
relayer-cli portfolio risks --wallet-id my-wallet --password s3cret
```

Output columns: `ACCT`, `SIDE`, `CURRENT`, `LIQ PRICE`, `DISTANCE` (% from liquidation), `MARGIN %`.

- Positive distance = safe margin
- Negative distance = past liquidation threshold

---

## Market Commands

Market commands query the relayer JSON-RPC API. No wallet is needed.

### `market price`

Get the current BTC/USD price.

```bash
relayer-cli market price
```

### `market orderbook`

Get the current order book (open limit orders), showing bids and asks.

```bash
relayer-cli market orderbook
```

### `market funding-rate`

Get the current funding rate.

```bash
relayer-cli market funding-rate
```

### `market fee-rate`

Get current fee rates for market/limit fills and settlements.

```bash
relayer-cli market fee-rate
```

### `market recent-trades`

Get recent trade orders.

```bash
relayer-cli market recent-trades
```

### `market position-size`

Get aggregate long/short position sizes.

```bash
relayer-cli market position-size
```

### `market lend-pool`

Get lending pool information. Outputs JSON.

```bash
relayer-cli market lend-pool
```

### `market pool-share-value`

Get the current pool share value.

```bash
relayer-cli market pool-share-value
```

### `market last-day-apy`

Get the annualized percentage yield (APY) for the last 24 hours.

```bash
relayer-cli market last-day-apy
```

### `market open-interest`

Get current open interest showing long and short exposure in BTC.

```bash
relayer-cli market open-interest
```

### `market market-stats`

Get comprehensive market risk statistics including pool equity, exposure, utilization, and risk parameters.

```bash
relayer-cli market market-stats
```

### `market server-time`

Get the relayer server's current UTC time.

```bash
relayer-cli market server-time
```

---

## Logging

Set the `RUST_LOG` environment variable to control log output:

```bash
RUST_LOG=info relayer-cli wallet balance
RUST_LOG=debug relayer-cli order fund --amount 50000
```
