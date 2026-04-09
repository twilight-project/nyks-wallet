# Relayer CLI

Command-line interface for managing Twilight wallets, trading orders, lending, portfolio tracking, transaction history, and querying market data.

## Building

### Prerequisites

- **Rust** — edition 2024 (install via [rustup](https://rustup.rs/))
- **Protocol Buffers compiler** (`protoc`) — required by `build.rs` for protobuf code generation
- **pkg-config** and **OpenSSL dev headers** (`libssl-dev`) — required by several dependencies
- **PostgreSQL client library** (`libpq-dev`) — only if building with the `postgresql` feature

**macOS:**

```bash
brew install protobuf openssl pkg-config
# For PostgreSQL backend:
brew install libpq
```

**Debian/Ubuntu:**

```bash
sudo apt-get install -y protobuf-compiler pkg-config libssl-dev
# For PostgreSQL backend:
sudo apt-get install -y libpq-dev
```

**Docker:** The included `Dockerfile` uses `rust:1.89` and installs all required dependencies automatically.

### Build

The CLI requires the `order-wallet` feature (enabled by default).

```bash
# Default build (SQLite backend — bundled libsqlite3, no system dependency)
cargo build --release --bin relayer-cli

# With PostgreSQL backend instead (requires libpq)
cargo build --release --bin relayer-cli --no-default-features --features postgresql
```

The binary will be at `target/release/relayer-cli`.

### Docker

```bash
docker build -t relayer-cli .
docker run -e RUST_LOG=info -e NETWORK_TYPE=testnet relayer-cli wallet balance
```

## Environment Variables

A `.env` file in the working directory is loaded automatically. See `.env.example` for a complete template.
If a variable is not set, `src/config.rs` applies code defaults (many are derived from `NETWORK_TYPE`).

### Core

| Variable           | Description                                                                                                | Default   |
| ------------------ | ---------------------------------------------------------------------------------------------------------- | --------- |
| `RUST_LOG`         | Logging level (`info`, `debug`, `trace`)                                                                   | —         |
| `RUST_BACKTRACE`   | Enable Rust backtraces for debugging (`1` or `full`)                                                       | —         |
| `CHAIN_ID`         | Blockchain network identifier                                                                              | `nyks`    |
| `NETWORK_TYPE`     | Network type — controls default endpoint URLs and BIP-44 derivation (coin type 118 for both networks)      | `mainnet` |
| `BTC_NETWORK_TYPE` | Bitcoin network for BTC key derivation, balance queries, and transfers. Falls back to `mainnet` if not set | `mainnet` |

### Chain Endpoints

Used by the on-chain wallet for balance queries, transaction broadcasting, and faucet requests. Defaults are selected from `NETWORK_TYPE`.

| Variable                   | Description                                                           | Default (`mainnet`)                | Default (`testnet`)                    |
| -------------------------- | --------------------------------------------------------------------- | ---------------------------------- | -------------------------------------- |
| `NYKS_RPC_BASE_URL`        | Nyks chain Tendermint RPC endpoint                                    | `https://rpc.twilight.org`         | `https://rpc.twilight.rest`            |
| `NYKS_LCD_BASE_URL`        | Nyks chain LCD (REST API) endpoint                                    | `https://lcd.twilight.org`         | `https://lcd.twilight.rest`            |
| `FAUCET_BASE_URL`          | Faucet endpoint for requesting test tokens                            | empty string (disabled by default) | `https://faucet-rpc.twilight.rest`     |
| `TWILIGHT_INDEXER_URL`     | Twilight indexer base URL (BTC block height, account queries)         | `https://indexer.twilight.org`     | `https://indexer.twilight.rest`        |
| `BTC_ESPLORA_PRIMARY_URL`  | Primary Esplora API for BTC balance, fee estimation, and broadcasting | `https://blockstream.info/api`     | `https://blockstream.info/testnet/api` |
| `BTC_ESPLORA_FALLBACK_URL` | Fallback Esplora API (used when primary is unreachable)               | `https://mempool.space/api`        | `https://mempool.space/testnet/api`    |

### Order-Wallet Endpoints (feature: `order-wallet`)

Required when using trading, lending, or ZkOS account commands. Defaults are selected from `NETWORK_TYPE`.

| Variable                     | Description                                                                      | Default (`mainnet`)             | Default (`testnet`)                    |
| ---------------------------- | -------------------------------------------------------------------------------- | ------------------------------- | -------------------------------------- |
| `RELAYER_API_RPC_SERVER_URL` | Relayer public JSON-RPC API endpoint                                             | `https://api.ephemeral.fi/api`  | `https://relayer.twilight.rest/api`    |
| `ZKOS_SERVER_URL`            | ZkOS server endpoint for zero-knowledge operations (also used by `relayer-init`) | `https://zkserver.twilight.org` | `https://nykschain.twilight.rest/zkos` |
| `RELAYER_PROGRAM_JSON_PATH`  | Path to the relayer program JSON file (ZkOS circuit parameters)                  | `./relayerprogram.json`         | `./relayerprogram.json`                |

### Wallet Configuration

| Variable                 | Description                                                                                                 | Default |
| ------------------------ | ----------------------------------------------------------------------------------------------------------- | ------- |
| `NYKS_WALLET_ID`         | Default wallet ID (fallback when `--wallet-id` is omitted)                                                  | —       |
| `NYKS_WALLET_PASSPHRASE` | Database encryption password (fallback when `--password` is omitted). Leave empty to use interactive prompt | —       |

### Database (feature: `sqlite` or `postgresql`)

Required for persisting wallets, ZkOS accounts, UTXOs, and order history.

| Variable                  | Description                                                       | Default            |
| ------------------------- | ----------------------------------------------------------------- | ------------------ |
| `DATABASE_URL_SQLITE`     | SQLite database file path (feature: `sqlite`, enabled by default) | `./wallet_data.db` |
| `DATABASE_URL_POSTGRESQL` | PostgreSQL connection string (feature: `postgresql`)              | —                  |

## Password and Wallet ID Resolution

Most commands accept `--wallet-id` and `--password` flags. When omitted, the CLI resolves them through a fallback chain:

**Wallet ID:** `--wallet-id` flag → session cache (see `wallet unlock`) → `NYKS_WALLET_ID` env var → error.

**Password:** `--password` flag → session cache (see `wallet unlock`) → `NYKS_WALLET_PASSPHRASE` env var → none.

## Usage

```
relayer-cli [--json] <COMMAND>
```

### Global Flags

| Flag     | Description                                                        |
| -------- | ------------------------------------------------------------------ |
| `--json` | Output results as JSON instead of formatted tables (for scripting) |

### Command Groups

- `wallet` — create, import, load, list, balance, accounts, export, backup, restore, unlock/lock, info, change-password, update-btc-address, sync-nonce, send, register-btc, deposit-btc, reserves, deposit-status, withdraw-btc, withdraw-status, faucet
- `bitcoin-wallet` — balance, transfer, receive, update-bitcoin-wallet, history (on-chain BTC operations)
- `zkaccount` — fund, withdraw, transfer, split
- `order` — open/close/cancel/query trader and lend orders, unlock-close-order, unlock-failed-order, history-trade, history-lend, funding-history, account-summary, tx-hashes, request-history
- `market` — query prices, orderbook, rates, historical data, candles, APY charts
- `history` — view order and transfer history (requires DB)
- `portfolio` — portfolio summary, balances (with unit conversion), liquidation risks
- `verify-test` — run verification tests against testnet (testnet only)
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

| Flag                   | Description                                                                     |
| ---------------------- | ------------------------------------------------------------------------------- |
| `--wallet-id <ID>`     | Wallet ID for DB storage (defaults to the Twilight address)                     |
| `--password <PASS>`    | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`)                 |
| `--btc-address <ADDR>` | BTC Native SegWit address (`bc1q...`) to use instead of generating a random one |

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

| Flag                   | Description                                                                    |
| ---------------------- | ------------------------------------------------------------------------------ |
| `--mnemonic <PHRASE>`  | 24-word BIP-39 mnemonic. If omitted, prompts securely via TTY                  |
| `--wallet-id <ID>`     | Wallet ID for DB storage (defaults to the Twilight address)                    |
| `--password <PASS>`    | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`)                |
| `--btc-address <ADDR>` | BTC Native SegWit address (`bc1q...`) to use instead of deriving from mnemonic |

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

| Flag                | Description                                                     |
| ------------------- | --------------------------------------------------------------- |
| `--wallet-id <ID>`  | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>` | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

### `wallet list`

List all wallets stored in the database. Requires `sqlite` or `postgresql` feature.

```bash
relayer-cli wallet list
relayer-cli wallet list --db-url "sqlite:///path/to/db"
```

| Flag             | Description                       |
| ---------------- | --------------------------------- |
| `--db-url <URL>` | Override the default database URL |

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

Output columns: `INDEX`, `BALANCE`, `ON-CHAIN`, `IO-TYPE`, `TX-TYPE`, `ACCOUNT`. The `TX-TYPE` column shows `ORDERTX` for trade orders, `LENDTX` for lend orders, or `-` when the account is in Coin state.

### `wallet backup`

Export a full database backup (all tables) to a JSON file. Includes ZkOS accounts, encrypted wallet data, order wallet config, UTXO details, request IDs, order history, transfer history, BTC deposits, and BTC withdrawals. Requires `sqlite` or `postgresql` feature.

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

| Flag                | Description                                                     |
| ------------------- | --------------------------------------------------------------- |
| `--wallet-id <ID>`  | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>` | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

Output shows the next sequence number, cached account number, and count of released (reclaimable) sequences.

### `wallet unlock`

Cache wallet ID and password for the current terminal session. Subsequent commands in the same shell will use the cached values automatically, so you don't need to pass `--wallet-id` or `--password` each time.

Both wallet ID and password can be provided via flags, environment variables, or interactive prompt. Before prompting for the wallet ID, the CLI lists all available wallets in the database (similar to `wallet list`).

The cache is scoped to the parent shell process — closing the terminal invalidates it. Only one session can be active at a time; use `--force` to overwrite an existing cached session, or run `wallet lock` first.

**Wallet ID resolution:** `--wallet-id` flag → `NYKS_WALLET_ID` env var → interactive prompt.

**Password resolution:** `--password` flag → `NYKS_WALLET_PASSPHRASE` env var → interactive prompt.

```bash
relayer-cli wallet unlock
# Lists available wallets, prompts for wallet ID and password, then:
relayer-cli wallet balance   # no --wallet-id or --password needed

# Pass wallet ID directly (skip interactive prompt)
relayer-cli wallet unlock --wallet-id my-wallet

# Pass both wallet ID and password (fully non-interactive)
relayer-cli wallet unlock --wallet-id my-wallet --password s3cret

# Overwrite an existing session
relayer-cli wallet unlock --force
```

| Flag                | Description                                                 |
| ------------------- | ----------------------------------------------------------- |
| `--wallet-id <ID>`  | Wallet ID to cache (prompts interactively if omitted)       |
| `--password <PASS>` | Wallet password to cache (prompts interactively if omitted) |
| `--force`           | Overwrite an existing session without error                 |

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

| Flag               | Description                                                    |
| ------------------ | -------------------------------------------------------------- |
| `--wallet-id <ID>` | Wallet to change password for (falls back to `NYKS_WALLET_ID`) |

If a session cache exists, it is updated with the new password automatically.

### `wallet info`

Show wallet info without making any chain calls. Displays address, BTC address, chain ID, account count, and nonce state. Requires DB.

```bash
relayer-cli wallet info
relayer-cli wallet info --wallet-id my-wallet --password s3cret
```

| Flag                | Description            |
| ------------------- | ---------------------- |
| `--wallet-id <ID>`  | Load wallet from DB    |
| `--password <PASS>` | DB encryption password |

### `wallet update-btc-address`

Update the BTC deposit address stored in the wallet. **Only allowed before on-chain registration.** Once a BTC address is registered on-chain (via `wallet register-btc`), it cannot be changed — each Twilight address can only be linked to a single BTC address.

The address must be a native SegWit address (`bc1q...`). Taproot addresses (`bc1p...`) are not supported.

```bash
relayer-cli wallet update-btc-address --btc-address bc1q... --wallet-id my-wallet
```

| Flag                   | Description                                             |
| ---------------------- | ------------------------------------------------------- |
| `--btc-address <ADDR>` | **Required.** New native SegWit BTC address (`bc1q...`) |
| `--wallet-id <ID>`     | Wallet ID (falls back to `NYKS_WALLET_ID`)              |
| `--password <PASS>`    | DB encryption password                                  |

### `wallet send`

Send tokens (nyks or sats) to another Twilight address. Requires DB.

```bash
# Send 1000 nyks (default denom)
relayer-cli wallet send --to twilight1abc... --amount 1000

# Send 500 sats
relayer-cli wallet send --to twilight1abc... --amount 500 --denom sats

relayer-cli wallet send --to twilight1abc... --amount 1000 --wallet-id my-wallet --password s3cret
```

| Flag                | Description                                    |
| ------------------- | ---------------------------------------------- |
| `--to <ADDR>`       | **Required.** Recipient Twilight address       |
| `--amount <N>`      | **Required.** Amount to send                   |
| `--denom <DENOM>`   | Token denomination: `nyks` (default) or `sats` |
| `--wallet-id <ID>`  | Wallet ID (falls back to `NYKS_WALLET_ID`)     |
| `--password <PASS>` | DB encryption password                         |

### `wallet register-btc`

**Mainnet only.** Register the wallet's BTC deposit address on-chain. Before registering, the CLI performs several safety checks:

1. **Checks if the BTC address is already registered** — if it is registered to your wallet, you are told to use `wallet deposit-btc` instead. If registered to another wallet, registration is blocked.
2. **Checks reserve status** — if all reserves are CRITICAL or EXPIRED, registration is blocked with an ETA for the next available reserve.
3. **Estimates transaction fee** — if a BTC wallet is available, the CLI dry-runs the transaction via BDK to get the real fee estimate (falls back to a 2,000 sat buffer if estimation fails).
4. **Checks BTC balance** — confirms the wallet has enough confirmed sats to cover `amount + estimated fee`.
5. **Registers on-chain** and saves a deposit record (including the target reserve ID) to the database.
6. **Auto-pays to the best reserve** — if a BTC wallet is available, automatically sends the deposit amount to the reserve with the latest expiry. If no BTC wallet is available, shows reserve addresses and manual instructions.

```bash
# Register for a 50,000 sat deposit
relayer-cli wallet register-btc --amount 50000

# Custom staking amount (default is 10000)
relayer-cli wallet register-btc --amount 100000 --staking-amount 10000
```

| Flag                   | Description                                            |
| ---------------------- | ------------------------------------------------------ |
| `--amount <SATS>`      | **Required.** Amount in satoshis you intend to deposit |
| `--staking-amount <N>` | Twilight staking amount (default: `10000`)             |
| `--wallet-id <ID>`     | Wallet ID (falls back to `NYKS_WALLET_ID`)             |
| `--password <PASS>`    | DB encryption password                                 |

**Behavior after registration depends on BTC wallet availability:**

- **BTC wallet available** (wallet created/imported from mnemonic): The CLI automatically sends the deposit to the best reserve (latest expiry, non-critical). The deposit is saved with status `sent` and the reserve ID.
- **BTC wallet not available** (wallet created from private key or manual BTC address): The CLI lists ACTIVE/WARNING reserves. If only one active reserve exists, it is pre-selected and saved in the deposit record. Otherwise you must pick one manually:
  ```
  wallet deposit-btc --reserve-address <RESERVE_ADDRESS>
  ```

### `wallet deposit-btc`

**Mainnet only.** Send a BTC deposit to a reserve after your address has been registered on-chain. Requires an amount (in sats, mBTC, or BTC). Verifies that your BTC address is registered, resolves a target reserve, and either auto-sends or provides manual instructions.

```bash
# Deposit with amount in satoshis (auto-selects best reserve)
relayer-cli wallet deposit-btc --amount 50000

# Deposit with a specific reserve address
relayer-cli wallet deposit-btc --amount 50000 --reserve-address bc1q...

# Amount in milli-BTC (1 mBTC = 100,000 sats)
relayer-cli wallet deposit-btc --amount-mbtc 0.5

# Amount in BTC (1 BTC = 100,000,000 sats)
relayer-cli wallet deposit-btc --amount-btc 0.0005
```

| Flag                       | Description                                                               |
| -------------------------- | ------------------------------------------------------------------------- |
| `--amount <SATS>`          | Amount in satoshis (priority 1)                                           |
| `--amount-mbtc <MBTC>`     | Amount in milli-BTC (1 mBTC = 100,000 sats, priority 2)                   |
| `--amount-btc <BTC>`       | Amount in BTC (1 BTC = 100,000,000 sats, priority 3)                      |
| `--reserve-address <ADDR>` | Reserve address to send BTC to. If omitted, auto-selects the best reserve |
| `--wallet-id <ID>`         | Wallet ID (falls back to `NYKS_WALLET_ID`)                                |
| `--password <PASS>`        | DB encryption password                                                    |

At least one amount flag is required. If multiple are provided, priority is: `--amount` > `--amount-mbtc` > `--amount-btc`.

**Pre-checks:**
- Verifies your BTC address is registered on-chain (errors if not — run `wallet register-btc` first)
- If `--reserve-address` is provided, validates it exists and is ACTIVE or WARNING (rejects CRITICAL/EXPIRED)
- If omitted, auto-selects the best reserve (latest expiry, non-critical)

**Behavior depends on BTC wallet availability:**

- **BTC wallet available**: Estimates the real transaction fee via BDK, checks that confirmed balance covers `amount + fee`, then automatically sends BTC to the target reserve. The deposit is saved with status `sent` and the reserve ID.
- **BTC wallet not available**: Saves the deposit with status `pending` and shows the reserve address to send BTC to manually.

**Important:** You **must** send BTC from your registered BTC address only. Sending from any other address will not be credited to your account.

**Full deposit flow:**
1. Run `wallet register-btc --amount <sats>` to register your BTC address on-chain (auto-sends if BTC wallet available)
2. If not auto-sent: run `wallet deposit-btc --amount <sats>` to send to a reserve (auto-sends if BTC wallet available, otherwise manual)
3. Wait for Bitcoin confirmation (~10 min) and then validator confirmation (can take 1+ hours)
4. Check status with `wallet deposit-status`

### `wallet reserves`

Show all BTC reserve addresses on-chain. Fetches the current Bitcoin block height to display real-time expiry status for each reserve. Also shows a QR code for the recommended reserve (latest-expiring active reserve), displayed side-by-side with the info if the terminal is wide enough.

```bash
relayer-cli wallet reserves
relayer-cli wallet reserves --wallet-id my-wallet --password s3cret
```

| Flag                | Description                                                     |
| ------------------- | --------------------------------------------------------------- |
| `--wallet-id <ID>`  | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>` | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

Output columns: `ID`, `ROUND`, `BLOCKS LEFT`, `STATUS`, `RESERVE ADDRESS`.

Also shows:
- Current BTC block height
- Expired reserve ETA (when a new reserve address will become available)
- QR code for the recommended reserve (latest-expiring active reserve)

**Status key:**
- `ACTIVE` — Safe to send BTC (72+ blocks remaining)
- `WARNING` — Less than ~12 hours remaining (4-72 blocks); send only if your BTC tx will confirm quickly
- `CRITICAL` — Less than 4 blocks remaining; do **not** send
- `EXPIRED` — Reserve is sweeping; do **not** send

Reserve addresses rotate every ~144 Bitcoin blocks (~24 hours). The reserve **must still be active** when your BTC transaction confirms on Bitcoin. Never send BTC to an expired reserve — funds may be delayed or require manual recovery by the validator set.

### `wallet deposit-status`

**Mainnet only.** Show deposit and withdrawal status by combining data from two sources:

1. **Twilight indexer** (`TWILIGHT_INDEXER_URL`) — confirmed on-chain transactions
2. **Local database** — pending deposits and withdrawals not yet confirmed on-chain

The command also auto-updates local DB records: if a pending deposit appears as confirmed on the indexer, its local status is updated to `confirmed`.

```bash
relayer-cli wallet deposit-status
relayer-cli wallet deposit-status --wallet-id my-wallet --password s3cret
```

| Flag                | Description                                                     |
| ------------------- | --------------------------------------------------------------- |
| `--wallet-id <ID>`  | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>` | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

Output sections:

1. **Account info** — address, transaction count, first/last seen timestamps
2. **Balances** — current NYKS and SATS balances
3. **Confirmed Deposits** (from indexer) — columns: `ID`, `AMOUNT`, `BTC HEIGHT`, `CONFIRMED`, `VOTES`, `DATE`
4. **Confirmed Withdrawals** (from indexer) — columns: `ID`, `BTC ADDRESS`, `AMOUNT`, `CONFIRMED`, `DATE`
5. **Pending Deposits — local** (from DB, not yet on indexer) — columns: `ID`, `BTC ADDRESS`, `AMOUNT`, `RESERVE ADDRESS`, `STATUS`, `DATE`
6. **Pending Withdrawals — local** (from DB, not yet confirmed) — columns: `ID`, `BTC ADDRESS`, `RESERVE`, `AMOUNT`, `STATUS`, `DATE`

Each section shows totals (confirmed vs pending) and cumulative amounts.

Validator confirmation can take over 1 hour after the BTC transaction confirms on Bitcoin. If a deposit shows as pending:
1. Ensure BTC was sent to an active reserve address
2. The Bitcoin transaction has at least 1 confirmation
3. Wait for validators to detect and confirm the deposit

For pending withdrawals, run `wallet withdraw-status` to check and update confirmation status.

### `wallet withdraw-btc`

**Mainnet only.** Submit a BTC withdrawal request. BTC is always withdrawn to the wallet's registered BTC address (the same `bc1q...` address used for deposits). The BTC address must be registered on-chain before withdrawing.

```bash
relayer-cli wallet withdraw-btc --reserve-id 1 --amount 50000
```

| Flag                | Description                                           |
| ------------------- | ----------------------------------------------------- |
| `--reserve-id <N>`  | **Required.** Reserve pool ID (see `wallet reserves`) |
| `--amount <SATS>`   | **Required.** Amount in satoshis to withdraw          |
| `--wallet-id <ID>`  | Wallet ID (falls back to `NYKS_WALLET_ID`)            |
| `--password <PASS>` | DB encryption password                                |

The withdrawal is submitted on-chain and saved to the local database with status `submitted`. Use `wallet withdraw-status` to check for confirmations.

### `wallet withdraw-status`

**Mainnet only.** Check on-chain confirmation status for all pending BTC withdrawal requests. The command loads all withdrawals from the database, queries the chain for each pending one, and updates confirmed entries in the database automatically.

```bash
relayer-cli wallet withdraw-status
relayer-cli wallet withdraw-status --wallet-id my-wallet --password s3cret
```

| Flag                | Description                                |
| ------------------- | ------------------------------------------ |
| `--wallet-id <ID>`  | Wallet ID (falls back to `NYKS_WALLET_ID`) |
| `--password <PASS>` | DB encryption password                     |

Output columns: `ID`, `BTC ADDRESS`, `RESERVE`, `AMOUNT`, `STATUS`, `DATE`.

The command displays totals for confirmed vs pending withdrawals and cumulative confirmed amounts. Run this periodically after submitting withdrawals to track their progress.

### `wallet faucet`

**Testnet only.** Request test NYKS and SATS tokens from the faucet. Automatically registers a BTC deposit address and mints test satoshis if needed.

```bash
relayer-cli wallet faucet
relayer-cli wallet faucet --wallet-id my-wallet --password s3cret
```

| Flag                | Description                                |
| ------------------- | ------------------------------------------ |
| `--wallet-id <ID>`  | Wallet ID (falls back to `NYKS_WALLET_ID`) |
| `--password <PASS>` | DB encryption password                     |

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

| Flag                   | Description                                                     |
| ---------------------- | --------------------------------------------------------------- |
| `--amount <SATS>`      | Amount in satoshis                                              |
| `--amount-mbtc <MBTC>` | Amount in milli-BTC (1 mBTC = 100,000 sats)                    |
| `--amount-btc <BTC>`   | Amount in BTC (1 BTC = 100,000,000 sats)                       |
| `--wallet-id <ID>`     | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>`    | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

At least one amount flag is required. All values are converted to satoshis before funding.

### `zkaccount withdraw`

Withdraw from a ZkOS trading account back to the on-chain wallet.

```bash
relayer-cli zkaccount withdraw --account-index 0
```

| Flag                  | Description                                                     |
| --------------------- | --------------------------------------------------------------- |
| `--account-index <N>` | **Required.** ZkOS account index to withdraw from               |
| `--wallet-id <ID>`    | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>`   | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

### `zkaccount transfer`

Transfer funds between ZkOS trading accounts (creates a new destination account).

```bash
relayer-cli zkaccount transfer --account-index 0
```

| Flag                  | Description                                                     |
| --------------------- | --------------------------------------------------------------- |
| `--account-index <N>` | **Required.** Source account index                              |
| `--wallet-id <ID>`    | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>`   | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

### `zkaccount split`

Split a ZkOS trading account into multiple new accounts with specified balances.

Balances can be provided in three units — **exactly one** must be used. If multiple are provided, priority is: `--balances` > `--balances-mbtc` > `--balances-btc` (a warning is shown).

```bash
# In satoshis
relayer-cli zkaccount split --account-index 0 --balances "1000,2000,3000"

# In milli-BTC (1 mBTC = 100,000 sats)
relayer-cli zkaccount split --account-index 0 --balances-mbtc "0.01,0.02,0.03"

# In BTC (1 BTC = 100,000,000 sats)
relayer-cli zkaccount split --account-index 0 --balances-btc "0.00001,0.00002,0.00003"
```

| Flag                     | Description                                                                    |
| ------------------------ | ------------------------------------------------------------------------------ |
| `--account-index <N>`    | **Required.** Source account index                                             |
| `--balances <LIST>`      | Comma-separated list of balances in **satoshis**. Priority 1 if multiple given |
| `--balances-mbtc <LIST>` | Comma-separated list of balances in **milli-BTC** (×100,000). Priority 2       |
| `--balances-btc <LIST>`  | Comma-separated list of balances in **BTC** (×100,000,000). Priority 3         |
| `--wallet-id <ID>`       | Wallet ID (falls back to `NYKS_WALLET_ID`)                                     |
| `--password <PASS>`      | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`)                |

At least one balance flag is required. All values are converted to satoshis internally. Zero-value balances are rejected.

---

## Order Commands

Trading and lending order commands. Most order commands require a wallet ID and password (exception: `tx-hashes` queries the relayer directly without a wallet). Resolution priority: `--flag` > session cache (`wallet unlock`) > env var (`NYKS_WALLET_ID` / `NYKS_WALLET_PASSPHRASE`).

### `order open-trade`

Open a leveraged derivative position.

```bash
# Market long at $65000, 10x leverage
relayer-cli order open-trade --account-index 0 --side LONG --entry-price 65000 --leverage 10

# Limit short
relayer-cli order open-trade --account-index 1 --order-type LIMIT --side SHORT --entry-price 70000 --leverage 5
```

| Flag                   | Description                                                     |
| ---------------------- | --------------------------------------------------------------- |
| `--account-index <N>`  | **Required.** ZkOS account index                                |
| `--side <LONG\|SHORT>` | **Required.** Position direction                                |
| `--entry-price <USD>`  | **Required.** Entry price in USD (integer)                      |
| `--leverage <1-50>`    | **Required.** Leverage multiplier                               |
| `--order-type <TYPE>`  | `MARKET` (default) or `LIMIT`                                   |
| `--wallet-id <ID>`     | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>`    | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

Output: Request ID. JSON mode (`--json`): `{"request_id": "..."}`.

### `order close-trade`

Close an open trader position.

```bash
# Market close (immediate, at current price)
relayer-cli order close-trade --account-index 0

# Limit close (sets a settle_limit trigger at execution-price)
relayer-cli order close-trade --account-index 0 --order-type LIMIT --execution-price 70000

# SLTP close with stop-loss and take-profit (uses SLTP order type automatically)
# Can be used multiple times to update stop-loss and take-profit values
relayer-cli order close-trade --account-index 0 --stop-loss 60000 --take-profit 75000

# SLTP close with stop-loss only
relayer-cli order close-trade --account-index 0 --stop-loss 60000
```

| Flag                    | Description                                                     |
| ----------------------- | --------------------------------------------------------------- |
| `--account-index <N>`   | **Required.** ZkOS account index                                |
| `--order-type <TYPE>`   | `MARKET` (default) or `LIMIT`                                   |
| `--execution-price <F>` | Execution price (default: `0.0` for market)                     |
| `--stop-loss <F>`       | Stop-loss price (triggers SLTP close)                           |
| `--take-profit <F>`     | Take-profit price (triggers SLTP close)                         |
| `--wallet-id <ID>`      | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>`     | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

> **Note:** `--order-type LIMIT` cannot be combined with `--stop-loss` or `--take-profit`. LIMIT sets a single settle_limit trigger at `--execution-price`, while `--stop-loss` / `--take-profit` use the SLTP close path. Use one or the other.

Output: Request ID. JSON mode (`--json`): `{"request_id": "..."}`.

### `order cancel-trade`

Cancel a pending trader order, remove settle limit for filled trader order or cancel stop-loss/take-profit on a filled order.

```bash
# Cancel a pending order or remove settle/close limit for filled order
relayer-cli order cancel-trade --account-index 0

# Cancel stop-loss on a filled order
relayer-cli order cancel-trade --account-index 0 --stop-loss

# Cancel take-profit on a filled order
relayer-cli order cancel-trade --account-index 0 --take-profit

# Cancel both stop-loss and take-profit
relayer-cli order cancel-trade --account-index 0 --stop-loss --take-profit
```

| Flag                  | Description                                                     |
| --------------------- | --------------------------------------------------------------- |
| `--account-index <N>` | **Required.** ZkOS account index                                |
| `--stop-loss`         | Cancel stop-loss trigger (enables SLTP cancel)                  |
| `--take-profit`       | Cancel take-profit trigger (enables SLTP cancel)                |
| `--wallet-id <ID>`    | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>`   | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

Output: Request ID. JSON mode (`--json`): `{"request_id": "..."}`.

### `order query-trade`

Query the status of a trader order. Outputs JSON (uses v1 endpoint with settle_limit, stop_loss, take_profit, and funding_applied).

```bash
relayer-cli order query-trade --account-index 0
relayer-cli --json order query-trade --account-index 0
```

| Flag                  | Description                                                     |
| --------------------- | --------------------------------------------------------------- |
| `--account-index <N>` | **Required.** ZkOS account index                                |
| `--wallet-id <ID>`    | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>`   | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

Output: Pretty-printed JSON of the `TraderOrderV1` object (includes order details, `settle_limit`, `stop_loss`, `take_profit`, `funding_applied`).

### `order unlock-close-order`

Unlock a settled order (trade or lend). Automatically detects the order type from the account's `TX-TYPE` field (`ORDERTX` or `LENDTX`) and calls the appropriate unlock method. Use this to reclaim a ZkOS account after an order has been settled by the relayer. Returns an error if the order is not yet settled or liquidated.

```bash
relayer-cli order unlock-close-order --account-index 0
relayer-cli order unlock-close-order --account-index 0 --wallet-id my-wallet --password s3cret
```

| Flag                  | Description                      |
| --------------------- | -------------------------------- |
| `--account-index <N>` | **Required.** ZkOS account index |
| `--wallet-id <ID>`    | Load wallet from DB              |
| `--password <PASS>`   | DB encryption password           |

Output: Account index, order status, and request ID. JSON mode (`--json`): `{"account_index": N, "order_status": "...", "request_id": "..."}`.

### `order unlock-failed-order`

Unlock a failed order. Use this to reclaim a ZkOS account when an order submission failed and the account is stuck in Memo state. Restores the account back to Coin state by fetching the current UTXO.

```bash
relayer-cli order unlock-failed-order --account-index 0
relayer-cli order unlock-failed-order --account-index 0 --wallet-id my-wallet --password s3cret
```

| Flag                  | Description                      |
| --------------------- | -------------------------------- |
| `--account-index <N>` | **Required.** ZkOS account index |
| `--wallet-id <ID>`    | Load wallet from DB              |
| `--password <PASS>`   | DB encryption password           |

Output: Account index and status. JSON mode (`--json`): `{"account_index": N, "status": "unlocked"}`.

### `order open-lend`

Open a lending order on a ZkOS account. The full account balance is deposited into the lending pool.

```bash
relayer-cli order open-lend --account-index 0
relayer-cli order open-lend --account-index 0 --wallet-id my-wallet --password s3cret
```

| Flag                  | Description                                                     |
| --------------------- | --------------------------------------------------------------- |
| `--account-index <N>` | **Required.** ZkOS account index                                |
| `--wallet-id <ID>`    | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>`   | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

Output: Request ID. JSON mode (`--json`): `{"request_id": "..."}`.

### `order close-lend`

Close an active lending order.

```bash
relayer-cli order close-lend --account-index 0
relayer-cli order close-lend --account-index 0 --wallet-id my-wallet --password s3cret
```

| Flag                  | Description                                                     |
| --------------------- | --------------------------------------------------------------- |
| `--account-index <N>` | **Required.** ZkOS account index                                |
| `--wallet-id <ID>`    | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>`   | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

Output: Request ID. JSON mode (`--json`): `{"request_id": "..."}`.

### `order query-lend`

Query the status of a lending order. Outputs JSON (uses v1 endpoint).

```bash
relayer-cli order query-lend --account-index 0
relayer-cli --json order query-lend --account-index 0
```

| Flag                  | Description                                                     |
| --------------------- | --------------------------------------------------------------- |
| `--account-index <N>` | **Required.** ZkOS account index                                |
| `--wallet-id <ID>`    | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>`   | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

Output: Pretty-printed JSON of the lend order v1 object.

### `order history-trade`

Query historical trader orders for an account from the relayer (not local DB). Requires wallet access for signing.

```bash
relayer-cli order history-trade --account-index 0
relayer-cli --json order history-trade --account-index 0
```

| Flag                  | Description                                                     |
| --------------------- | --------------------------------------------------------------- |
| `--account-index <N>` | **Required.** ZkOS account index                                |
| `--wallet-id <ID>`    | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>`   | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

Output columns: `UUID`, `STATUS`, `TYPE`, `SIDE`, `ENTRY`, `SIZE`, `LEV`, `MARGIN`, `PnL`.

### `order history-lend`

Query historical lend orders for an account from the relayer (not local DB). Requires wallet access for signing.

```bash
relayer-cli order history-lend --account-index 0
relayer-cli --json order history-lend --account-index 0
```

| Flag                  | Description                                                     |
| --------------------- | --------------------------------------------------------------- |
| `--account-index <N>` | **Required.** ZkOS account index                                |
| `--wallet-id <ID>`    | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>`   | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

Output columns: `UUID`, `STATUS`, `DEPOSIT`, `BALANCE`, `SHARES`, `PAYMENT`.

### `order funding-history`

Query funding payment history for a position. Shows each funding interval's payment, rate, and order ID.

```bash
relayer-cli order funding-history --account-index 0
relayer-cli --json order funding-history --account-index 0
```

| Flag                  | Description                                                     |
| --------------------- | --------------------------------------------------------------- |
| `--account-index <N>` | **Required.** ZkOS account index                                |
| `--wallet-id <ID>`    | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>`   | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

Output columns: `TIME`, `SIDE`, `PAYMENT`, `RATE`, `ORDER ID`, plus total funding sum.

### `order account-summary`

Query your wallet's trading activity summary from the relayer — filled, settled, and liquidated counts and position sizes.

```bash
relayer-cli order account-summary
relayer-cli order account-summary --from 2024-01-01 --to 2024-12-31
relayer-cli --json order account-summary
```

| Flag                | Description                                                     |
| ------------------- | --------------------------------------------------------------- |
| `--from <DATE>`     | Start date filter (RFC3339 or YYYY-MM-DD)                       |
| `--to <DATE>`       | End date filter (RFC3339 or YYYY-MM-DD)                         |
| `--since <DATE>`    | Alternative date filter (RFC3339 or YYYY-MM-DD)                 |
| `--wallet-id <ID>`  | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>` | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

Output: Period, filled/settled/liquidated counts and position sizes.

### `order tx-hashes`

Look up on-chain transaction hashes by request ID, account address, or tx ID.
Results are grouped by order ID, with the account address displayed above the table.

```bash
# By request ID (default)
relayer-cli order tx-hashes --id REQID9804F25B...

# By account address
relayer-cli order tx-hashes --by account --id <account_address>

# By tx ID
relayer-cli order tx-hashes --by tx --id <tx_id>

# Filter by status
relayer-cli order tx-hashes --id REQID... --status FILLED

# Include reason column
relayer-cli order tx-hashes --by account --id <account_address> --reason
```

| Flag           | Description                                             |
| -------------- | ------------------------------------------------------- |
| `--by <MODE>`  | Lookup mode: `request` (default), `account`, or `tx`    |
| `--id <ID>`    | **Required.** The ID to look up                         |
| `--status <S>` | Filter by order status (PENDING, FILLED, SETTLED, etc.) |
| `--limit <N>`  | Max results                                             |
| `--offset <N>` | Pagination offset                                       |
| `--reason`     | Show reason column after TX HASH                        |

Output: Account and Order ID displayed as headers. Table columns: `STATUS`, `TYPE`, `DATE`, `OLD PRICE`, `NEW PRICE`, `TX HASH` (and `REASON` if `--reason` is passed).

### `order request-history`

Look up transaction hashes for a wallet account by account index. Resolves the account address from the wallet automatically.

```bash
# Basic usage
relayer-cli order request-history --account-index 0

# With wallet credentials
relayer-cli order request-history --account-index 0 --wallet-id my-wallet --password s3cret

# Filter by status and paginate
relayer-cli order request-history --account-index 0 --status FILLED --limit 20

# Include reason column
relayer-cli order request-history --account-index 0 --reason
```

| Flag                    | Description                                             |
| ----------------------- | ------------------------------------------------------- |
| `--account-index <N>`   | **Required.** Account index to look up                  |
| `--wallet-id <ID>`      | Wallet ID (falls back to `NYKS_WALLET_ID`)              |
| `--password <PASS>`     | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |
| `--status <S>`          | Filter by order status (PENDING, FILLED, SETTLED, etc.) |
| `--limit <N>`           | Max results                                             |
| `--offset <N>`          | Pagination offset                                       |
| `--reason`              | Show reason column after TX HASH                        |

Output: Same format as `tx-hashes` — account and order ID as headers, results grouped by order ID. Table columns: `STATUS`, `TYPE`, `DATE`, `OLD PRICE`, `NEW PRICE`, `TX HASH` (and `REASON` if `--reason` is passed).

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

| Flag                | Description                                                     |
| ------------------- | --------------------------------------------------------------- |
| `--wallet-id <ID>`  | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>` | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

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

| Flag                | Description                                                     |
| ------------------- | --------------------------------------------------------------- |
| `--unit <U>`        | Display unit: `sats` (default), `mbtc`, `btc`                   |
| `--wallet-id <ID>`  | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>` | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

Output columns: `INDEX`, `BALANCE`, `IO-TYPE`, `ON-CHAIN`, plus a total.

### `portfolio risks`

Show liquidation risk for all open trader positions. Queries live price data from the relayer.

```bash
relayer-cli portfolio risks
relayer-cli portfolio risks --wallet-id my-wallet --password s3cret
```

| Flag                | Description                                                     |
| ------------------- | --------------------------------------------------------------- |
| `--wallet-id <ID>`  | Wallet ID (falls back to `NYKS_WALLET_ID`)                      |
| `--password <PASS>` | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`) |

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

Get comprehensive market risk statistics including pool equity, exposure, utilization, risk parameters, funding rate and expected funding rate.

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

| Flag            | Description                                      |
| --------------- | ------------------------------------------------ |
| `--from <DATE>` | **Required.** Start date (RFC3339 or YYYY-MM-DD) |
| `--to <DATE>`   | **Required.** End date (RFC3339 or YYYY-MM-DD)   |
| `--limit <N>`   | Max results (default: `50`)                      |
| `--offset <N>`  | Pagination offset (default: `0`)                 |

### `market candles`

Query OHLCV candlestick data.

```bash
relayer-cli market candles --since 2024-01-01 --interval 1h
relayer-cli market candles --since 2024-01-01 --interval 1d --limit 30
relayer-cli --json market candles --since 2024-01-01 --interval 5m
```

| Flag             | Description                                                                        |
| ---------------- | ---------------------------------------------------------------------------------- |
| `--since <DATE>` | **Required.** Start date (RFC3339 or YYYY-MM-DD)                                   |
| `--interval <I>` | Candle interval: `1m`, `5m`, `15m`, `30m`, `1h` (default), `4h`, `8h`, `12h`, `1d` |
| `--limit <N>`    | Max results (default: `50`)                                                        |
| `--offset <N>`   | Pagination offset (default: `0`)                                                   |

Output columns: `START`, `OPEN`, `HIGH`, `LOW`, `CLOSE`, `VOLUME`, `TRADES`.

### `market history-funding`

Query historical funding rates over a date range.

```bash
relayer-cli market history-funding --from 2024-01-01 --to 2024-01-31
relayer-cli --json market history-funding --from 2024-01-01 --to 2024-01-31
```

| Flag            | Description                                      |
| --------------- | ------------------------------------------------ |
| `--from <DATE>` | **Required.** Start date (RFC3339 or YYYY-MM-DD) |
| `--to <DATE>`   | **Required.** End date (RFC3339 or YYYY-MM-DD)   |
| `--limit <N>`   | Max results (default: `50`)                      |
| `--offset <N>`  | Pagination offset (default: `0`)                 |

### `market history-fees`

Query historical fee rates over a date range.

```bash
relayer-cli market history-fees --from 2024-01-01 --to 2024-01-31
relayer-cli --json market history-fees --from 2024-01-01 --to 2024-01-31
```

| Flag            | Description                                      |
| --------------- | ------------------------------------------------ |
| `--from <DATE>` | **Required.** Start date (RFC3339 or YYYY-MM-DD) |
| `--to <DATE>`   | **Required.** End date (RFC3339 or YYYY-MM-DD)   |
| `--limit <N>`   | Max results (default: `50`)                      |
| `--offset <N>`  | Pagination offset (default: `0`)                 |

Output columns: `MKT FILL`, `LMT FILL`, `MKT SETTLE`, `LMT SETTLE`, `TIMESTAMP`.

### `market apy-chart`

Query APY chart data for the lend pool over time.

```bash
relayer-cli market apy-chart
relayer-cli market apy-chart --range 30d --step 1d
relayer-cli --json market apy-chart --range 7d
```

| Flag             | Description                                        |
| ---------------- | -------------------------------------------------- |
| `--range <R>`    | Time range, e.g. `7d`, `30d`, `1y` (default: `7d`) |
| `--step <S>`     | Step/granularity, e.g. `1h`, `1d`                  |
| `--lookback <L>` | Lookback period for rolling average                |

Output columns: `TIME`, `APY %`.

---

## Bitcoin Wallet Commands

On-chain Bitcoin operations: check balance, transfer BTC, and view receive address. These commands query the Bitcoin network directly via [Blockstream Esplora](https://blockstream.info) (with [mempool.space](https://mempool.space) as fallback).

Network selection follows the `BTC_NETWORK_TYPE` env var (default: `mainnet`).

### `bitcoin-wallet balance`

Check the actual on-chain Bitcoin balance for a BTC address. Displays confirmed, unconfirmed (mempool), and total balances in sats, mBTC, and BTC.

When `--btc-address` is provided, no wallet is loaded — you can check any arbitrary address without credentials.

```bash
# Use wallet's own BTC address (default: sats)
relayer-cli bitcoin-wallet balance
relayer-cli bitcoin-wallet balance --wallet-id my-wallet --password s3cret

# Display in BTC or mBTC
relayer-cli bitcoin-wallet balance --btc
relayer-cli bitcoin-wallet balance --mbtc

# Check any arbitrary BTC address (no wallet required)
relayer-cli bitcoin-wallet balance --btc-address bc1q...

# Testnet
BTC_NETWORK_TYPE=testnet relayer-cli bitcoin-wallet balance --btc-address tb1q...
```

| Flag                   | Description                                                        |
| ---------------------- | ------------------------------------------------------------------ |
| `--wallet-id <ID>`     | Wallet ID (falls back to `NYKS_WALLET_ID` env var)                 |
| `--password <PASS>`    | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`)    |
| `--btc-address <ADDR>` | Query an arbitrary BTC address instead of the wallet's own address |
| `--btc`                | Display balance in BTC                                             |
| `--mbtc`               | Display balance in mBTC                                            |

Default unit is sats. If both `--btc` and `--mbtc` are passed, `--btc` takes priority.

### `bitcoin-wallet transfer`

Transfer BTC to a native SegWit address. Uses the BTC wallet stored inside the Twilight wallet (created when the wallet was generated from a mnemonic). If the wallet was created from a private key instead of a mnemonic, the BTC wallet is not available and the command returns an error.

BDK handles coin selection and fee estimation. If the balance (including fees) is insufficient, a clear error is shown with available vs requested amounts.

Provide exactly one of `--amount` (sats), `--amount-mbtc`, or `--amount-btc`. If multiple are given, priority is: `--amount` > `--amount-mbtc` > `--amount-btc`.

```bash
# In satoshis
relayer-cli bitcoin-wallet transfer --to bc1q... --amount 50000

# In milli-BTC (1 mBTC = 100,000 sats)
relayer-cli bitcoin-wallet transfer --to bc1q... --amount-mbtc 0.5

# In BTC (1 BTC = 100,000,000 sats)
relayer-cli bitcoin-wallet transfer --to bc1q... --amount-btc 0.0005

# Custom fee rate (higher = faster confirmation)
relayer-cli bitcoin-wallet transfer --to bc1q... --amount 50000 --fee-rate 5

relayer-cli bitcoin-wallet transfer --to bc1q... --amount 50000 --wallet-id my-wallet --password s3cret

# Testnet
BTC_NETWORK_TYPE=testnet relayer-cli bitcoin-wallet transfer --to tb1q... --amount 10000
```

| Flag                   | Description                                                       |
| ---------------------- | ----------------------------------------------------------------- |
| `--to <ADDR>`          | **Required.** Destination BTC address (bc1q.../tb1q...)           |
| `--amount <SATS>`      | Amount in satoshis (priority 1)                                   |
| `--amount-mbtc <MBTC>` | Amount in milli-BTC (1 mBTC = 100,000 sats, priority 2)           |
| `--amount-btc <BTC>`   | Amount in BTC (1 BTC = 100,000,000 sats, priority 3)              |
| `--fee-rate <SAT/VB>`  | Fee rate in sat/vB — higher = faster confirmation (default: auto) |
| `--wallet-id <ID>`     | Wallet ID (falls back to `NYKS_WALLET_ID` env var)                |
| `--password <PASS>`    | DB encryption password (falls back to `NYKS_WALLET_PASSPHRASE`)   |

At least one amount flag is required. All values are converted to satoshis before sending.

On success, displays the transaction ID, fee paid, and a block explorer link.

### `bitcoin-wallet receive`

Show the wallet's BTC receive address with network and address type details, along with a scannable QR code. The QR code is displayed side-by-side with the text info when the terminal is wide enough, otherwise below.

```bash
relayer-cli bitcoin-wallet receive
relayer-cli bitcoin-wallet receive --wallet-id my-wallet --password s3cret
```

| Flag                | Description                                        |
| ------------------- | -------------------------------------------------- |
| `--wallet-id <ID>`  | Wallet ID (falls back to `NYKS_WALLET_ID` env var) |
| `--password <PASS>` | DB encryption password                             |

Output includes:
- BTC address with QR code
- Network (mainnet/testnet)
- Address type (Native SegWit P2WPKH / Taproot P2TR)
- On-chain registration status
- BTC wallet availability and derivation path

### `bitcoin-wallet update-bitcoin-wallet`

Update the BTC wallet by providing a new mnemonic phrase. Re-derives BIP-84 keys and updates the wallet's BTC address and keys. Requires `sqlite` or `postgresql` feature.

```bash
# Prompt for mnemonic securely (recommended)
relayer-cli bitcoin-wallet update-bitcoin-wallet

# Pass mnemonic on command line
relayer-cli bitcoin-wallet update-bitcoin-wallet --mnemonic "word1 word2 ... word24"
```

| Flag                  | Description                                                   |
| --------------------- | ------------------------------------------------------------- |
| `--wallet-id <ID>`    | Wallet ID (falls back to `NYKS_WALLET_ID` env var)            |
| `--password <PASS>`   | DB encryption password                                        |
| `--mnemonic <PHRASE>` | 24-word BIP-39 mnemonic. If omitted, prompts securely via TTY |

**Pre-checks:**
- Mnemonic must not be empty
- Current BTC address must **not** be registered on-chain (cannot change if registered)
- New BTC address derived from the mnemonic must not be linked to a different twilight address
- If the new address is already registered to your own twilight address, `btc_address_registered` is set to `true` automatically

### `bitcoin-wallet history`

Show BTC transfer history with confirmation status. Requires `sqlite` or `postgresql` feature.

Transfers are saved automatically when using `bitcoin-wallet transfer`. Each record tracks the from/to addresses, amount, fee, transaction ID, confirmation count, and status.

```bash
relayer-cli bitcoin-wallet history
relayer-cli bitcoin-wallet history --wallet-id my-wallet --password s3cret

# Filter by status
relayer-cli bitcoin-wallet history --status broadcast
relayer-cli bitcoin-wallet history --status confirmed

# Limit results
relayer-cli bitcoin-wallet history --limit 10
```

| Flag                | Description                                           |
| ------------------- | ----------------------------------------------------- |
| `--wallet-id <ID>`  | Wallet ID (falls back to `NYKS_WALLET_ID` env var)    |
| `--password <PASS>` | DB encryption password                                |
| `--status <STATUS>` | Filter by status: `pending`, `broadcast`, `confirmed` |
| `--limit <N>`       | Max results (default: `50`)                           |

Output columns: `ID`, `STATUS`, `FROM`, `TO`, `AMOUNT`, `FEE`, `CONFIRMS`, `NET`, `DATE`.

Shows totals for confirmed vs pending transfers and cumulative amounts sent and fees paid.

---

## Verify-Test Commands

**Testnet only.** Run verification tests against the testnet to validate that CLI commands are working correctly. Blocked on mainnet (`NETWORK_TYPE` must be `testnet`).

Each subcommand creates temporary test wallets, exercises the relevant commands, and reports PASS/FAIL/SKIP results with a final summary.

To run on testnet in a single command (without modifying `.env`):

```bash
NETWORK_TYPE=testnet relayer-cli verify-test all
```

### `verify-test wallet`

Verify all wallet subcommands: create, info, faucet, balance, export, accounts, reserves, send (to self), backup, and sync-nonce.

```bash
relayer-cli verify-test wallet
```

### `verify-test market`

Verify market data queries: price, orderbook, funding-rate, lend-pool-info, and server-time.

```bash
relayer-cli verify-test market
```

### `verify-test zkaccount`

Verify ZkOS account commands: fund, query, and withdraw. Requires a funded wallet (uses faucet automatically).

```bash
relayer-cli verify-test zkaccount
```

### `verify-test order`

Verify order commands: open-trade (MARKET LONG), query-trade, and close-trade. Requires a funded ZkOS account (set up automatically).

```bash
relayer-cli verify-test order
```

### `verify-test all`

Run all verification tests in sequence: wallet → market → zkaccount → order.

```bash
relayer-cli verify-test all
```

---

## Logging

Set the `RUST_LOG` environment variable to control log output:

```bash
RUST_LOG=info relayer-cli wallet balance
RUST_LOG=debug relayer-cli order fund --amount 50000
```
