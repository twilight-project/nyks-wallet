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

A `.env` file in the working directory is loaded automatically. See `.env.example` for a complete template.

### Core

| Variable         | Description                                                                   | Default    |
| ---------------- | ----------------------------------------------------------------------------- | ---------- |
| `RUST_LOG`       | Logging level (`info`, `debug`, `trace`)                                      | —          |
| `RUST_BACKTRACE` | Enable Rust backtraces for debugging (`1` or `full`)                          | —          |
| `CHAIN_ID`       | Blockchain network identifier                                                 | `nyks`     |
| `NETWORK_TYPE`   | Network type — controls BIP-44 coin type (`testnet` uses coin type 1, `mainnet` uses 118) | `mainnet`  |

### Chain Endpoints

Used by the on-chain wallet for balance queries, transaction broadcasting, and faucet requests.

| Variable           | Description                                          | Default                |
| ------------------ | ---------------------------------------------------- | ---------------------- |
| `NYKS_RPC_BASE_URL`| Nyks chain Tendermint RPC endpoint                   | `http://0.0.0.0:26657` |
| `NYKS_LCD_BASE_URL`| Nyks chain LCD (REST API) endpoint                   | `http://0.0.0.0:1317`  |
| `FAUCET_BASE_URL`  | Faucet endpoint for requesting test tokens (testnet only) | `http://0.0.0.0:6969`  |

### Order-Wallet Endpoints (feature: `order-wallet`)

Required when using trading, lending, or ZkOS account commands.

| Variable                     | Description                                                       | Default                   |
| ---------------------------- | ----------------------------------------------------------------- | ------------------------- |
| `RELAYER_API_RPC_SERVER_URL` | Relayer public JSON-RPC API endpoint                              | `http://0.0.0.0:8088/api` |
| `ZKOS_SERVER_URL`            | ZkOS server endpoint for zero-knowledge operations (also used by `relayer-init`) | `http://0.0.0.0:3030`     |
| `RELAYER_PROGRAM_JSON_PATH`  | Path to the relayer program JSON file (ZkOS circuit parameters)   | `./relayerprogram.json`   |

### Wallet Configuration

| Variable               | Description                                                                         | Default          |
| ---------------------- | ----------------------------------------------------------------------------------- | ---------------- |
| `NYKS_WALLET_ID`       | Default wallet ID (fallback when `--wallet-id` is omitted)                          | —                |
| `NYKS_WALLET_PASSPHRASE` | Database encryption password (fallback when `--password` is omitted). Leave empty to use interactive prompt | — |

### Validator-Wallet (feature: `validator-wallet`)

| Variable               | Description                        | Default               |
| ---------------------- | ---------------------------------- | --------------------- |
| `VALIDATOR_WALLET_PATH`| Path to the validator mnemonic file| `validator.mnemonic`  |

### Database (feature: `sqlite` or `postgresql`)

Required for persisting wallets, ZkOS accounts, UTXOs, and order history.

| Variable                 | Description                                                  | Default            |
| ------------------------ | ------------------------------------------------------------ | ------------------ |
| `DATABASE_URL_SQLITE`    | SQLite database file path (feature: `sqlite`, enabled by default) | `./wallet_data.db` |
| `DATABASE_URL_POSTGRESQL`| PostgreSQL connection string (feature: `postgresql`)         | —                  |

## Password and Wallet ID Resolution

Most commands accept `--wallet-id` and `--password` flags. When omitted, the CLI resolves them through a fallback chain:

**Wallet ID:** `--wallet-id` flag → session cache (see `wallet unlock`) → `NYKS_WALLET_ID` env var → error.

**Password:** `--password` flag → session cache (see `wallet unlock`) → `NYKS_WALLET_PASSPHRASE` env var → none.

## Usage

```
relayer-cli [--json] <COMMAND>
```

### Global Flags

| Flag     | Description                                                     |
| -------- | --------------------------------------------------------------- |
| `--json` | Output results as JSON instead of formatted tables (for scripting) |

### Command Groups

- `wallet` — create, import, load, list, export, backup, restore, unlock/lock, info, change-password, update-btc-address, sync-nonce
- `zkaccount` — fund, withdraw, transfer, and split ZkOS trading accounts
- `order` — open/close/cancel/query trader and lend orders, unlock-trade, history-trade, history-lend, funding-history, account-summary, tx-hashes
- `market` — query prices, orderbook, rates, historical data, candles, APY charts
- `history` — view order and transfer history (requires DB)
- `portfolio` — portfolio summary, balances (with unit conversion), liquidation risks
- `help` — show help for the CLI or a specific command group

### Built-in Help

```bash
relayer-cli help                # global overview
relayer-cli help wallet         # wallet subcommands and examples
relayer-cli help order          # order subcommands and examples
relayer-cli help market         # market subcommands and examples
```

---

## Wallet Commands

### `wallet create`

Create a new wallet and persist it to the database. Prints the mnemonic phrase once — save it.

```bash
relayer-cli wallet create
relayer-cli wallet create --wallet-id my-wallet --password s3cret
relayer-cli wallet create --btc-address bc1q...
```

| Flag                   | Description                                                   |
| ---------------------- | ------------------------------------------------------------- |
| `--wallet-id <ID>`    | Wallet ID for DB storage (defaults to the Twilight address)   |
| `--password <PASS>`   | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |
| `--btc-address <ADDR>`| BTC SegWit address (`bc1q...` or `bc1p...`) to use instead of generating a random one |

### `wallet import`

Import a wallet from a BIP-39 mnemonic and persist it to the database. If `--mnemonic` is omitted, the CLI prompts for it securely via TTY (the phrase is not echoed to the terminal).

```bash
# Prompt for mnemonic securely (recommended)
relayer-cli wallet import

# Pass mnemonic on command line
relayer-cli wallet import --mnemonic "word1 word2 ... word24"

relayer-cli wallet import --wallet-id restored --password s3cret
relayer-cli wallet import --btc-address bc1q...
```

| Flag                   | Description                           |
| ---------------------- | ------------------------------------- |
| `--mnemonic <PHRASE>` | 24-word BIP-39 mnemonic. If omitted, prompts securely via TTY |
| `--wallet-id <ID>`    | Wallet ID for DB storage (defaults to the Twilight address)   |
| `--password <PASS>`   | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |
| `--btc-address <ADDR>`| BTC SegWit address (`bc1q...` or `bc1p...`) to use instead of deriving from mnemonic |

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

Prompt for wallet ID and password, then cache both for the current terminal session. Subsequent commands in the same shell will use the cached values automatically, so you don't need to pass `--wallet-id` or `--password` each time.

Before prompting for the wallet ID, the CLI lists all available wallets in the database (similar to `wallet list`). You can also pass `--wallet-id` on the command line to skip the interactive prompt.

The cache is scoped to the parent shell process — closing the terminal invalidates it. Only one session can be active at a time; use `--force` to overwrite an existing cached session, or run `wallet lock` first.

Resolution priority for both wallet ID and password: `--flag` > session cache > environment variable.

```bash
relayer-cli wallet unlock
# Lists available wallets, prompts for wallet ID and password, then:
relayer-cli wallet balance   # no --wallet-id or --password needed

# Pass wallet ID directly (skip interactive prompt)
relayer-cli wallet unlock --wallet-id my-wallet

# Overwrite an existing session
relayer-cli wallet unlock --force
```

| Flag          | Description                                        |
| ------------- | -------------------------------------------------- |
| `--wallet-id` | Wallet ID to cache (prompts interactively if omitted) |
| `--force`     | Overwrite an existing session without error        |

### `wallet lock`

Clear the cached session (wallet ID and password) immediately.

```bash
relayer-cli wallet lock
```

### `wallet change-password`

Change the database encryption password for a wallet. Always prompts for both old and new passwords via secure TTY input — the session cache and `NYKS_WALLET_PASSPHRASE` env var are intentionally ignored to prevent accidental password changes.

```bash
relayer-cli wallet change-password --wallet-id my-wallet
```

| Flag               | Description                                         |
| ------------------ | --------------------------------------------------- |
| `--wallet-id <ID>` | Wallet to change password for (falls back to `NYKS_WALLET_ID`) |

If a session cache exists, it is updated with the new password automatically.

### `wallet info`

Show wallet info without making any chain calls. Displays address, BTC address, chain ID, account count, and nonce state. Requires DB.

```bash
relayer-cli wallet info
relayer-cli wallet info --wallet-id my-wallet --password s3cret
```

| Flag                | Description                      |
| ------------------- | -------------------------------- |
| `--wallet-id <ID>`  | Load wallet from DB              |
| `--password <PASS>` | DB encryption password           |

### `wallet update-btc-address`

Update the BTC deposit address stored in the wallet. The `btc_address_registered` flag is reset to `false` — the address will be re-registered on the next balance check.

```bash
relayer-cli wallet update-btc-address --btc-address bc1q... --wallet-id my-wallet
```

| Flag                    | Description                                          |
| ----------------------- | ---------------------------------------------------- |
| `--btc-address <ADDR>`  | **Required.** New BTC address                        |
| `--wallet-id <ID>`      | Wallet ID (falls back to `NYKS_WALLET_ID`)           |
| `--password <PASS>`     | DB encryption password                               |

---

## ZkOS Account Commands

Manage ZkOS trading accounts — fund from on-chain, withdraw back, transfer between accounts, or split into multiple accounts.

All zkaccount commands require a wallet ID and password. Resolution priority: `--flag` > session cache (`wallet unlock`) > env var (`NYKS_WALLET_ID` / `NYKS_WALLET_PASSPHRASE`).

### `zkaccount fund`

Fund a new ZkOS trading account from the on-chain wallet. Provide exactly one of `--amount`, `--amount-mbtc`, or `--amount-btc`. If multiple are given, priority is: `--amount` > `--amount-mbtc` > `--amount-btc`.

```bash
# In satoshis
relayer-cli zkaccount fund --amount 100000

# In milli-BTC (1 mBTC = 100,000 sats)
relayer-cli zkaccount fund --amount-mbtc 1.0

# In BTC (1 BTC = 100,000,000 sats)
relayer-cli zkaccount fund --amount-btc 0.001
```

| Flag                  | Description                                   |
| --------------------- | --------------------------------------------- |
| `--amount <SATS>`     | Amount in satoshis                            |
| `--amount-mbtc <MBTC>`| Amount in milli-BTC (1 mBTC = 100,000 sats)  |
| `--amount-btc <BTC>`  | Amount in BTC (1 BTC = 100,000,000 sats)     |

At least one amount flag is required. All values are converted to satoshis before funding.

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

| Flag         | Description                        |
| ------------ | ---------------------------------- |
| `--from <N>` | **Required.** Source account index |

### `zkaccount split`

Split a ZkOS trading account into multiple new accounts with specified balances.

Balances can be provided in three units — **exactly one** must be used. If multiple are provided, priority is: `--balances` > `--balances-mbtc` > `--balances-btc` (a warning is shown).

```bash
# In satoshis
relayer-cli zkaccount split --from 0 --balances "1000,2000,3000"

# In milli-BTC (1 mBTC = 100,000 sats)
relayer-cli zkaccount split --from 0 --balances-mbtc "0.01,0.02,0.03"

# In BTC (1 BTC = 100,000,000 sats)
relayer-cli zkaccount split --from 0 --balances-btc "0.00001,0.00002,0.00003"
```

| Flag                     | Description                                                                    |
| ------------------------ | ------------------------------------------------------------------------------ |
| `--from <N>`             | **Required.** Source account index                                             |
| `--balances <LIST>`      | Comma-separated list of balances in **satoshis**. Priority 1 if multiple given |
| `--balances-mbtc <LIST>` | Comma-separated list of balances in **milli-BTC** (×100,000). Priority 2       |
| `--balances-btc <LIST>`  | Comma-separated list of balances in **BTC** (×100,000,000). Priority 3         |

At least one balance flag is required. All values are converted to satoshis internally. Zero-value balances are rejected.

---

## Order Commands

Trading and lending order commands. All order commands require a wallet ID and password. Resolution priority: `--flag` > session cache (`wallet unlock`) > env var (`NYKS_WALLET_ID` / `NYKS_WALLET_PASSPHRASE`).

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

| Flag                  | Description                      |
| --------------------- | -------------------------------- |
| `--account-index <N>` | **Required.** ZkOS account index |
| `--wallet-id <ID>`    | Load wallet from DB              |
| `--password <PASS>`   | DB encryption password           |

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

### `order history-trade`

Query historical trader orders for an account from the relayer (not local DB). Requires wallet access for signing.

```bash
relayer-cli order history-trade --account-index 0
relayer-cli --json order history-trade --account-index 0
```

| Flag                  | Description                      |
| --------------------- | -------------------------------- |
| `--account-index <N>` | **Required.** ZkOS account index |

Output columns: `UUID`, `STATUS`, `TYPE`, `SIDE`, `ENTRY`, `SIZE`, `LEV`, `MARGIN`, `PnL`.

### `order history-lend`

Query historical lend orders for an account from the relayer (not local DB). Requires wallet access for signing.

```bash
relayer-cli order history-lend --account-index 0
relayer-cli --json order history-lend --account-index 0
```

| Flag                  | Description                      |
| --------------------- | -------------------------------- |
| `--account-index <N>` | **Required.** ZkOS account index |

Output columns: `UUID`, `STATUS`, `DEPOSIT`, `BALANCE`, `SHARES`, `PAYMENT`.

### `order funding-history`

Query funding payment history for a position. Shows each funding interval's payment, rate, and order ID.

```bash
relayer-cli order funding-history --account-index 0
relayer-cli --json order funding-history --account-index 0
```

| Flag                  | Description                      |
| --------------------- | -------------------------------- |
| `--account-index <N>` | **Required.** ZkOS account index |

Output columns: `TIME`, `SIDE`, `PAYMENT`, `RATE`, `ORDER ID`, plus total funding sum.

### `order account-summary`

Query your wallet's trading activity summary from the relayer — filled, settled, and liquidated counts and position sizes.

```bash
relayer-cli order account-summary
relayer-cli order account-summary --from 2024-01-01 --to 2024-12-31
relayer-cli --json order account-summary
```

| Flag            | Description                                           |
| --------------- | ----------------------------------------------------- |
| `--from <DATE>` | Start date filter (RFC3339 or YYYY-MM-DD)             |
| `--to <DATE>`   | End date filter (RFC3339 or YYYY-MM-DD)               |
| `--since <DATE>`| Alternative date filter (RFC3339 or YYYY-MM-DD)       |

### `order tx-hashes`

Look up on-chain transaction hashes by request ID, account address, or tx ID.

```bash
# By request ID (default)
relayer-cli order tx-hashes --id REQID9804F25B...

# By account address
relayer-cli order tx-hashes --by account --id <account_address>

# By tx ID
relayer-cli order tx-hashes --by tx --id <tx_id>

# Filter by status
relayer-cli order tx-hashes --id REQID... --status FILLED
```

| Flag              | Description                                          |
| ----------------- | ---------------------------------------------------- |
| `--by <MODE>`     | Lookup mode: `request` (default), `account`, or `tx` |
| `--id <ID>`       | **Required.** The ID to look up                      |
| `--status <S>`    | Filter by order status (PENDING, FILLED, SETTLED, etc.) |
| `--limit <N>`     | Max results                                          |
| `--offset <N>`    | Pagination offset                                    |

Output columns: `ORDER ID`, `STATUS`, `TYPE`, `TX HASH`, `DATE`.

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
- Mark price, on-chain and trading balances
- Total margin used and utilization percentage
- Unrealized PnL across open positions (excludes PENDING orders)
- Realised PnL from settled positions
- Liquidation loss from liquidated positions
- Lend deposits, current value, and lend PnL
- **Trader Positions** table: `ACCT`, `STATUS`, `SIDE`, `ENTRY`, `SIZE`, `LEV`, `A.MARGIN`, `U_PnL`, `LIQ PRICE`, `FEE`, `FUNDING`, `LIMIT`, `TP`, `SL`
- **Closed Positions** table (settled & unlocked): `ACCT`, `SIDE`, `ENTRY`, `SIZE`, `LEV`, `A.MARGIN`, `R_PnL`, `NET_PnL`, `FEE_FILL`, `FEE_SETT`, `FUNDING`, plus total realised PnL
- **Liquidated Positions** table: `ACCT`, `SIDE`, `ENTRY`, `SIZE`, `LEV`, `I.MARGIN`, `FEE_FILL`, `FEE_SETT`, plus total liquidation loss
- **Lend Positions** table: `ACCT`, `DEPOSIT`, `VALUE`, `PnL`, `uPnL`, `APR %`, `SHARES`

Settled and liquidated accounts are automatically unlocked (restored to Coin state) when the summary is generated.

### `portfolio balances`

Show a per-account balance breakdown for all ZkOS accounts. Supports display in different units.

```bash
relayer-cli portfolio balances
relayer-cli portfolio balances --unit mbtc
relayer-cli portfolio balances --unit btc
relayer-cli portfolio balances --wallet-id my-wallet --password s3cret
```

| Flag          | Description                                |
| ------------- | ------------------------------------------ |
| `--unit <U>`  | Display unit: `sats` (default), `mbtc`, `btc` |

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

### `market history-price`

Query historical BTC/USD prices over a date range.

```bash
relayer-cli market history-price --from 2024-01-01 --to 2024-01-31
relayer-cli market history-price --from 2024-01-01T00:00:00Z --to 2024-01-31T23:59:59Z --limit 100
relayer-cli --json market history-price --from 2024-01-01 --to 2024-01-31
```

| Flag              | Description                                     |
| ----------------- | ----------------------------------------------- |
| `--from <DATE>`   | **Required.** Start date (RFC3339 or YYYY-MM-DD)|
| `--to <DATE>`     | **Required.** End date (RFC3339 or YYYY-MM-DD)  |
| `--limit <N>`     | Max results (default: `50`)                     |
| `--offset <N>`    | Pagination offset (default: `0`)                |

### `market candles`

Query OHLCV candlestick data.

```bash
relayer-cli market candles --since 2024-01-01 --interval 1h
relayer-cli market candles --since 2024-01-01 --interval 1d --limit 30
relayer-cli --json market candles --since 2024-01-01 --interval 5m
```

| Flag              | Description                                         |
| ----------------- | --------------------------------------------------- |
| `--since <DATE>`  | **Required.** Start date (RFC3339 or YYYY-MM-DD)    |
| `--interval <I>`  | Candle interval: `1m`, `5m`, `15m`, `30m`, `1h` (default), `4h`, `8h`, `12h`, `1d` |
| `--limit <N>`     | Max results (default: `50`)                         |
| `--offset <N>`    | Pagination offset (default: `0`)                    |

Output columns: `START`, `OPEN`, `HIGH`, `LOW`, `CLOSE`, `VOLUME`, `TRADES`.

### `market history-funding`

Query historical funding rates over a date range.

```bash
relayer-cli market history-funding --from 2024-01-01 --to 2024-01-31
relayer-cli --json market history-funding --from 2024-01-01 --to 2024-01-31
```

| Flag              | Description                                     |
| ----------------- | ----------------------------------------------- |
| `--from <DATE>`   | **Required.** Start date (RFC3339 or YYYY-MM-DD)|
| `--to <DATE>`     | **Required.** End date (RFC3339 or YYYY-MM-DD)  |
| `--limit <N>`     | Max results (default: `50`)                     |
| `--offset <N>`    | Pagination offset (default: `0`)                |

### `market history-fees`

Query historical fee rates over a date range.

```bash
relayer-cli market history-fees --from 2024-01-01 --to 2024-01-31
relayer-cli --json market history-fees --from 2024-01-01 --to 2024-01-31
```

| Flag              | Description                                     |
| ----------------- | ----------------------------------------------- |
| `--from <DATE>`   | **Required.** Start date (RFC3339 or YYYY-MM-DD)|
| `--to <DATE>`     | **Required.** End date (RFC3339 or YYYY-MM-DD)  |
| `--limit <N>`     | Max results (default: `50`)                     |
| `--offset <N>`    | Pagination offset (default: `0`)                |

Output columns: `MKT FILL`, `LMT FILL`, `MKT SETTLE`, `LMT SETTLE`, `TIMESTAMP`.

### `market apy-chart`

Query APY chart data for the lend pool over time.

```bash
relayer-cli market apy-chart
relayer-cli market apy-chart --range 30d --step 1d
relayer-cli --json market apy-chart --range 7d
```

| Flag              | Description                                  |
| ----------------- | -------------------------------------------- |
| `--range <R>`     | Time range, e.g. `7d`, `30d`, `1y` (default: `7d`) |
| `--step <S>`      | Step/granularity, e.g. `1h`, `1d`            |
| `--lookback <L>`  | Lookback period for rolling average          |

Output columns: `TIME`, `APY %`.

---

## Logging

Set the `RUST_LOG` environment variable to control log output:

```bash
RUST_LOG=info relayer-cli wallet balance
RUST_LOG=debug relayer-cli order fund --amount 50000
```
