# Relayer CLI

Command-line interface for managing Twilight wallets, trading orders, lending, and querying market data.

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

| Variable | Description | Default |
|---|---|---|
| `RELAYER_API_RPC_SERVER_URL` | Relayer JSON-RPC endpoint | `http://0.0.0.0:8088/api` |
| `NYKS_WALLET_PASSPHRASE` | Database encryption password (fallback when `--password` is omitted) | — |

A `.env` file in the working directory is loaded automatically.

## Usage

```
relayer-cli <COMMAND>
```

Three top-level command groups:

- `wallet` — create, import, load, list, export wallets
- `order` — fund accounts, trade, lend, withdraw
- `market` — query prices, orderbook, rates

---

## Wallet Commands

### `wallet create`

Create a new wallet. Prints the mnemonic phrase once — save it.

```bash
relayer-cli wallet create

# With database persistence
relayer-cli wallet create --with-db --wallet-id my-wallet --password s3cret
```

| Flag | Description |
|---|---|
| `--wallet-id <ID>` | Optional ID for DB storage (defaults to the Twilight address) |
| `--password <PASS>` | DB encryption password |
| `--with-db` | Enable database persistence |

### `wallet import`

Restore a wallet from a BIP-39 mnemonic.

```bash
relayer-cli wallet import --mnemonic "word1 word2 ... word24"

# With DB persistence
relayer-cli wallet import --mnemonic "..." --with-db --wallet-id restored --password s3cret
```

| Flag | Description |
|---|---|
| `--mnemonic <PHRASE>` | **Required.** 24-word BIP-39 mnemonic |
| `--wallet-id <ID>` | Optional DB wallet ID |
| `--password <PASS>` | DB encryption password |
| `--with-db` | Enable database persistence |

### `wallet load`

Load a wallet from the database. Requires `sqlite` or `postgresql` feature.

```bash
relayer-cli wallet load --wallet-id my-wallet --password s3cret
```

| Flag | Description |
|---|---|
| `--wallet-id <ID>` | **Required.** Wallet ID in the database |
| `--password <PASS>` | DB encryption password |
| `--db-url <URL>` | Override the default database URL |

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

| Flag | Description |
|---|---|
| `--output <PATH>` | Output file path (default: `wallet.json`) |
| `--wallet-id <ID>` | Load wallet from DB |
| `--password <PASS>` | DB encryption password |

### `wallet accounts`

List all ZkOS trading accounts associated with a wallet.

```bash
relayer-cli wallet accounts
relayer-cli wallet accounts --wallet-id my-wallet --password s3cret
```

Output columns: `INDEX`, `BALANCE`, `ON-CHAIN`, `IO-TYPE`, `ACCOUNT`.

---

## Order Commands

All order commands accept `--wallet-id` and `--password` to load a persisted wallet. Without these flags, a fresh in-memory wallet is created.

### `order fund`

Fund a new ZkOS trading account from the on-chain wallet.

```bash
relayer-cli order fund --amount 100000
relayer-cli order fund --amount 100000 --wallet-id my-wallet --password s3cret
```

| Flag | Description |
|---|---|
| `--amount <SATS>` | **Required.** Amount in satoshis to fund |

### `order withdraw`

Withdraw from a ZkOS trading account back to the on-chain wallet.

```bash
relayer-cli order withdraw --account-index 0
```

| Flag | Description |
|---|---|
| `--account-index <N>` | **Required.** ZkOS account index to withdraw from |

### `order transfer`

Transfer funds between ZkOS trading accounts (creates a new destination account).

```bash
relayer-cli order transfer --from 0
```

| Flag | Description |
|---|---|
| `--from <N>` | **Required.** Source account index |

### `order open-trade`

Open a leveraged derivative position.

```bash
# Market long at $65000, 10x leverage
relayer-cli order open-trade --account-index 0 --side LONG --entry-price 65000 --leverage 10

# Limit short
relayer-cli order open-trade --account-index 1 --order-type LIMIT --side SHORT --entry-price 70000 --leverage 5
```

| Flag | Description |
|---|---|
| `--account-index <N>` | **Required.** ZkOS account index |
| `--side <LONG\|SHORT>` | **Required.** Position direction |
| `--entry-price <USD>` | **Required.** Entry price in USD (integer) |
| `--leverage <1-50>` | **Required.** Leverage multiplier |
| `--order-type <TYPE>` | `MARKET` (default) or `LIMIT` |

### `order close-trade`

Close an open trader position.

```bash
# Market close
relayer-cli order close-trade --account-index 0

# Close with stop-loss / take-profit (uses SLTP order type automatically)
relayer-cli order close-trade --account-index 0 --stop-loss 60000 --take-profit 75000
```

| Flag | Description |
|---|---|
| `--account-index <N>` | **Required.** ZkOS account index |
| `--order-type <TYPE>` | `MARKET` (default) or `LIMIT` |
| `--execution-price <F>` | Execution price (default: `0.0` for market) |
| `--stop-loss <F>` | Stop-loss price (triggers SLTP close) |
| `--take-profit <F>` | Take-profit price (triggers SLTP close) |

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

---

## Logging

Set the `RUST_LOG` environment variable to control log output:

```bash
RUST_LOG=info relayer-cli wallet balance
RUST_LOG=debug relayer-cli order fund --amount 50000
```
