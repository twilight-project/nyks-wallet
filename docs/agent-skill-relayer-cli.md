# Agent Skill: Relayer CLI

A complete guide for AI agents and developers to build, configure, and operate the Twilight `relayer-cli`. This document is self-contained — read it to go from zero to executing trades.

## Contents

1. [Quick Start (pre-built binary)](#1-quick-start-pre-built-binary) — install the released binary in one command
2. [Build from Source](#2-build-from-source) — Rust/protoc prerequisites, SQLite vs PostgreSQL
3. [Configure](#3-configure) — env vars, endpoints, credential resolution, DB
4. [Global CLI pattern](#4-global-cli-pattern) — `--json`, REPL, amount-units convention
5. [Complete Workflow: Testnet](#5-complete-workflow-testnet) — create → faucet → trade → withdraw
6. [Complete Workflow: Mainnet (BTC onboarding)](#6-complete-workflow-mainnet-btc-onboarding) — register/deposit/withdraw, manual flow for no-mnemonic wallets
7. [Account State Model](#7-account-state-model) — ZkAccount fields, `Coin`/`Memo`/`State` semantics
8. [Order Lifecycle](#8-order-lifecycle) — trade/lend flows, close-trade rules, status reference, numeric limits, preconditions
9. [Command Quick Reference](#9-command-quick-reference) — every command in one place, grouped by domain
10. [Multiple Orders via Split](#10-multiple-orders-via-split) — parallel orders from one funded account
11. [Critical Rules](#11-critical-rules) — address reuse, reserve timing, network restrictions
12. [Error Recovery](#12-error-recovery) — symptom → resolution table
13. [JSON Output](#13-json-output) — `--json` shape by command category
14. [Getting Unstuck](#14-getting-unstuck) — inline `help` / `--help` pointers

---

## 1. Quick Start (pre-built binary)

Download the latest pre-built binary from [GitHub releases](https://github.com/twilight-project/nyks-wallet/releases) — no build tools required.

### Install script (macOS / Linux)

```bash
curl -sSfL https://raw.githubusercontent.com/twilight-project/nyks-wallet/main/install.sh | sh
```

This auto-detects your platform, downloads the latest release, and installs `relayer-cli` in the current directory.

### Install script (Windows PowerShell)

```powershell
irm https://raw.githubusercontent.com/twilight-project/nyks-wallet/main/install.ps1 | iex
```

> **Note:** The pre-built binary is compiled with the **default feature set (SQLite backend)**. If you need the PostgreSQL backend, build from source — see [Section 2](#2-build-from-source).

### Verify

```bash
./relayer-cli --help
```

### Updating

Once installed, the CLI can update itself:

```bash
relayer-cli update            # download and install the latest version
relayer-cli update --check    # check if an update is available
```

---

## 2. Build from Source

### Prerequisites

| Dependency | macOS | Debian/Ubuntu |
|---|---|---|
| Rust (edition 2024) | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` | same |
| protoc | `brew install protobuf` | `sudo apt-get install -y protobuf-compiler` |
| OpenSSL + pkg-config | `brew install openssl pkg-config` | `sudo apt-get install -y pkg-config libssl-dev` |
| libpq (PostgreSQL only) | `brew install libpq` | `sudo apt-get install -y libpq-dev` |

### Compile

```bash
# Default (SQLite backend, bundled — no system SQLite dependency)
cargo build --release --bin relayer-cli

# PostgreSQL backend instead
cargo build --release --bin relayer-cli --no-default-features --features postgresql
```

Binary: `target/release/relayer-cli`

### Docker

```bash
docker build -t relayer-cli .
docker run -e RUST_LOG=info -e NETWORK_TYPE=testnet relayer-cli <command>
```

---

## 3. Configure

A `.env` file in the working directory is loaded automatically. All defaults are built into the binary — you typically don't need to set anything beyond `RUST_LOG` and (for testnet) `NETWORK_TYPE`.

### Minimal .env

```bash
# Mainnet
RUST_LOG=info

# Testnet
RUST_LOG=info
NETWORK_TYPE=testnet
```

### Core variables

| Variable | Purpose | Default |
|---|---|---|
| `RUST_LOG` | Logging level (`info`, `debug`, `trace`) | — |
| `RUST_BACKTRACE` | Rust backtraces (`1` or `full`) | — |
| `CHAIN_ID` | Blockchain network identifier | `nyks` |
| `NETWORK_TYPE` | `mainnet` or `testnet` — controls nyks chain endpoints and BIP-44 derivation | `mainnet` |
| `BTC_NETWORK_TYPE` | Bitcoin network for BTC key derivation. See note below — leave as `mainnet` for normal use | `mainnet` |

> **Important — `BTC_NETWORK_TYPE` does NOT follow `NETWORK_TYPE`.** The Nyks chain only supports **BTC mainnet**, even when running against nyks testnet. All deposit/withdraw flows (`register-btc`, `deposit-btc`, `withdraw-btc`, reserves) operate on BTC mainnet regardless of `NETWORK_TYPE`. Keep `BTC_NETWORK_TYPE=mainnet` (the default) for both nyks mainnet and nyks testnet. Only set `BTC_NETWORK_TYPE=testnet` when **locally testing the `bitcoin-wallet` subcommands** (`balance`, `transfer`, `receive`) against the Bitcoin testnet — never combine it with the register/deposit/withdraw flows.

### Chain endpoints (auto-resolved from `NETWORK_TYPE`)

| Variable | Default (mainnet) | Default (testnet) |
|---|---|---|
| `NYKS_RPC_BASE_URL` | `https://rpc.twilight.org` | `https://rpc.twilight.rest` |
| `NYKS_LCD_BASE_URL` | `https://lcd.twilight.org` | `https://lcd.twilight.rest` |
| `FAUCET_BASE_URL` | disabled | `https://faucet-rpc.twilight.rest` |
| `TWILIGHT_INDEXER_URL` | `https://indexer.twilight.org` | `https://indexer.twilight.rest` |
| `BTC_ESPLORA_PRIMARY_URL` | `https://blockstream.info/api` | `https://blockstream.info/testnet/api` |
| `BTC_ESPLORA_FALLBACK_URL` | `https://mempool.space/api` | `https://mempool.space/testnet/api` |

### Order-wallet endpoints (required for trading/lending/ZkOS)

| Variable | Default (mainnet) | Default (testnet) |
|---|---|---|
| `RELAYER_API_RPC_SERVER_URL` | `https://api.ephemeral.fi/api` | `https://relayer.twilight.rest/api` |
| `ZKOS_SERVER_URL` | `https://zkserver.twilight.org` | `https://nykschain.twilight.rest/zkos` |
| `RELAYER_PROGRAM_JSON_PATH` | `./relayerprogram.json` | `./relayerprogram.json` |

### Wallet credentials

| Variable | Purpose |
|---|---|
| `NYKS_WALLET_ID` | Default wallet ID when `--wallet-id` is omitted |
| `NYKS_WALLET_PASSPHRASE` | Default password when `--password` is omitted |

**Resolution order** (for both wallet-id and password):
1. CLI flag (`--wallet-id`, `--password`)
2. Session cache (set via `wallet unlock`)
3. Env var (`NYKS_WALLET_ID`, `NYKS_WALLET_PASSPHRASE`)
4. Interactive prompt (only for `wallet unlock`; other commands error out)

### Database

| Variable | Purpose | Default |
|---|---|---|
| `DATABASE_URL_SQLITE` | SQLite file path (default feature) | `./wallet_data.db` |
| `DATABASE_URL_POSTGRESQL` | PostgreSQL connection string (requires `--features postgresql` build) | — |

---

## 4. Global CLI pattern

```bash
relayer-cli [--json] <command-group> <subcommand> [flags]
```

`--json` outputs JSON instead of formatted tables. Useful for scripting/parsing.

### Amount units (shared across commands that take a BTC amount)

Commands that accept a BTC amount (`zkaccount fund`, `wallet deposit-btc`, `bitcoin-wallet transfer`) take **exactly one** of these three flags:

| Flag | Unit | Example |
|---|---|---|
| `--amount <sats>` | satoshis (integer) | `--amount 50000` |
| `--amount-mbtc <mBTC>` | milli-BTC (float) | `--amount-mbtc 0.5` |
| `--amount-btc <BTC>` | BTC (float) | `--amount-btc 0.0005` |

Conversion: **1 BTC = 100,000 mBTC = 100,000,000 sats**.

`zkaccount split` uses the plural-list variants: `--balances "1000,2000,3000"` (sats), `--balances-mbtc`, `--balances-btc`.

**Display units (separate from input):**
- `bitcoin-wallet balance` accepts `--btc` or `--mbtc` flags to display the balance in that unit (default: sats).
- `portfolio balances` accepts `--unit sats|mbtc|btc`.

### REPL (Interactive Mode)

For interactive sessions, use the `repl` command. It loads the wallet once and keeps it in memory, so you don't repeat credentials or the `relayer-cli` prefix on every command.

```bash
relayer-cli repl                                    # prompts for wallet ID + password
relayer-cli repl --wallet-id test1                  # prompts for password only
relayer-cli repl --wallet-id test1 --password pass  # fully non-interactive start
```

Inside the REPL, type commands directly:

```
twilight1a...z123> wallet balance
twilight1a...z123> order query-trade --account-index 0
twilight1a...z123> market price --json
```

The REPL supports full line editing (arrow keys, Home/End, Ctrl+A/E) and persistent command history across sessions (`~/.relayer_cli_history`). Additional REPL-only commands: `reload` (re-read wallet from DB), `clear`, `help [group]`, `exit`/`quit`.

---

## 5. Complete Workflow: Testnet

```bash
# 1. Create wallet
relayer-cli wallet create --wallet-id test1 --password pass123

# 2. Get testnet tokens
relayer-cli wallet faucet

# 3. Check balance
relayer-cli wallet balance

# 4. Fund a ZkOS trading account (50,000 sats)
relayer-cli zkaccount fund --amount 50000

# 5. Check accounts
relayer-cli wallet accounts

# 6. Open a trade
relayer-cli order open-trade --account-index 0 --side LONG --entry-price 65000 --leverage 5

# 7. Query position
relayer-cli order query-trade --account-index 0

# 8. Close the trade
relayer-cli order close-trade --account-index 0

# 9. Unlock the account (after settlement)
relayer-cli order unlock-close-order --account-index 0

# 10. Transfer before reuse (required — address can't be reused)
relayer-cli zkaccount transfer --account-index 0

# 11. Withdraw back to on-chain wallet
relayer-cli zkaccount withdraw --account-index <new_index>
```

---

## 6. Complete Workflow: Mainnet (BTC onboarding)

```bash
# 1. Import wallet from mnemonic (enables BTC auto-pay)
relayer-cli wallet import

# 2. Check BTC address and QR code
relayer-cli bitcoin-wallet receive

# 3. Send BTC to your address from an external wallet, then verify
relayer-cli bitcoin-wallet balance

# 4. Register BTC address + auto-deposit to best reserve
relayer-cli wallet register-btc --amount 50000

# 5. Wait for confirmation (can take 1+ hours)
relayer-cli wallet deposit-status

# 6. Check Twilight balance
relayer-cli wallet balance

# 7. Fund ZkOS account and trade (same as testnet steps 4-10)
relayer-cli zkaccount fund --amount 50000
relayer-cli order open-trade --account-index 0 --side LONG --entry-price 65000 --leverage 5

# 8. Withdraw BTC back to Bitcoin network
relayer-cli zkaccount withdraw --account-index 0
relayer-cli wallet withdraw-btc --reserve-id 1 --amount 50000
relayer-cli wallet withdraw-status
```

### Manual BTC deposit (no mnemonic, e.g. hardware wallet / external custody)

If you don't want the CLI to hold Bitcoin signing keys, skip `wallet import` and instead set an external BTC address you control. The CLI will **not** auto-send — it only records the deposit intent and shows you the reserve address to pay from your external wallet.

```bash
# 1. Create wallet (no mnemonic, no BTC signing keys)
relayer-cli wallet create --wallet-id main --password <pwd>

# 2. Point the wallet at a BTC address you control elsewhere
relayer-cli wallet update-btc-address --btc-address bc1q<your_external_address>

# 3. Register + record a deposit intent. The CLI prints the reserve address to pay.
relayer-cli wallet register-btc --amount 50000

# 4. Send 50,000 sats from your external wallet to that reserve address
#    IMPORTANT: MUST come from bc1q<your_external_address> — other senders are not credited.

# 5. Wait for Bitcoin confirmation (~10 min) + validator confirmation (~1h+)
relayer-cli wallet deposit-status

# 6. Balance appears on Twilight — proceed to trading as in step 7 above
relayer-cli wallet balance
```

For subsequent deposits after registration, use `wallet deposit-btc` (same pattern — prints the reserve address, you send manually).

---

## 7. Account State Model

A **ZkAccount** is a privacy-preserving trading account on the ZkOS layer. Each account holds a balance, a cryptographic commitment (QuisQuis account), and state metadata.

### Full account fields (as shown in `wallet accounts` output and returned by queries)

| Field | Values / Type | Meaning |
|---|---|---|
| `index` | integer | Unique identifier within the wallet (used as `--account-index` everywhere) |
| `balance` | integer (satoshis) | Current balance |
| `qq_address` | hex string | QuisQuis encrypted account = public key + ElGamal commitment. Changes on every transfer (this is why an address can only be used once) |
| `account` | string | Derived account address used for on-chain lookups (`tx-hashes --by account`, `request-history`) |
| `scalar` | hex string | Randomness scalar used in the ElGamal commitment. Secret — part of the wallet DB, never exposed on-chain |
| `io_type` | `Coin` / `Memo` / `State` | Account state on ZkOS (see below) |
| `tx_type` | `ORDERTX` / `LENDTX` / `None` | Type of active order when `io_type = Memo` |
| `on_chain` | `true` / `false` | Whether the account currently exists on-chain (set false after transfer/withdraw) |

### State meanings and transitions

- **`Coin`** — idle UTXO, ready for new orders or transfers.
- **`Memo`** — locked in an active order; `tx_type` indicates which kind (`ORDERTX` = trade, `LENDTX` = lend).
- **`State`** — on-chain initialization state, used briefly during account setup.

After `close-trade` + `unlock-close-order`, the account returns to `Coin` with a new balance (margin ± PnL), but **the address is spent** — must run `zkaccount transfer` before opening a new order from that account.

---

## 8. Order Lifecycle

### Trade order

```
Coin → open-trade → Memo (PENDING)
  ├── fills → FILLED
  │     ├── close-trade (MARKET)     → SETTLED → unlock → Coin
  │     ├── close-trade (LIMIT)      → sets settle_limit → SETTLED → unlock → Coin
  │     ├── close-trade (SLTP)       → sets SL/TP triggers → SETTLED → unlock → Coin
  │     └── liquidation              → LIQUIDATE → unlock → Coin
  ├── cancel-trade (if PENDING)      → CANCELLED → Coin
  └── failed                         → unlock-failed-order → Coin
```

### Cancel-trade behavior

| Scenario | Flags | Precondition | Effect |
|---|---|---|---|
| Cancel pending order | none | Order is `PENDING` | Status → `CANCELLED`, account → `Coin` |
| Remove close limit | none | Order is `FILLED` + has `settle_limit` | Removes settle_limit, position stays open |
| Cancel stop-loss | `--stop-loss` | Order is `FILLED` + has stop_loss trigger | Removes SL trigger, position stays open |
| Cancel take-profit | `--take-profit` | Order is `FILLED` + has take_profit trigger | Removes TP trigger, position stays open |
| Cancel both | `--stop-loss --take-profit` | Order is `FILLED` + has SL and/or TP | Removes both triggers |

### Close-trade rules

| Mode | Flags | Precondition |
|---|---|---|
| MARKET | `--order-type MARKET` (default) | Order must be `FILLED` |
| LIMIT | `--order-type LIMIT --execution-price <P>` | Order must be `FILLED`, price > 0 |
| SLTP | `--stop-loss <P>` and/or `--take-profit <P>` | Order must be `FILLED` |

`--order-type LIMIT` **cannot** be combined with `--stop-loss` or `--take-profit`.

### Close-trade examples

```bash
# MARKET close (default) — immediate close at market price
relayer-cli order close-trade --account-index 0

# LIMIT close — settles when price hits 70000
relayer-cli order close-trade --account-index 0 --order-type LIMIT --execution-price 70000

# SLTP close — sets stop-loss 60000 and take-profit 75000, position stays open until a trigger fires
relayer-cli order close-trade --account-index 0 --stop-loss 60000 --take-profit 75000

# SLTP with only stop-loss (same pattern for take-profit alone)
relayer-cli order close-trade --account-index 0 --stop-loss 60000

# Remove a settle_limit set by a prior LIMIT close (position stays open)
relayer-cli order cancel-trade --account-index 0

# Remove stop-loss only (keeps take-profit, keeps position open)
relayer-cli order cancel-trade --account-index 0 --stop-loss

# Remove both SL and TP
relayer-cli order cancel-trade --account-index 0 --stop-loss --take-profit
```

### Lend order

```
Coin → open-lend → Memo (PENDING) → fills → FILLED
  ├── close-lend → SETTLED → unlock → Coin
  └── failed     → unlock-failed-order → Coin
```

### Lend position metrics (fields returned by `query-lend` / `portfolio summary`)

| Field | Meaning |
|---|---|
| `deposit` | Original amount deposited into the pool (sats) |
| `balance` | Current value of the position = `deposit + accrued yield` |
| `npoolshare` | Fractional share of the total lending pool |
| `payment` | Cumulative interest/yield payment received |
| `apr` | Annualised percentage rate (from v1 API) |

### Order status reference (values in `order_status` field across `query-trade`, `query-lend`, `history-*`)

| Status | When it applies | Next action |
|---|---|---|
| `PENDING` | Order submitted, waiting to be filled | `cancel-trade` (no flags) → `CANCELLED` |
| `FILLED` | Matched/accepted, position is active | `close-trade` (MARKET/LIMIT/SLTP) or `close-lend` |
| `SETTLED` | Position closed and settled | `unlock-close-order` → `Coin` |
| `LIQUIDATE` | Position liquidated (trade only) | `unlock-close-order` → `Coin` |
| `CANCELLED` | Cancelled before filling | Account already back to `Coin` |
| `LENDED` | Internal lend state (rare, usually seen mid-settlement) | Wait for `SETTLED`, then unlock |

### `unlock-close-order` routing

The CLI uses the account's `tx_type` to pick the right unlock path:

| `tx_type` | Action |
|---|---|
| `ORDERTX` | Query trader order, verify `SETTLED`/`LIQUIDATE`, fetch UTXO, restore to `Coin` |
| `LENDTX` | Query lend order, verify `SETTLED`, fetch UTXO, restore to `Coin` |
| `None` | Falls back to trader-order unlock (backward compat for pre-`tx_type` accounts) |

`portfolio summary` auto-runs `unlock-close-order` for any settled/liquidated accounts it discovers, so you rarely need to call it manually if you check portfolio regularly.

### Numeric constraints

| Flag / input | Constraint |
|---|---|
| `order open-trade --leverage` | Integer in **1–50** |
| `order open-trade --entry-price` | Integer USD (u64) — no decimals |
| `order close-trade --execution-price` | Float, must be `> 0` when used with `--order-type LIMIT` |
| `order close-trade --stop-loss` / `--take-profit` | Float, must be `> 0` |
| `zkaccount split` | Creates **at most 8** new accounts per call (tx size limit) |
| `zkaccount split --balances` | All entries must be `> 0`; sum must be `≤` source account balance |
| Amount flags (`--amount`, `--amount-mbtc`, `--amount-btc`) | Provide exactly one; must be `> 0` |

### Precondition cheat-sheet (common "why did this error?" cases)

| Command | Required account state | Required order status | Other |
|---|---|---|---|
| `open-trade` | `Coin` + `on_chain` + **unused address** | — | Market not halted |
| `open-lend` | `Coin` + `on_chain` + **unused address** | — | Market not halted |
| `close-trade MARKET` | `Memo` + `ORDERTX` | `FILLED` | Market not halted |
| `close-trade LIMIT` | `Memo` + `ORDERTX` | `FILLED` | `--execution-price > 0`; no `--stop-loss`/`--take-profit` |
| `close-trade SLTP` | `Memo` + `ORDERTX` | `FILLED` | At least one of `--stop-loss`/`--take-profit`; no `LIMIT` |
| `close-lend` | `Memo` + `LENDTX` | `FILLED` | — |
| `cancel-trade` (no flags) | `Memo` | `PENDING` **or** `FILLED` with active `settle_limit` | — |
| `cancel-trade --stop-loss` / `--take-profit` | `Memo` | `FILLED` with matching trigger | Trigger must exist |
| `unlock-close-order` | `Memo` | `SETTLED` **or** `LIQUIDATE` | Errors otherwise (does not silently skip) |
| `unlock-failed-order` | `Memo` with no active order | — | — |
| `zkaccount transfer` / `withdraw` / `split` | `Coin` + `on_chain` | — | `split` sum ≤ balance, no zero balances |

**"Unused address"** means the account has never been the source of an order. After `close-trade` + `unlock-close-order`, the address is spent — you must `zkaccount transfer` before opening a new order from that account (see [Critical Rule #1](#11-critical-rules)).

---

## 9. Command Quick Reference

### Wallet

| Command | Purpose | Wallet needed? |
|---|---|---|
| `wallet create` | Create new wallet | No |
| `wallet import` | Import from mnemonic | No |
| `wallet load` | Load from DB | No (loads it) |
| `wallet list` | List all wallets | No |
| `wallet balance` | Show NYKS/SATS balance | Yes |
| `wallet accounts` | List ZkOS accounts | Yes |
| `wallet info` | Show wallet info (no chain call) | Yes |
| `wallet export` | Export to JSON | Yes |
| `wallet backup` / `restore` | Full DB backup/restore | Yes |
| `wallet unlock` / `lock` | Session credential caching | — |
| `wallet change-password` | Change DB encryption password | Yes |
| `wallet update-btc-address` | Change BTC address (before registration only) | Yes |
| `wallet sync-nonce` | Sync nonce from chain | Yes |
| `wallet send` | Send NYKS/SATS to another address | Yes |
| `wallet register-btc` | Register BTC for deposit (mainnet) | Yes |
| `wallet deposit-btc` | Send BTC to reserve (mainnet) | Yes |
| `wallet reserves` | Show reserve addresses | Yes |
| `wallet deposit-status` | Check deposit/withdrawal status (mainnet) | Yes |
| `wallet withdraw-btc` | Request BTC withdrawal (mainnet) | Yes |
| `wallet withdraw-status` | Check withdrawal confirmations (mainnet) | Yes |
| `wallet faucet` | Get test tokens (testnet) | Yes |

### ZkOS Accounts

| Command | Purpose |
|---|---|
| `zkaccount fund` | Fund new ZkOS account from on-chain wallet |
| `zkaccount withdraw` | Withdraw ZkOS account back to on-chain wallet |
| `zkaccount transfer` | Transfer to fresh account (required before reuse) |
| `zkaccount split` | Split one account into multiple |

### Orders

| Command | Purpose | Wallet needed? |
|---|---|---|
| `order open-trade` | Open leveraged position | Yes |
| `order close-trade` | Close position (MARKET/LIMIT/SLTP) | Yes |
| `order cancel-trade` | Cancel pending or remove SL/TP triggers | Yes |
| `order query-trade` | Query order status (v1 with SL/TP/funding) | Yes |
| `order unlock-close-order` | Unlock settled order → Coin | Yes |
| `order unlock-failed-order` | Unlock failed order → Coin | Yes |
| `order open-lend` | Open lending position | Yes |
| `order close-lend` | Close lending position | Yes |
| `order query-lend` | Query lend order status | Yes |
| `order history-trade` | Historical trader orders (from relayer) | Yes |
| `order history-lend` | Historical lend orders (from relayer) | Yes |
| `order funding-history` | Funding payment history | Yes |
| `order account-summary` | Trading activity summary | Yes |
| `order tx-hashes` | Look up tx hashes by `--id` + `--by request\|account\|tx` | **No** |
| `order request-history` | Look up tx hashes by account index (resolves address from wallet) | Yes |

### Bitcoin Wallet (on-chain BTC, not ZkOS)

| Command | Purpose | Wallet needed? |
|---|---|---|
| `bitcoin-wallet balance` | On-chain BTC balance (confirmed + unconfirmed); `--btc` / `--mbtc` for units | Yes (or `--btc-address` for arbitrary lookup) |
| `bitcoin-wallet transfer` | Send BTC to a native SegWit address (requires mnemonic-backed BTC wallet) | Yes |
| `bitcoin-wallet receive` | Show BTC receive address, derivation path, QR code, registration status | Yes |
| `bitcoin-wallet history` | BTC transfer history with confirmation status (`--status pending\|broadcast\|confirmed`) | Yes |
| `bitcoin-wallet update-bitcoin-wallet` | Re-derive BTC keys from a new mnemonic (only if BTC address not yet registered) | Yes |

### Market (no wallet needed)

| Command | Purpose |
|---|---|
| `market price` | Current BTC/USD price |
| `market orderbook` | Open limit orders |
| `market funding-rate` | Current funding rate |
| `market fee-rate` | Current fee rates |
| `market recent-trades` | Recent trades |
| `market position-size` | Aggregate long/short sizes |
| `market lend-pool` | Lending pool info |
| `market pool-share-value` | Pool share value |
| `market last-day-apy` | Last 24h APY |
| `market open-interest` | Open interest |
| `market market-stats` | Full market stats + funding rate |
| `market server-time` | Relayer server time |
| `market history-price` | Historical prices (requires `--from`, `--to`) |
| `market candles` | OHLCV candles (requires `--since`) |
| `market history-funding` | Historical funding rates (requires `--from`, `--to`) |
| `market history-fees` | Historical fee rates (requires `--from`, `--to`) |
| `market apy-chart` | APY chart data |

### Portfolio

| Command | Purpose |
|---|---|
| `portfolio summary` | Full portfolio: balances, positions, PnL (auto-unlocks settled) |
| `portfolio balances` | Per-account balance breakdown (`--unit sats\|mbtc\|btc`) |
| `portfolio risks` | Liquidation risk for open positions |

### Update

| Command | Purpose |
|---|---|
| `update` | Check for updates and self-update the binary (`--check` for dry run) |

### History (from local DB)

| Command | Purpose |
|---|---|
| `history orders` | Order history (open/close/cancel events) |
| `history transfers` | Transfer history (fund/withdraw/transfer events) |

### Misc

| Command | Purpose |
|---|---|
| `repl [--wallet-id] [--password]` | Interactive REPL — loads wallet once, then runs commands without the `relayer-cli` prefix. See §4 for details |
| `help [group]` | Inline help — `relayer-cli help` for overview, `relayer-cli help <group>` (e.g. `order`, `wallet`) for group details |
| `verify-test <wallet\|market\|zkaccount\|order\|all>` | **Testnet only** — runs self-tests against live testnet. Developer tool; requires funded wallet for `zkaccount`/`order` suites |

---

## 10. Multiple Orders via Split

You can place multiple orders simultaneously by splitting a funded account. The **split-created accounts** are immediately ready for orders. The **source account** keeps its remaining balance but must be rotated (`zkaccount transfer`) before placing an order from it — however, it can be split again without rotating.

### Example

```bash
# Account 0 has 20,000 sats after funding
relayer-cli zkaccount fund --amount 20000

# Split into two new accounts (remaining 13,000 stays in account 0)
relayer-cli zkaccount split --account-index 0 --balances "5000,2000"
# Creates: account 1 (5,000 sats), account 2 (2,000 sats)
# Account 0 still has 13,000 sats

# Accounts 1 and 2 are ready for orders immediately
relayer-cli order open-trade --account-index 1 --side LONG --entry-price 65000 --leverage 5
relayer-cli order open-lend --account-index 2

# Account 0 can be split further WITHOUT rotating
relayer-cli zkaccount split --account-index 0 --balances "3000,4000"
# Creates: account 3 (3,000 sats), account 4 (4,000 sats)
# Account 0 still has 6,000 sats

# But to place an order FROM account 0, rotate first
relayer-cli zkaccount transfer --account-index 0
# Now the new account (index 5) with 6,000 sats is ready for orders
relayer-cli order open-trade --account-index 5 --side SHORT --entry-price 70000 --leverage 3
```

### Rules

| Account | Can split? | Can place order? |
|---|---|---|
| Split-created accounts (new) | Yes | **Yes** — immediately ready |
| Source account after split | **Yes** — can split again | **No** — must `zkaccount transfer` first |
| Source account after transfer | Yes | **Yes** — fresh address |

---

## 11. Critical Rules

1. **Account address reuse**: A ZkOS account address can only be used for **one order**. After close + unlock, you **must** `zkaccount transfer` before placing a new order.

2. **Reserve timing**: A BTC *reserve* is a validator-controlled Bitcoin address that the protocol uses to custody deposited BTC. Reserves **rotate every ~144 Bitcoin blocks (~24h)** — a new reserve becomes ACTIVE and the old one is swept. You send BTC to the current ACTIVE reserve; validators then credit your Twilight wallet with an equivalent SATS balance. The reserve must **still be ACTIVE when your BTC transaction confirms on Bitcoin** (~10 min + validator confirmation, ~1h+ total). Check `wallet reserves` before sending. Status table:

   | Status | Blocks left | Safe to send? |
   |---|---|---|
   | ACTIVE | > 72 | Yes |
   | WARNING | 5 – 72 | Only if tx will confirm quickly |
   | CRITICAL | ≤ 4 | **No** — tx likely won't confirm in time |
   | EXPIRED | 0 | **No** — reserve is being swept |

   If all reserves are expired/critical, wait for the next rotation; `wallet reserves` shows ETA.

3. **Registered address only**: BTC deposits must come from your registered BTC address. Sending from any other address will not be credited.

4. **Market halt**: Trading commands check if the market is halted before executing. If halted, the command returns an error.

5. **LIMIT vs SLTP conflict**: `--order-type LIMIT` cannot be combined with `--stop-loss` or `--take-profit` on `close-trade`.

6. **Unlock requires settlement**: `unlock-close-order` returns an error if the order is not `SETTLED` or `LIQUIDATE`. It does not silently skip.

7. **BTC wallet availability**: Auto-pay (register-btc, deposit-btc) only works when the wallet was created/imported from a **mnemonic**. Otherwise, manual BTC payment is required.

8. **Network restrictions**: `register-btc`, `deposit-btc`, `deposit-status`, `withdraw-btc`, `withdraw-status` are **mainnet only**. `faucet` is **testnet only**.

---

## 12. Error Recovery

| Situation | Command / Resolution |
|---|---|
| Order submission failed, account stuck in Memo | `order unlock-failed-order --account-index N` |
| Order settled but account still Memo | `order unlock-close-order --account-index N` |
| Want to reuse account after order | `zkaccount transfer --account-index N` |
| Nonce/sequence mismatch | `wallet sync-nonce` |
| "Value Witness Verification Failed" on open-trade | Account address was already used — run `zkaccount transfer` first |
| All reserves expired / critical | Wait for rotation (~144 blocks, ~24h). `wallet reserves` shows ETA |
| `register-btc` says "BTC address is already registered to your wallet" | Already done — use `wallet deposit-btc` instead for subsequent deposits |
| "BTC address is registered to a different twilight address" | Another wallet already owns this BTC address on-chain — use a different BTC address (`wallet update-btc-address`) |
| Deposit stuck as pending for >1h | (1) Confirm BTC tx has 1+ confirmations on mempool explorer, (2) confirm BTC was sent from your *registered* BTC address, (3) confirm the target reserve was still ACTIVE at confirmation time, (4) rerun `wallet deposit-status` |
| `register-btc` / `deposit-btc` says "Failed to estimate fee" | If BTC just arrived, wait for 1 confirmation and retry; otherwise CLI falls back to a 2,000 sat fee buffer |
| `bitcoin-wallet update-bitcoin-wallet` rejected | Current BTC address is already registered on-chain and can't be changed. Either continue using it, or create a fresh wallet |
| "BTC wallet not available" on auto-pay | Wallet was created without a mnemonic — either re-import with a mnemonic, or use the manual deposit flow (§6) |

---

## 13. JSON Output

`--json` is a global flag — it works on every command. Use the shape table below to parse the result.

| Command category | JSON shape |
|---|---|
| Submitters (`open-trade`, `close-trade`, `cancel-trade`, `open-lend`, `close-lend`, `register-btc`, `deposit-btc`, `withdraw-btc`, `send`, `zkaccount fund/withdraw/transfer/split`) | `{"request_id": "..."}` (and/or tx-hash fields) |
| Queries (`query-trade`, `query-lend`, `history-*`, `funding-history`, `account-summary`, `tx-hashes`, `request-history`) | Full object (`TraderOrderV1`, `LendOrderV1`, arrays of records, etc.) |
| Market (`price`, `orderbook`, `funding-rate`, `candles`, …) | Raw value or object from the relayer |
| Unlocks (`unlock-close-order`) | `{"account_index": N, "order_status": "...", "request_id": "..."}` |
| Unlocks (`unlock-failed-order`) | `{"account_index": N, "status": "unlocked"}` |
| Wallet info (`balance`, `info`, `accounts`, `reserves`, `deposit-status`, `withdraw-status`) | Object with the corresponding fields |
| Portfolio (`summary`, `balances`, `risks`) | Full portfolio JSON |

Example:

```bash
relayer-cli --json order open-trade --account-index 0 --side LONG --entry-price 65000 --leverage 5
# → {"request_id": "..."}

relayer-cli --json order query-trade --account-index 0
# → full TraderOrderV1 object with order_status, entry_nonce, settle_limit, stop_loss, take_profit, funding_applied, ...
```

---

## 14. Getting Unstuck

If a command fails and §12 doesn't cover it, use `relayer-cli help <group>` for inline flag/precondition reference — every command group has detailed help built into the binary:

```bash
relayer-cli help wallet
relayer-cli help order
relayer-cli help zkaccount
relayer-cli help market
relayer-cli help bitcoin-wallet
relayer-cli help portfolio
relayer-cli help history
relayer-cli help update
```

Each command also supports `--help` for per-subcommand flag details:

```bash
relayer-cli order close-trade --help
relayer-cli wallet register-btc --help
```
