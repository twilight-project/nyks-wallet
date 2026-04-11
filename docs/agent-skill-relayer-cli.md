# Agent Skill: Relayer CLI

A complete guide for AI agents and developers to build, configure, and operate the Twilight `relayer-cli`.

**Standalone use:** If the agent has **only this file** (no [relayer-cli.md](relayer-cli.md), no source tree), treat this document as the single source of truth. Default URLs, resolution order, and common flags are inlined below. The closing section lists optional repo docs for humans who have the full repository.

**When to use this document:** Install/build, `.env`, credentials, ZkOS account lifecycle, trade and lend orders, market queries, BTC onboarding, split accounts, error recovery, JSON output, and testnet verification.

---

## Table of Contents

1. [Quick Start (pre-built binary)](#1-quick-start-pre-built-binary)
2. [Build from Source](#2-build-from-source)
3. [Configure](#3-configure)
4. [Global CLI Pattern](#4-global-cli-pattern)
5. [Flag Patterns](#5-flag-patterns-agents-without-the-long-manual)
6. [Complete Workflow: Testnet](#6-complete-workflow-testnet)
7. [Complete Workflow: Mainnet (BTC onboarding)](#7-complete-workflow-mainnet-btc-onboarding)
8. [Account State Model](#8-account-state-model)
9. [Order Lifecycle & Server-Side Mechanics](#9-order-lifecycle) — fills, SL/TP, PnL, fees, funding, liquidation, risk gates
10. [Preconditions Compendium](#10-preconditions-compendium)
11. [Command Quick Reference](#11-command-quick-reference)
12. [Multiple Orders via Split](#12-multiple-orders-via-split)
13. [Critical Rules](#13-critical-rules)
14. [Error Recovery](#14-error-recovery)
15. [JSON Output](#15-json-output)
16. [Further Reading](#16-further-reading)

---

## 1. Quick Start (pre-built binary)

Download a pre-built binary from [GitHub releases](https://github.com/twilight-project/nyks-wallet/releases) — no build tools required. Release asset names are `nyks-wallet-macos-arm64`, `nyks-wallet-linux-amd64`, and `nyks-wallet-windows-amd64.exe`; the download URL includes the release tag (example below uses `v0.1.1-relayer-cli` — **always check the releases page for the latest tag** and substitute accordingly).

```bash
# macOS ARM64 (Apple Silicon)
curl -LO https://github.com/twilight-project/nyks-wallet/releases/download/v0.1.1-relayer-cli/nyks-wallet-macos-arm64
mv nyks-wallet-macos-arm64 relayer-cli
chmod +x relayer-cli

# Linux x86_64
curl -LO https://github.com/twilight-project/nyks-wallet/releases/download/v0.1.1-relayer-cli/nyks-wallet-linux-amd64
mv nyks-wallet-linux-amd64 relayer-cli
chmod +x relayer-cli

# Windows x86_64
curl -LO https://github.com/twilight-project/nyks-wallet/releases/download/v0.1.1-relayer-cli/nyks-wallet-windows-amd64.exe
mv nyks-wallet-windows-amd64.exe relayer-cli.exe
```

```bash
./relayer-cli --help
```

---

## 2. Build from Source

### Prerequisites

| Dependency              | macOS                                                             | Debian/Ubuntu                                   |
| ----------------------- | ----------------------------------------------------------------- | ----------------------------------------------- |
| Rust (edition 2024)     | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` | same                                            |
| protoc                  | `brew install protobuf`                                           | `sudo apt-get install -y protobuf-compiler`     |
| OpenSSL + pkg-config    | `brew install openssl pkg-config`                                 | `sudo apt-get install -y pkg-config libssl-dev` |
| libpq (PostgreSQL only) | `brew install libpq`                                              | `sudo apt-get install -y libpq-dev`             |

### Compile

The `relayer-cli` binary is gated on the `order-wallet` feature, which is enabled by default together with SQLite.

```bash
# Default (SQLite backend, bundled — no system SQLite dependency)
cargo build --release --bin relayer-cli

# PostgreSQL backend instead
cargo build --release --bin relayer-cli --no-default-features --features postgresql
```

Binary: `target/release/relayer-cli`

### Docker

The image `ENTRYPOINT` is the `relayer-cli` binary. The first `relayer-cli` below is the **image name**; everything after it is passed to the binary.

```bash
docker build -t relayer-cli .
docker run -e RUST_LOG=info -e NETWORK_TYPE=testnet relayer-cli wallet balance
```

---

## 3. Configure

### Minimal .env for mainnet

```bash
RUST_LOG=info
```

That's it. All defaults (`NETWORK_TYPE=mainnet`, endpoints, database path, relayer program path) are built into the binary. No other env vars are required for a standard install.

If you have the repository, `.env.example` lists every variable; agents without the repo rely on the tables in this section only.

### Minimal .env for testnet

```bash
RUST_LOG=info
NETWORK_TYPE=testnet
```

All testnet endpoints auto-resolve from `NETWORK_TYPE=testnet`. Override only if needed (e.g., local development).

### Key env vars

| Variable                     | Purpose                                                                  | Default                 |
| ---------------------------- | ------------------------------------------------------------------------ | ----------------------- |
| `NETWORK_TYPE`               | `mainnet` or `testnet` — controls endpoints and BIP-44 derivation        | `mainnet`               |
| `BTC_NETWORK_TYPE`           | Bitcoin network for BTC keys/balance — falls back to hardcoded `mainnet` | `mainnet`               |
| `NYKS_WALLET_ID`             | Default wallet ID when `--wallet-id` is omitted                          | —                       |
| `NYKS_WALLET_PASSPHRASE`     | Default password when `--password` is omitted                            | —                       |
| `DATABASE_URL_SQLITE`        | SQLite database path                                                     | `./wallet_data.db`      |
| `DATABASE_URL_POSTGRESQL`    | PostgreSQL connection string (only with `--features postgresql`)         | —                       |
| `RELAYER_API_RPC_SERVER_URL` | Relayer JSON-RPC base URL                                                | Network-dependent       |
| `ZKOS_SERVER_URL`            | ZkOS server URL                                                          | Network-dependent       |
| `RELAYER_PROGRAM_JSON_PATH`  | Path to relayer program JSON (circuit params)                            | `./relayerprogram.json` |
| `RUST_BACKTRACE`             | Set to `1` or `full` for Rust stack traces when debugging                | —                       |
| `CHAIN_ID`                   | Cosmos chain id string                                                   | `nyks`                  |

### Built-in endpoint defaults (override with env)

Values below apply when the corresponding env var is **unset**. They switch with `NETWORK_TYPE`.

**Nyks chain + BTC indexers (on-chain wallet, faucet, BTC):**

| Variable                   | Default if `NETWORK_TYPE=mainnet` | Default if `NETWORK_TYPE=testnet`      |
| -------------------------- | --------------------------------- | -------------------------------------- |
| `NYKS_RPC_BASE_URL`        | `https://rpc.twilight.org`        | `https://rpc.twilight.rest`            |
| `NYKS_LCD_BASE_URL`        | `https://lcd.twilight.org`        | `https://lcd.twilight.rest`            |
| `FAUCET_BASE_URL`          | _(empty — faucet disabled)_       | `https://faucet-rpc.twilight.rest`     |
| `TWILIGHT_INDEXER_URL`     | `https://indexer.twilight.org`    | `https://indexer.twilight.rest`        |
| `BTC_ESPLORA_PRIMARY_URL`  | `https://blockstream.info/api`    | `https://blockstream.info/testnet/api` |
| `BTC_ESPLORA_FALLBACK_URL` | `https://mempool.space/api`       | `https://mempool.space/testnet/api`    |

**Relayer + ZkOS (trading, lending, `zkaccount`, most `order`, `market`, `portfolio`):**

| Variable                     | Default if `NETWORK_TYPE=mainnet` | Default if `NETWORK_TYPE=testnet`      |
| ---------------------------- | --------------------------------- | -------------------------------------- |
| `RELAYER_API_RPC_SERVER_URL` | `https://api.ephemeral.fi/api`    | `https://relayer.twilight.rest/api`    |
| `ZKOS_SERVER_URL`            | `https://zkserver.twilight.org`   | `https://nykschain.twilight.rest/zkos` |

### Working directory matters

- `DATABASE_URL_SQLITE` defaults to `./wallet_data.db` **relative to the process current working directory**.
- `RELAYER_PROGRAM_JSON_PATH` defaults to `./relayerprogram.json` **relative to cwd**. If RELAYER_PROGRAM_JSON_PATH is not provided (or points to a missing file), relayer-cli falls back to the built-in default program JSON and continues working.

### Wallet credential resolution order

Implementation matches `src/bin/relayer_cli/helpers.rs` (`resolve_wallet_id` / `resolve_password`).

**Most commands** (anything that loads the wallet via those helpers):

1. `--wallet-id` / `--password` CLI flags
2. Session cache from `wallet unlock` (see note below)
3. `NYKS_WALLET_ID` / `NYKS_WALLET_PASSPHRASE` env vars

**`wallet unlock` only** (there is no session yet): wallet ID is `--wallet-id` → `NYKS_WALLET_ID` → interactive prompt; password is `--password` → `NYKS_WALLET_PASSPHRASE` → interactive prompt.

**Session cache:** Persisting credentials with `wallet unlock` is supported on **Unix** (macOS, Linux). On Windows, use flags or env vars each time — session caching is not available. Use `wallet unlock --force` to overwrite an existing cached session (e.g., to switch wallets without calling `wallet lock` first).

### Interactive-only commands (automation pitfalls)

- **`wallet import`:** Without `--mnemonic`, the CLI prompts on a TTY. For scripts, pass `--mnemonic "..."` (be aware of shell history) or use a controlled stdin.
- **`wallet unlock`:** Can be fully non-interactive with `--wallet-id` + `--password` or env vars. The CLI **validates credentials** by loading the wallet from DB before caching — if the password is wrong, `unlock` fails. When no wallet ID is provided interactively, it lists available wallets before prompting.
- **`wallet change-password`:** Always prompts for old and new passwords on a secure TTY; **session cache and `NYKS_WALLET_PASSPHRASE` are ignored** so passwords are not changed accidentally. A "confirm new password" prompt ensures no typos. If a session is active, it is automatically updated with the new password.

### Database requirement (release binaries)

Nearly all commands need the wallet database (SQLite by default). Exceptions: **`wallet create`**, **`wallet lock`**, and **`bitcoin-wallet balance`** when **`--btc-address`** is set (query any SegWit address without loading a wallet). Everything else expects a DB at `DATABASE_URL_SQLITE` (or PostgreSQL if built that way).

---

## 4. Global CLI pattern

```bash
relayer-cli [--json] <command-group> <subcommand> [flags]
```

Built-in Clap `help` is disabled; use the `help` command group:

```bash
relayer-cli help                  # global overview
relayer-cli help wallet
relayer-cli help bitcoin-wallet
relayer-cli help zkaccount
relayer-cli help order
relayer-cli help market
relayer-cli help history
relayer-cli help portfolio
relayer-cli help verify-test
```

`--json` outputs JSON instead of formatted tables. Useful for scripting/parsing.

### Agent preflight checklist (run before state-changing commands)

1. Confirm network intent (`NETWORK_TYPE=mainnet` or `testnet`) and command network restrictions (§13 rule 8).
2. Ensure wallet resolution is deterministic: pass `--wallet-id` and `--password` explicitly for automation, or confirm session/env state (§3).
3. For order/zkaccount commands, `RELAYER_PROGRAM_JSON_PATH` is used if present; otherwise the binary falls back to its built-in default circuit params (§13 rule 9). Only verify the file if you need custom parameters.
4. Before opening orders/lends, check market state and limits:
   - `relayer-cli --json market market-stats`
   - ensure status is not `HALT` / `CLOSE_ONLY`
   - ensure leverage and position size are within returned limits
5. Before BTC register/deposit/withdraw, check reserve freshness:
   - `relayer-cli wallet reserves`
   - avoid `CRITICAL` / `EXPIRED` reserves

---

## 5. Flag patterns (agents without the long manual)

### Amounts (exactly one unit per command)

| Area                      | Flags                                                                            | Notes                                              |
| ------------------------- | -------------------------------------------------------------------------------- | -------------------------------------------------- |
| `zkaccount fund`          | `--amount` (sats), `--amount-mbtc`, `--amount-btc`                               | One required                                       |
| `zkaccount split`         | `--balances "a,b,c"` (sats), `--balances-mbtc`, `--balances-btc`                 | Comma-separated                                    |
| `wallet deposit-btc`      | `--amount`, `--amount-mbtc`, `--amount-btc`                                      | One required; optional `--reserve-address`         |
| `wallet send`             | `--to` (twilight addr) + `--amount` + `--denom` (`nyks` or `sats`)               | `--to` and `--amount` required                     |
| `bitcoin-wallet transfer` | `--to` (BTC addr) + `--amount`, `--amount-mbtc`, `--amount-btc`                  | `--to` required; one amount required; `--fee-rate` |
| `wallet register-btc`     | `--amount` (sats) required; `--staking-amount` optional (default **10000** sats) | Mainnet only                                       |

**Amount priority:** If multiple amount flags are given, the CLI warns and uses: `--amount` > `--amount-mbtc` > `--amount-btc` (same for `--balances` variants on `split`).

### Display-unit flags

| Command                  | Flags                                                  | Notes                           |
| ------------------------ | ------------------------------------------------------ | ------------------------------- |
| `bitcoin-wallet balance` | `--btc` (display in BTC), `--mbtc` (display in mBTC)  | Default display is sats         |
| `portfolio balances`     | `--unit sats\|mbtc\|btc`                               | Default `sats`                  |

### Output / export flags

| Command          | Flag                    | Default                |
| ---------------- | ----------------------- | ---------------------- |
| `wallet export`  | `--output <path>`       | `wallet.json`          |
| `wallet backup`  | `--output <path>`       | `wallet_backup.json`   |

### Trade orders

| Action                  | Important flags                                                                                                                                                                                                                                                             |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `order open-trade`      | `--account-index`, `--side` (`LONG` / `SHORT`), `--entry-price` (USD int), `--leverage` (upper bound from relayer **`market market-stats`** `max_leverage`), `--order-type` `MARKET` (default) or `LIMIT`                                                                   |
| `order close-trade`     | `--account-index`; default `--order-type MARKET`. **LIMIT close:** `--order-type LIMIT --execution-price <P>` (required, must be positive). **SLTP:** `--stop-loss` / `--take-profit` (SLTP mode; do **not** combine with LIMIT)                                            |
| `order cancel-trade`    | `--account-index`. **No SL/TP flags:** cancel **PENDING** order **or** remove **close limit** (`settle_limit`) on **FILLED** order. **With `--stop-loss` / `--take-profit`:** remove those triggers on a **FILLED** order (at least one flag required; triggers must exist) |
| `order tx-hashes`       | `--id` **required**; `--by` = `request` (default), `account`, or `tx`; optional `--status`, `--limit`, `--offset`; `--reason` = flag (prints reason column / JSON field)                                                                                                    |
| `order request-history` | `--account-index`; optional `--status`, `--limit`, `--offset`, `--reason` (same boolean flag); optional `--wallet-id`, `--password`                                                                                                                                          |
| `order account-summary` | Optional `--from`, `--to`, `--since` (date filters)                                                                                                                                                                                                                         |

### Market data (required time args)

| Command                                         | Required / optional                                                                                                                                            |
| ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `market history-price`                          | `--from`, `--to` required; `--limit` default **50**, `--offset` default **0**                                                                                  |
| `market candles`                                | `--since` required; `--interval` default `1h` — values: `1m`, `5m`, `15m`, `30m`, `1h`, `4h`, `8h`, `12h`, `1d`; `--limit` / `--offset` defaults same as above |
| `market history-funding`, `market history-fees` | `--from`, `--to` required; `--limit` / `--offset` defaults **50** / **0**                                                                                      |
| `market apy-chart`                              | `--range` default `7d`; optional `--step`, `--lookback`                                                                                                        |

Date strings for these commands and for `order account-summary` (`--from`, `--to`, `--since`): **RFC3339** or **`YYYY-MM-DD`**.

### Wallet list / DB path

- `wallet list` and some other wallet subcommands accept `--db-url` to override `DATABASE_URL_SQLITE` / PostgreSQL URL for that invocation.

### BTC / SegWit

Custom BTC addresses must be **native SegWit**: `bc1q...` / `tb1q...`. Deposits to reserves must be sent from the **registered** wallet BTC address.

### Lending (minimal)

```bash
relayer-cli order open-lend --account-index N    # full account balance → pool; account → Memo / LENDTX
relayer-cli order query-lend --account-index N
relayer-cli order close-lend --account-index N   # after FILLED
relayer-cli order unlock-close-order --account-index N
relayer-cli zkaccount transfer --account-index N # before another lend/trade on same address
```

---

## 6. Complete Workflow: Testnet

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

# 6. Open a trade (--order-type defaults to MARKET; add --order-type LIMIT for a limit entry)
relayer-cli order open-trade --account-index 0 --side LONG --entry-price 65000 --leverage 5
# Limit entry example: --order-type LIMIT --side SHORT --entry-price 70000 --leverage 5

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

## 7. Complete Workflow: Mainnet (BTC onboarding)

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

# 7. Fund ZkOS account and trade (for a full close / unlock / transfer cycle, mirror testnet §6 steps 6–10)
relayer-cli zkaccount fund --amount 50000
relayer-cli order open-trade --account-index 0 --side LONG --entry-price 65000 --leverage 5

# 8. Withdraw ZkOS balance to chain, then BTC to external wallet (use the correct --account-index after any transfer)
relayer-cli zkaccount withdraw --account-index 0
relayer-cli wallet withdraw-btc --reserve-id 1 --amount 50000
relayer-cli wallet withdraw-status
```

---

## 8. Account State Model

Each ZkOS account has three key fields:

| Field      | Values                      | Meaning                         |
| ---------- | --------------------------- | ------------------------------- |
| `io_type`  | `Coin`, `Memo`, `State`     | Account state on ZkOS           |
| `tx_type`  | `ORDERTX`, `LENDTX`, `None` | Type of active order            |
| `on_chain` | `true`, `false`             | Whether account exists on-chain |

**State transitions:**

- `Coin` = idle, ready for orders or transfers
- `Memo` = locked in an active order
- After close + unlock: back to `Coin`, but **must transfer before reuse**

**Implementation note:** `zkaccount withdraw` internally performs a **transfer first** (creates a fresh account), then burns from the new account back to the on-chain wallet. This is why withdrawals take longer than a simple transfer.

---

## 9. Order Lifecycle

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

| Scenario             | Flags                       | Precondition                                | Effect                                    |
| -------------------- | --------------------------- | ------------------------------------------- | ----------------------------------------- |
| Cancel pending order | none                        | Order is `PENDING`                          | Status → `CANCELLED`, account → `Coin`    |
| Remove close limit   | none                        | Order is `FILLED` + has `settle_limit`      | Removes settle_limit, position stays open |
| Cancel stop-loss     | `--stop-loss`               | Order is `FILLED` + has stop_loss trigger   | Removes SL trigger, position stays open   |
| Cancel take-profit   | `--take-profit`             | Order is `FILLED` + has take_profit trigger | Removes TP trigger, position stays open   |
| Cancel both          | `--stop-loss --take-profit` | Order is `FILLED` + has SL and/or TP        | Removes both triggers                     |

### Close-trade rules

| Mode   | Flags                                        | Precondition                      |
| ------ | -------------------------------------------- | --------------------------------- |
| MARKET | `--order-type MARKET` (default)              | Order must be `FILLED`            |
| LIMIT  | `--order-type LIMIT --execution-price <P>`   | Order must be `FILLED`, price > 0 |
| SLTP   | `--stop-loss <P>` and/or `--take-profit <P>` | Order must be `FILLED`            |

`--order-type LIMIT` **cannot** be combined with `--stop-loss` or `--take-profit`.

### Lend order

```
Coin → open-lend → Memo (PENDING) → fills → FILLED
  ├── close-lend → SETTLED → unlock → Coin
  └── failed     → unlock-failed-order → Coin
```

### Limit order fill conditions

A **PENDING** limit order becomes **FILLED** when the market price crosses the entry price:

| Side  | Fills when                              | Intuition                              |
| ----- | --------------------------------------- | -------------------------------------- |
| LONG  | Market price rises **above** limit price | "Buy low" — your bid gets hit          |
| SHORT | Market price drops **below** limit price | "Sell high" — your ask gets hit        |

The relayer checks on **every price tick** (not only on user requests). Once filled, the order is immediately active at `current_price` as entry price.

### SL/TP trigger conditions

Stop-loss and take-profit orders are evaluated server-side on every price tick:

| Side  | Stop-loss triggers when     | Take-profit triggers when    |
| ----- | --------------------------- | ---------------------------- |
| LONG  | `sl >= current_price`       | `tp <= current_price`        |
| SHORT | `sl <= current_price`       | `tp >= current_price`        |

When triggered, the position is **settled at market** using `FilledOnMarket` fees (same as a market close). If not triggered, the SL/TP price is queued and re-checked on the next tick.

### PnL formula (inverse perpetual)

Twilight uses an **inverse perpetual** model — PnL is denominated in sats (BTC), not USD:

- **LONG:** `pnl = position_size × (settle_price − entry_price) / (entry_price × settle_price)`
- **SHORT:** `pnl = position_size × (entry_price − settle_price) / (entry_price × settle_price)`

Because of the inverse formula, equal USD moves produce **different** sat-denominated PnL depending on the price level. `position_size` here is `initial_margin × leverage` in sats.

### Fee formula

Settlement fees on close:

```
fee = (fee_percentage / 100) × position_size / current_price
```

**Minimum floor:** if the computed fee is **< 1 sat**, it is rounded up to **1 sat**.

Four fee types exist on the server — only the first two are charged on settlement:

| Fee type          | Used when                                   |
| ----------------- | ------------------------------------------- |
| `FilledOnMarket`  | Market close, SL/TP trigger, liquidation    |
| `FilledOnLimit`   | Limit close that fills immediately          |
| `SettledOnMarket` | Stored but **not used** in settlement math  |
| `SettledOnLimit`  | Stored but **not used** in settlement math  |

Current fee rates are visible via `market fee-rate`.

### Funding rate

Funding is applied periodically to all FILLED positions, adjusting `available_margin`:

**Global rate formula:**

```
fundingrate = ((total_long − total_short) / all_position_size)² / (ψ × 8)
```

Negative when shorts dominate (ψ defaults to 1.0).

**Per-order impact:**

```
funding_payment = (fundingrate × position_size) / (current_price × 100)
```

- **LONGs pay** when longs dominate (positive rate); **SHORTs receive**
- **SHORTs pay** when shorts dominate (negative rate); **LONGs receive**
- `available_margin` is clamped to **0** — it never goes negative from funding alone, but hitting 0 can trigger liquidation

Current funding rate: `market funding-rate`. History: `market history-funding`.

### Maintenance margin and liquidation

**Maintenance margin formula:**

```
MM = (mm_ratio × entry_value + fee × bankruptcy_value + funding × bankruptcy_value) / 100
```

Default `mm_ratio = 0.4`. The maintenance margin defines the liquidation threshold.

**Liquidation triggers (server-side):**

1. **Price-driven:** on every price tick, the server checks if the market price has crossed the liquidation price (derived from MM)
2. **Funding-driven:** after each funding cycle, if `available_margin <= maintenance_margin`, the position is liquidated

**Liquidation result:** `available_margin` is forced to **0** (total loss of margin). The payment equals `−initial_margin`, meaning the entire margin is absorbed by the lending pool.

### Settlement payment

The full settlement math when a position is closed:

```
margin_difference = available_margin − initial_margin    (captures accumulated funding)
payment = round(unrealized_pnl) + round(margin_difference) − fee
post_settle_available_margin = round(initial_margin + payment)
```

If the position was profitable and fees/funding didn't erode the margin, `payment > 0` and the trader receives more than their initial margin. If `payment < 0`, the difference goes to the lending pool.

### Server-side risk gates

Beyond the CLI's local pre-validation (§10), the relayer server applies additional gates that can reject operations:

| Gate | Effect on operations | How to detect |
| ---- | -------------------- | ------------- |
| **HALT** | Blocks **all** operations (opens, closes, cancels) except lend settlement | `market market-stats` → status `HALT` |
| **CLOSE_ONLY** | Blocks new opens only; closes and cancels proceed | `market market-stats` → status `CLOSE_ONLY` |
| **Price feed paused** (stale > 30s or admin) | Blocks all non-LEND operations | "Risk engine rejected order" with no CLI warning |
| **Kafka unhealthy** (10+ consecutive failures) | Blocks **all** operations | "Risk engine rejected order" — transient, retry later |
| **Pool equity ≤ 0** | Auto-triggers HALT | `market market-stats` → status `HALT`, reason `PoolEquityInvalid` |

**Lend operations** are exempt from the price-feed pause — `open-lend` and `close-lend` can proceed even when trading is blocked by stale prices.

### Default risk parameters

These are the server defaults (operators can override via env vars). Always prefer live values from `market market-stats`:

| Parameter | Default | Meaning |
| --------- | ------- | ------- |
| `max_leverage` | **50.0** | Max leverage (hard 50× ceiling in handler, plus configurable cap) |
| `max_oi_mult` | **4.0** | Max total open interest = 4× pool equity |
| `max_net_mult` | **0.8** | Max net long/short exposure = 0.8× pool equity |
| `max_position_pct` | **0.02** | Max single position = 2% of pool equity |
| `min_position_btc` | **0** | Disabled by default |
| `mm_ratio` | **0.4** | Maintenance margin ratio (for liquidation) |

---

## 10. Preconditions compendium

Condensed from `docs/cli-command-rules.md` and `src/bin/relayer_cli/commands.rs`. When a command errors, match the failure to these gates before guessing flags.

### Wallet lifecycle

| Command / area                                                       | Hard requirements                                                                                                                                                                                              |
| -------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `wallet create`                                                      | Optional `--wallet-id`, `--password`, `--btc-address`. If `--btc-address`: native SegWit only. Prints mnemonic once — user must save it.                                                                       |
| `wallet import`                                                      | Mnemonic non-empty (`--mnemonic` or TTY prompt). Optional `--btc-address` (native SegWit); if omitted, derived from mnemonic. BTC address must not be registered to a **different** Twilight address on-chain. |
| `wallet load`                                                        | **`--wallet-id` required**. Wallet exists; password correct. Optional `--db-url`.                                                                                                                              |
| `wallet backup`                                                      | **`--wallet-id` required**.                                                                                                                                                                                    |
| `wallet restore`                                                     | **`--input`** + **`--wallet-id` required**; use `--force` if backup `wallet_id` ≠ target.                                                                                                                      |
| `wallet register-btc`                                                | **Mainnet only.** Reserves exist; at least one **ACTIVE** (see §13 rule 2). Not already registered to this wallet (else use `deposit-btc`). Balance rules depend on mnemonic-backed `btc_wallet` (§13 rule 7). |
| `wallet deposit-btc`                                                 | **Mainnet only.** BTC address already registered. Exactly **one** amount flag. If `--reserve-address`: that reserve must have **> 4** blocks left; if omitted, best ACTIVE reserve is auto-selected. **Auto-pays** to reserve when mnemonic-backed `btc_wallet` is available; otherwise shows manual instructions. |
| `wallet withdraw-btc`                                                | **Mainnet only.** Registered BTC; reserve exists; **sufficient balance** in that reserve for `--amount`.                                                                                                       |
| `wallet update-btc-address` / `bitcoin-wallet update-bitcoin-wallet` | Current BTC address **not** yet registered on-chain; new address valid SegWit and not tied to another Twilight addr.                                                                                           |

### ZkOS account (`zkaccount`)

| Command                         | Preconditions                                                                       |
| ------------------------------- | ----------------------------------------------------------------------------------- |
| `fund`                          | Wallet loadable; exactly one amount flag; amount **> 0**; enough **on-chain sats**. |
| `withdraw`, `transfer`, `split` | Account exists; **`Coin`** + **`on_chain`**.                                        |
| `split` additionally            | Sum of split amounts ≤ balance; **no zero** entries.                                |

### Trading & lending (`order`)

Assume wallet loadable. **Server-side gates** (§9 "Server-side risk gates" and §13 rules 4–6) apply on top of CLI-side checks: **HALT** blocks closes and cancels too (not just opens); **price feed pause** blocks all non-lend operations; **Kafka unhealthy** blocks everything.

**`open-trade` local pre-validation** (`validate_open_order` in `order_wallet.rs`) — the CLI runs these checks against live `market market-stats` data **before** submitting to the relayer, giving immediate descriptive errors:

1. Market status — reject if **HALT** or **CLOSE_ONLY**
2. Max leverage — reject if leverage exceeds `params.max_leverage`
3. Min position size — reject if entry_value (`margin × leverage`) < `params.min_position_btc`
4. Per-position cap — reject if entry_value > `params.max_position_pct × pool_equity`
5. Directional headroom — reject if entry_value > `max_long_btc` (LONG) or `max_short_btc` (SHORT)

**`open-lend`** checks only step 1 (market halt).

| Command                         | Preconditions                                                                                                                                                                                                                                                                           |
| ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `open-trade`, `open-lend`       | Account **`Coin` + `on_chain`**; **address never used for a prior order** without an intervening `zkaccount transfer`.                                                                                                                                                                  |
| `close-trade`                   | Order **`FILLED`** for a normal close. If order is already **`SETTLED`** or **`LIQUIDATE`**, the implementation **auto-unlocks** instead of a new close request (see cli rules). LIMIT close needs positive `--execution-price`; cannot mix LIMIT with `--stop-loss` / `--take-profit`. |
| `cancel-trade`                  | See §5 table (pending vs settle_limit vs SL/TP flags).                                                                                                                                                                                                                                  |
| `query-trade`                   | Account has an **active trader** order. **Side effect:** if the query fails and the order is not PENDING/FILLED/LIQUIDATE (i.e. it failed), the account is **auto-unlocked** back to `Coin`.                                                                                            |
| `query-lend`                    | Account has an **active lend** order. **Side effect:** if the query fails and the order is not FILLED/SETTLED, the account is **auto-unlocked** back to `Coin`.                                                                                                                         |
| `close-lend`                    | Lend order **`FILLED`**.                                                                                                                                                                                                                                                                |
| `unlock-close-order`            | Order status **`SETTLED`** or **`LIQUIDATE`**; applies to **trade or lend** depending on account `tx_type`.                                                                                                                                                                             |
| `unlock-failed-order`           | Account stuck **`Memo`** with **no active order** (failed submission).                                                                                                                                                                                                                  |
| `history-trade`, `history-lend` | Wallet loadable; relayer query is **signed** (needs keys).                                                                                                                                                                                                                              |

### Relayer-only (no DB wallet)

| Command           | Notes                                                                        |
| ----------------- | ---------------------------------------------------------------------------- |
| `order tx-hashes` | Uses `RELAYER_API_RPC_SERVER_URL` only. `--by` ∈ `request`, `account`, `tx`. |
| All `market *`    | Relayer reachable; **no wallet**.                                            |

### Local DB history

| Command             | Notes                                                                             |
| ------------------- | --------------------------------------------------------------------------------- |
| `history orders`    | `wallet_id` resolved like §3; **`--limit` default 50**, **`--offset` default 0**. |
| `history transfers` | Same defaults.                                                                    |

### Portfolio

| Command   | Notes                                                                             |
| --------- | --------------------------------------------------------------------------------- |
| `summary` | Relayer reachable; **auto-unlocks** settled/liquidated accounts where applicable. |
| `risks`   | Relayer reachable for live prices.                                                |

---

## 11. Command Quick Reference

### Wallet

| Command                     | Purpose                                                  | Wallet needed? |
| --------------------------- | -------------------------------------------------------- | -------------- |
| `wallet create`             | Create new wallet                                        | No             |
| `wallet import`             | Import from mnemonic; optional `--btc-address`           | No             |
| `wallet load`               | Load from DB (`--wallet-id` required)                    | No (loads it)  |
| `wallet list`               | List all wallets                                         | No             |
| `wallet balance`            | Show NYKS/SATS balance                                   | Yes            |
| `wallet accounts`           | List ZkOS accounts; optional `--on-chain-only`           | Yes            |
| `wallet info`               | Show wallet info (no chain call)                         | Yes            |
| `wallet export`             | Export to JSON (`--output`, default `wallet.json`)       | Yes            |
| `wallet backup` / `restore` | Full DB backup/restore (`backup` requires `--wallet-id`) | Yes            |
| `wallet unlock` / `lock`    | Session credential caching; `unlock --force` overwrites  | No             |
| `wallet change-password`    | Change DB encryption password                            | Yes            |
| `wallet update-btc-address` | Change BTC address (before registration only)            | Yes            |
| `wallet sync-nonce`         | Sync nonce from chain                                    | Yes            |
| `wallet send`               | Send NYKS/SATS (`--to`, `--amount`, `--denom`)           | Yes            |
| `wallet register-btc`       | Register BTC for deposit (mainnet)                       | Yes            |
| `wallet deposit-btc`        | Send BTC to reserve; `--reserve-address` optional        | Yes            |
| `wallet reserves`           | Show reserve addresses + QR for best reserve             | Yes            |
| `wallet deposit-status`     | Check deposit/withdrawal status (mainnet)                | Yes            |
| `wallet withdraw-btc`       | Request BTC withdrawal (mainnet)                         | Yes            |
| `wallet withdraw-status`    | Check withdrawal confirmations (mainnet)                 | Yes            |
| `wallet faucet`             | Get test tokens (testnet)                                | Yes            |

### ZkOS Accounts

| Command              | Purpose                                           |
| -------------------- | ------------------------------------------------- |
| `zkaccount fund`     | Fund new ZkOS account from on-chain wallet        |
| `zkaccount withdraw` | Withdraw ZkOS account back to on-chain wallet     |
| `zkaccount transfer` | Transfer to fresh account (required before reuse) |
| `zkaccount split`    | Split one account into multiple                   |

### Orders

| Command                     | Purpose                                                                            | Wallet needed? |
| --------------------------- | ---------------------------------------------------------------------------------- | -------------- |
| `order open-trade`          | Open leveraged position; `--order-type MARKET` (default) or `LIMIT`                | Yes            |
| `order close-trade`         | Close position (MARKET/LIMIT/SLTP)                                                 | Yes            |
| `order cancel-trade`        | Cancel pending or remove SL/TP triggers                                            | Yes            |
| `order query-trade`         | Query order status (v1 with SL/TP/funding)                                         | Yes            |
| `order unlock-close-order`  | Unlock settled order → Coin                                                        | Yes            |
| `order unlock-failed-order` | Unlock failed order → Coin                                                         | Yes            |
| `order open-lend`           | Open lending position                                                              | Yes            |
| `order close-lend`          | Close lending position                                                             | Yes            |
| `order query-lend`          | Query lend order status                                                            | Yes            |
| `order history-trade`       | Historical trader orders (from relayer)                                            | Yes            |
| `order history-lend`        | Historical lend orders (from relayer)                                              | Yes            |
| `order funding-history`     | Funding payment history                                                            | Yes            |
| `order account-summary`     | Trading activity summary                                                           | Yes            |
| `order tx-hashes`           | Look up on-chain tx hashes by `--id`; `--by request` (default), `account`, or `tx` | **No**         |
| `order request-history`     | Look up tx hashes by account index (wallet resolves address)                       | Yes            |

### Market (no wallet needed)

| Command                   | Purpose                                              |
| ------------------------- | ---------------------------------------------------- |
| `market price`            | Current BTC/USD price                                |
| `market orderbook`        | Open limit orders                                    |
| `market funding-rate`     | Current funding rate                                 |
| `market fee-rate`         | Current fee rates                                    |
| `market recent-trades`    | Recent trades                                        |
| `market position-size`    | Aggregate long/short sizes                           |
| `market lend-pool`        | Lending pool info                                    |
| `market pool-share-value` | Pool share value                                     |
| `market last-day-apy`     | Last 24h APY                                         |
| `market open-interest`    | Open interest                                        |
| `market market-stats`     | Full market stats + funding rate                     |
| `market server-time`      | Relayer server time                                  |
| `market history-price`    | Historical prices (requires `--from`, `--to`)        |
| `market candles`          | OHLCV candles (requires `--since`)                   |
| `market history-funding`  | Historical funding rates (requires `--from`, `--to`) |
| `market history-fees`     | Historical fee rates (requires `--from`, `--to`)     |
| `market apy-chart`        | APY chart data                                       |

### Portfolio

| Command              | Purpose                                                         |
| -------------------- | --------------------------------------------------------------- |
| `portfolio summary`  | Full portfolio: balances, positions, PnL (auto-unlocks settled) |
| `portfolio balances` | Per-account balance breakdown (`--unit sats\|mbtc\|btc`)        |
| `portfolio risks`    | Liquidation risk for open positions                             |

### History (from local DB)

Requires a loadable wallet. Wallet ID and password use the same resolution as §3 (`resolve_wallet_id` and `resolve_password` inside `load_order_wallet_from_db`).

| Command             | Purpose                                                                                     |
| ------------------- | ------------------------------------------------------------------------------------------- |
| `history orders`    | Order history (open/close/cancel events); optional `--account-index`, `--limit`, `--offset` |
| `history transfers` | Transfer history (fund/withdraw/transfer events); optional `--limit`, `--offset`            |

### Bitcoin wallet (on-chain BTC)

Uses the wallet’s BIP-84 BTC keys when the wallet was created or imported from a **mnemonic**. `bitcoin-wallet transfer` requires that key material; see rule 7 in §13.

| Command                                | Purpose                                                              | Wallet needed?             |
| -------------------------------------- | -------------------------------------------------------------------- | -------------------------- |
| `bitcoin-wallet balance`               | On-chain BTC balance (`--btc-address`, `--btc`, `--mbtc`)            | If no `--btc-address`: Yes |
| `bitcoin-wallet receive`               | Show receive address + QR code, registration status                  | Yes                        |
| `bitcoin-wallet transfer`              | Sign and broadcast BTC (`--to`, amount flags, `--fee-rate`)          | Yes                        |
| `bitcoin-wallet update-bitcoin-wallet` | Re-derive BTC wallet from a new mnemonic (`--mnemonic` or TTY)       | Yes                        |
| `bitcoin-wallet history`               | BTC transfer history; auto-checks pending confirmations; `--status`, `--limit` | Yes                |

### Verify-test (testnet only)

Requires `NETWORK_TYPE=testnet`. Exits non-zero if any step fails. **`verify-test zkaccount`** and **`verify-test order`** need a funded wallet / ZkOS account respectively (see built-in help).

| Command                 | Purpose                                              |
| ----------------------- | ---------------------------------------------------- |
| `verify-test wallet`    | Exercise create, balance, faucet, send, etc.         |
| `verify-test market`    | Relayer market RPC checks                            |
| `verify-test zkaccount` | ZkOS account flows (needs on-chain balance / faucet) |
| `verify-test order`     | Order flows (needs funded ZkOS account)              |
| `verify-test all`       | Runs wallet → market → zkaccount → order in order    |

---

## 12. Multiple Orders via Split

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

| Account                       | Can split?                | Can place order?                         |
| ----------------------------- | ------------------------- | ---------------------------------------- |
| Split-created accounts (new)  | Yes                       | **Yes** — immediately ready              |
| Source account after split    | **Yes** — can split again | **No** — must `zkaccount transfer` first |
| Source account after transfer | Yes                       | **Yes** — fresh address                  |

**Limit:** Create at most **8 accounts per split** due to transaction size limits. For more, split in multiple calls.

---

## 13. Critical Rules

1. **Account address reuse**: A ZkOS account address can only be used for **one order**. After close + unlock, you **must** `zkaccount transfer` before placing a new order.

2. **Reserve timing**: BTC reserves rotate every ~144 blocks (~24h). The reserve must be ACTIVE when your BTC tx confirms. Status: ACTIVE (>72 blocks) > WARNING (5-72) > CRITICAL (<=4, don't send) > EXPIRED (0, don't send).

3. **Registered address only**: BTC deposits must come from your registered BTC address. Sending from any other address will not be credited.

4. **Market status (server-side, two-layer)**: The CLI pre-validates opens locally, but the **server** applies broader gates. **`HALT`** blocks **all** operations — opens, closes, cancels — except lend settlement. **`CLOSE_ONLY`** blocks new opens only; closes and cancels proceed normally. HALT can be triggered manually **or** automatically when pool equity drops to zero. Check status with **`market market-stats`** before retrying.

5. **Hard 50× leverage ceiling**: The server enforces a **fixed `leverage ≤ 50`** check on all order creation RPCs, independent of the configurable `max_leverage` from risk parameters. Even if `market market-stats` shows a higher `max_leverage` (unlikely but theoretically possible), **50× is the absolute ceiling**.

6. **Price feed pause**: When the price feed is **stale > 30 seconds** (or admin-paused), the server rejects **all non-LEND operations** — opens, closes, cancels, and SLTP execution. **Lend operations (`open-lend`, `close-lend`) are exempt** and proceed normally. The CLI gives no warning; the rejection appears as `"Risk engine rejected order"`. If you see this and market status is HEALTHY, the price feed is likely stale — wait and retry.

7. **LIMIT vs SLTP conflict**: `--order-type LIMIT` cannot be combined with `--stop-loss` or `--take-profit` on `close-trade`.

8. **Unlock requires settlement**: `unlock-close-order` returns an error if the order is not `SETTLED` or `LIQUIDATE`. It does not silently skip.

9. **BTC wallet availability**: On-chain BTC signing and auto-send to reserves need BIP-84 keys from a **mnemonic**-backed wallet. If those keys are missing: use **`wallet deposit-btc`** with manual payment to the reserve address, and plan fees yourself; balance checks may not include fee slack the same way.

10. **Network restrictions**: `register-btc`, `deposit-btc`, `deposit-status`, `withdraw-btc`, `withdraw-status` are **mainnet only**. `faucet` and `verify-test` are **testnet only**.

11. **Relayer program file**: ZkOS-facing commands (`zkaccount`, `order`, etc.) use circuit parameters from `RELAYER_PROGRAM_JSON_PATH` (default `./relayerprogram.json` in the process working directory) **if the file exists**. If the file is missing, the binary falls back to its **built-in default program JSON** and continues working. Override with a custom file only when needed (e.g., updated circuits not yet bundled in the binary).

---

## 14. Error Recovery

| Situation                                         | Command                                                                                |
| ------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Order submission failed, account stuck in Memo    | `order unlock-failed-order --account-index N`                                          |
| Order settled but account still Memo              | `order unlock-close-order --account-index N`                                           |
| Want to reuse account after order                 | `zkaccount transfer --account-index N`                                                 |
| Nonce/sequence mismatch                           | `wallet sync-nonce`                                                                    |
| Deposit stuck as pending                          | Check `wallet deposit-status`, ensure BTC tx has 1+ confirmations, wait for validators |
| "Value Witness Verification Failed" on open-trade | Account address was already used — run `zkaccount transfer` first                      |
| `query-trade` / `query-lend` returns "order failed" | The account was **auto-unlocked** back to `Coin` by the query itself — no manual unlock needed |
| All reserves expired                              | Wait for rotation (~144 blocks). `wallet reserves` shows ETA                           |
| “Session already cached” on `wallet unlock`       | `wallet lock` or `wallet unlock --force`                                               |
| Cannot script `wallet change-password`            | By design — interactive TTY only; env/session ignored                                  |
| “Market is halted” on **any** operation            | **HALT blocks everything** (opens, closes, cancels) — not just opens. Run `market market-stats`; if reason is `PoolEquityInvalid`, pool equity ≤ 0 (auto-halt). Wait for HALT to be lifted |
| “close-only mode” on open-\*                      | Only opens are blocked; closes and cancels still work. Run `market market-stats`; wait for **CLOSE_ONLY** to be lifted |
| Leverage or position size rejected                | Check `market market-stats` for `max_leverage`, min/max position rules                 |
| “Risk engine rejected order” (no CLI pre-error)   | Server-side gate. Check: (1) `market market-stats` for HALT, (2) price feed may be stale > 30s — wait and retry, (3) Kafka may be unhealthy — transient, retry after a few seconds |
| Close/cancel rejected during HALT                 | Unlike CLOSE_ONLY, **HALT blocks closes and cancels** too (§13 rule 4). Wait for HALT to be lifted |
| Lend operation fails with “risk engine rejected”  | Lend is exempt from price-feed pause but **not** from HALT or Kafka issues — check `market market-stats` |

---

## 15. JSON Output

All commands support `--json` for machine-readable output. Common patterns:

```bash
# Commands that submit orders/actions return request_id
relayer-cli --json order open-trade ...    # {"request_id": "..."}
relayer-cli --json order close-trade ...   # {"request_id": "..."}
relayer-cli --json order cancel-trade ...  # {"request_id": "..."}

# Query commands return full JSON objects
relayer-cli --json order query-trade --account-index 0   # TraderOrderV1 JSON
relayer-cli --json order query-lend --account-index 0    # LendOrderV1 JSON
relayer-cli --json market price                          # price value

# Tx hash lookup (no wallet; talks to relayer only)
relayer-cli --json order tx-hashes --by request --id REQID...
relayer-cli --json order tx-hashes --by account --id <twilight_or_zkos_address>
relayer-cli --json order tx-hashes --by tx --id <chain_tx_hash>

# Unlock commands return status
relayer-cli --json order unlock-close-order ...   # {"account_index": N, "order_status": "...", "request_id": "..."}
relayer-cli --json order unlock-failed-order ...  # {"account_index": N, "status": "unlocked"}
```

---

## 16. Further Reading

Agents that **only** have this skill file should **not** depend on the paths below. Developers with the full `nyks-wallet` repo may use:

| Document | Contents |
| -------- | -------- |
| [relayer-cli.md](relayer-cli.md) | Exhaustive per-command examples, every flag, output columns |
| [cli-command-rules.md](cli-command-rules.md) | Same preconditions as **§10** in prose tables; wallet-resolution footer is **wrong** vs §3 (trust §3) |
| [order-lifecycle.md](order-lifecycle.md) | Extra diagrams for ZkAccount / order states |
| [btc-onboarding.md](btc-onboarding.md) | BTC deposit/withdraw narrative and troubleshooting |
