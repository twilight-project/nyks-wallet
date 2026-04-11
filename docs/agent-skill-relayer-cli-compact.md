# Relayer CLI — Compact Agent Reference

Token-optimized version of `agent-skill-relayer-cli.md`. Every detail preserved; formatting compressed.

**Standalone use:** If the agent has only this file, treat it as the single source of truth. Default URLs, resolution order, and common flags are inlined below.
**When to use:** Install/build, `.env`, credentials, ZkOS account lifecycle, trade and lend orders, market queries, BTC onboarding, split accounts, error recovery, JSON output, and testnet verification.

## 1. Quick Start

Download pre-built binary from [GitHub releases](https://github.com/twilight-project/nyks-wallet/releases) — always check for the latest tag.
Asset names: `nyks-wallet-macos-arm64`, `nyks-wallet-linux-amd64`, `nyks-wallet-windows-amd64.exe`.

```bash
# macOS ARM64
curl -LO https://github.com/twilight-project/nyks-wallet/releases/download/v0.1.1-relayer-cli/nyks-wallet-macos-arm64
mv nyks-wallet-macos-arm64 relayer-cli && chmod +x relayer-cli

# Linux x86_64
curl -LO https://github.com/twilight-project/nyks-wallet/releases/download/v0.1.1-relayer-cli/nyks-wallet-linux-amd64
mv nyks-wallet-linux-amd64 relayer-cli && chmod +x relayer-cli

# Windows x86_64
curl -LO https://github.com/twilight-project/nyks-wallet/releases/download/v0.1.1-relayer-cli/nyks-wallet-windows-amd64.exe
mv nyks-wallet-windows-amd64.exe relayer-cli.exe
```

**Build from source** (Rust edition 2024, requires protoc + OpenSSL + pkg-config):
```bash
# Default (SQLite backend, bundled)
cargo build --release --bin relayer-cli
# PostgreSQL backend
cargo build --release --bin relayer-cli --no-default-features --features postgresql
```
Binary: `target/release/relayer-cli`. The binary is gated on the `order-wallet` feature (enabled by default with SQLite).

**Docker:**
```bash
docker build -t relayer-cli .
docker run -e RUST_LOG=info -e NETWORK_TYPE=testnet relayer-cli wallet balance
```

## 2. Configure

Mainnet .env: `RUST_LOG=info` (all defaults built in).
Testnet .env: `RUST_LOG=info` + `NETWORK_TYPE=testnet`.

**Env vars:**
- `NETWORK_TYPE`: `mainnet`|`testnet`, controls endpoints + BIP-44 derivation. Default `mainnet`
- `BTC_NETWORK_TYPE`: Bitcoin network for BTC keys. Default `mainnet`
- `NYKS_WALLET_ID`: default wallet ID when `--wallet-id` omitted
- `NYKS_WALLET_PASSPHRASE`: default password when `--password` omitted
- `DATABASE_URL_SQLITE`: SQLite path. Default `./wallet_data.db`
- `DATABASE_URL_POSTGRESQL`: PostgreSQL conn string (only with `--features postgresql`)
- `RELAYER_API_RPC_SERVER_URL`: Relayer JSON-RPC URL. Default network-dependent
- `ZKOS_SERVER_URL`: ZkOS server URL. Default network-dependent
- `RELAYER_PROGRAM_JSON_PATH`: circuit params JSON. Default `./relayerprogram.json`; falls back to built-in if missing
- `RUST_BACKTRACE`: `1`|`full` for stack traces
- `CHAIN_ID`: Cosmos chain id. Default `nyks`

**Endpoint defaults (switch with NETWORK_TYPE):**
- `NYKS_RPC_BASE_URL`: mainnet `https://rpc.twilight.org` / testnet `https://rpc.twilight.rest`
- `NYKS_LCD_BASE_URL`: mainnet `https://lcd.twilight.org` / testnet `https://lcd.twilight.rest`
- `FAUCET_BASE_URL`: mainnet _(disabled)_ / testnet `https://faucet-rpc.twilight.rest`
- `TWILIGHT_INDEXER_URL`: mainnet `https://indexer.twilight.org` / testnet `https://indexer.twilight.rest`
- `BTC_ESPLORA_PRIMARY_URL`: mainnet `https://blockstream.info/api` / testnet `https://blockstream.info/testnet/api`
- `BTC_ESPLORA_FALLBACK_URL`: mainnet `https://mempool.space/api` / testnet `https://mempool.space/testnet/api`
- `RELAYER_API_RPC_SERVER_URL`: mainnet `https://api.ephemeral.fi/api` / testnet `https://relayer.twilight.rest/api`
- `ZKOS_SERVER_URL`: mainnet `https://zkserver.twilight.org` / testnet `https://nykschain.twilight.rest/zkos`

**Working directory:** `DATABASE_URL_SQLITE` and `RELAYER_PROGRAM_JSON_PATH` default paths are relative to cwd.

**Credential resolution (most commands):** `--wallet-id`/`--password` → session cache (`wallet unlock`) → `NYKS_WALLET_ID`/`NYKS_WALLET_PASSPHRASE` env.
**`wallet unlock` only:** `--wallet-id` → env → interactive; `--password` → env → interactive. Validates credentials by loading wallet from DB before caching. Lists available wallets when prompting. Use `--force` to overwrite existing session.
**Session cache:** Unix only (macOS, Linux). Not available on Windows.

**Interactive-only commands:**
- `wallet import`: pass `--mnemonic "..."` for scripts; otherwise TTY prompt
- `wallet change-password`: always secure TTY; session/env ignored; confirms new password; auto-updates active session

**DB requirement:** Nearly all commands need wallet DB. Exceptions: `wallet create`, `wallet lock`, `bitcoin-wallet balance --btc-address`.

## 3. CLI Pattern

```
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

**Agent preflight checklist (run before state-changing commands):**
1. Confirm `NETWORK_TYPE` and network restrictions (rule 10)
2. Ensure wallet resolution is deterministic: pass `--wallet-id`/`--password` explicitly for automation, or confirm session/env state
3. `RELAYER_PROGRAM_JSON_PATH` is used if present; otherwise built-in default circuit params. Only verify the file if you need custom parameters
4. Before opening orders/lends, check market state and limits:
   - `relayer-cli --json market market-stats`
   - ensure status is not `HALT` / `CLOSE_ONLY`
   - ensure leverage and position size are within returned limits
5. Before BTC register/deposit/withdraw, check reserve freshness:
   - `relayer-cli wallet reserves`
   - avoid `CRITICAL` / `EXPIRED` reserves

## 4. Flag Patterns

**Amounts (exactly one unit flag per command):**
- `zkaccount fund`: `--amount` (sats) | `--amount-mbtc` | `--amount-btc`. One required
- `zkaccount split`: `--balances "a,b,c"` (sats) | `--balances-mbtc` | `--balances-btc`. Comma-separated
- `wallet deposit-btc`: `--amount` | `--amount-mbtc` | `--amount-btc`. Optional `--reserve-address`
- `wallet send`: `--to` (twilight addr) + `--amount` + `--denom` (nyks|sats). Required
- `bitcoin-wallet transfer`: `--to` (BTC addr) + amount flag + optional `--fee-rate`
- `wallet register-btc`: `--amount` (sats) req; `--staking-amount` opt (def 10000). Mainnet only
- **Priority if multiple given:** `--amount` > `--amount-mbtc` > `--amount-btc`

**Display units:** `bitcoin-wallet balance`: `--btc`|`--mbtc` (def sats). `portfolio balances`: `--unit sats|mbtc|btc`.
**Export:** `wallet export --output` (def `wallet.json`). `wallet backup --output` (def `wallet_backup.json`).
**Dates:** `--from`, `--to`, `--since` accept RFC3339 or `YYYY-MM-DD`.
**BTC addresses:** native SegWit only (`bc1q...`/`tb1q...`). Deposits must come from registered address.

**Trade order flags:**
- `open-trade`: `--account-index`, `--side` LONG|SHORT, `--entry-price` (USD int), `--leverage` (≤ max_leverage from market-stats), `--order-type` MARKET (def)|LIMIT
- `close-trade`: `--account-index`; def MARKET. LIMIT: `--order-type LIMIT --execution-price <P>` (req, >0). SLTP: `--stop-loss`/`--take-profit` (cannot combine with LIMIT)
- `cancel-trade`: `--account-index`. No SL/TP flags: cancel PENDING or remove close-limit on FILLED. With `--stop-loss`/`--take-profit`: remove those triggers on FILLED
- `tx-hashes`: `--id` req; `--by` request(def)|account|tx; opt `--status`, `--limit`, `--offset`, `--reason`
- `request-history`: `--account-index`; opt `--status`, `--limit`, `--offset`, `--reason`, `--wallet-id`, `--password`
- `account-summary`: opt `--from`, `--to`, `--since`

**Market data time args:**
- `history-price`: `--from`, `--to` req; `--limit` def 50, `--offset` def 0
- `candles`: `--since` req; `--interval` def `1h` (1m|5m|15m|30m|1h|4h|8h|12h|1d); `--limit`/`--offset` same
- `history-funding`, `history-fees`: `--from`, `--to` req; same defaults
- `apy-chart`: `--range` def `7d`; opt `--step`, `--lookback`

**Wallet list / DB path:** `wallet list` and some other wallet subcommands accept `--db-url` to override `DATABASE_URL_SQLITE` / PostgreSQL URL for that invocation.

**Lending flow:**
```bash
relayer-cli order open-lend --account-index N    # full account balance → pool; account → Memo / LENDTX
relayer-cli order query-lend --account-index N
relayer-cli order close-lend --account-index N   # after FILLED
relayer-cli order unlock-close-order --account-index N
relayer-cli zkaccount transfer --account-index N # before another lend/trade on same address
```

## 5. Workflows

**Testnet:**
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

**Mainnet (BTC onboarding):**
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
# 7. Fund ZkOS account and trade (for close/unlock/transfer cycle, mirror testnet steps 6-10)
relayer-cli zkaccount fund --amount 50000
relayer-cli order open-trade --account-index 0 --side LONG --entry-price 65000 --leverage 5
# 8. Withdraw ZkOS balance to chain, then BTC to external wallet
relayer-cli zkaccount withdraw --account-index 0
relayer-cli wallet withdraw-btc --reserve-id 1 --amount 50000
relayer-cli wallet withdraw-status
```

## 6. Account State Model

Each ZkOS account has three key fields:
- `io_type`: `Coin` | `Memo` | `State` — account state on ZkOS
- `tx_type`: `ORDERTX` | `LENDTX` | `None` — type of active order
- `on_chain`: `true` | `false` — whether account exists on-chain

**State transitions:**
- `Coin` = idle, ready for orders or transfers
- `Memo` = locked in an active order
- After close + unlock: back to `Coin`, but **must transfer before reuse**

**Implementation note:** `zkaccount withdraw` internally performs a **transfer first** (creates a fresh account), then burns from the new account back to the on-chain wallet. This is why withdrawals take longer than a simple transfer.

## 7. Order Lifecycle

**Trade:**
```
Coin → open-trade → Memo(PENDING)
  ├─ fills → FILLED
  │   ├─ close-trade(MARKET) → SETTLED → unlock → Coin
  │   ├─ close-trade(LIMIT) → settle_limit → SETTLED → unlock → Coin
  │   ├─ close-trade(SLTP) → SL/TP triggers → SETTLED → unlock → Coin
  │   └─ liquidation → LIQUIDATE → unlock → Coin
  ├─ cancel-trade(PENDING) → CANCELLED → Coin
  └─ failed → unlock-failed-order → Coin
```

**Cancel-trade:**
- No flags: PENDING → CANCELLED; or FILLED + settle_limit → removes limit
- `--stop-loss`: FILLED → removes SL trigger
- `--take-profit`: FILLED → removes TP trigger
- Both flags: removes both

**Close-trade:** MARKET (def, must be FILLED) | LIMIT (`--execution-price` req, >0) | SLTP (`--stop-loss`/`--take-profit`). LIMIT cannot combine with SLTP.

**Lend:** Coin → open-lend → Memo(PENDING) → FILLED → close-lend → SETTLED → unlock → Coin.

**Limit fill conditions:**
- LONG fills when market price rises **above** limit price ("buy low" — your bid gets hit)
- SHORT fills when market price drops **below** limit price ("sell high" — your ask gets hit)
- The relayer checks on **every price tick** (not only on user requests). Once filled, the order is immediately active at `current_price` as entry price.

**SL/TP triggers (server-side, every tick):**
- LONG SL: `sl >= current_price`. LONG TP: `tp <= current_price`
- SHORT SL: `sl <= current_price`. SHORT TP: `tp >= current_price`
- Triggered positions settle at market using FilledOnMarket fees.

## 8. PnL, Fees, Funding, Liquidation

**PnL (inverse perpetual, sats-denominated):**
- LONG: `pnl = position_size × (settle - entry) / (entry × settle)`
- SHORT: `pnl = position_size × (entry - settle) / (entry × settle)`
- `position_size = initial_margin × leverage` (sats). Equal USD moves → different sat PnL.

**Fee:** `fee = (fee_pct / 100) × position_size / current_price`. Min floor: 1 sat.
- `FilledOnMarket`: market close, SL/TP trigger, liquidation
- `FilledOnLimit`: limit close filling immediately
- `SettledOnMarket`/`SettledOnLimit`: stored, not used in settlement math
- Current rates: `market fee-rate`

**Funding rate (periodic, adjusts available_margin on FILLED positions):**
- Global: `rate = ((total_long - total_short) / all_position_size)² / (ψ × 8)`, negative when shorts dominate (ψ=1.0)
- Per order: `funding_payment = (rate × position_size) / (current_price × 100)`
- LONGs pay when longs dominate; SHORTs pay when shorts dominate
- `available_margin` clamped to 0 (never negative from funding, but 0 can trigger liquidation)
- Check: `market funding-rate`. History: `market history-funding`

**Maintenance margin:** `MM = (mm_ratio × entry_value + fee × bankruptcy_value + funding × bankruptcy_value) / 100`. Default `mm_ratio = 0.4`.

**Liquidation triggers:**
1. Price-driven: market price crosses liquidation price (every tick)
2. Funding-driven: `available_margin <= maintenance_margin` after funding cycle
- Result: `available_margin = 0` (total loss), `payment = -initial_margin` (absorbed by lend pool)

**Settlement payment:**
```
margin_difference = available_margin - initial_margin  (accumulated funding)
payment = round(unrealized_pnl) + round(margin_difference) - fee
post_settle_am = round(initial_margin + payment)
```

## 9. Server-Side Risk Gates

- **HALT**: blocks ALL ops (opens, closes, cancels) except lend settlement. Detect: `market market-stats` → HALT. Causes: manual halt OR pool equity ≤ 0 (PoolEquityInvalid)
- **CLOSE_ONLY**: blocks new opens only; closes/cancels proceed. Detect: `market market-stats` → CLOSE_ONLY
- **Price feed paused** (stale >30s or admin): blocks all non-LEND ops. Appears as "Risk engine rejected order". Lend exempt
- **Kafka unhealthy** (10+ failures): blocks ALL ops. Transient — retry
- **Pool equity ≤ 0**: auto-triggers HALT

**Default risk params** (prefer live values from `market market-stats`):
- `max_leverage` 50.0 (hard 50× ceiling + configurable cap)
- `max_oi_mult` 4.0 (max OI = 4× pool equity)
- `max_net_mult` 0.8 (max net exposure = 0.8× pool equity)
- `max_position_pct` 0.02 (2% pool equity per position)
- `min_position_btc` 0 (disabled)
- `mm_ratio` 0.4

## 10. Preconditions

**Wallet lifecycle:**
- `create`: opt `--wallet-id`, `--password`, `--btc-address` (SegWit). Prints mnemonic once
- `import`: mnemonic req (`--mnemonic` or TTY). Opt `--btc-address` (SegWit; if omitted, derived). BTC addr must not be registered to different Twilight addr
- `load`: `--wallet-id` req. Password correct. Opt `--db-url`
- `backup`: `--wallet-id` req
- `restore`: `--input` + `--wallet-id` req; `--force` if wallet_id mismatch
- `register-btc`: mainnet only. Reserves exist, ≥1 ACTIVE. Not already registered. Balance depends on btc_wallet (rule 9)
- `deposit-btc`: mainnet only. BTC addr registered. One amount flag. `--reserve-address` opt (must have >4 blocks; else auto-selects). Auto-pays if mnemonic btc_wallet available
- `withdraw-btc`: mainnet only. Registered BTC; reserve exists; sufficient balance
- `update-btc-address`/`update-bitcoin-wallet`: current BTC not registered; new addr valid SegWit, not tied to another Twilight addr

**ZkOS account:**
- `fund`: wallet loadable; one amount flag; amount > 0; enough on-chain sats
- `withdraw`/`transfer`/`split`: account exists, Coin + on_chain
- `split` additionally: sum ≤ balance; no zeros; max 8 accounts per split

**Trading & lending:** Wallet loadable. Server-side gates (§9) apply: HALT blocks closes/cancels too; price feed pause blocks non-lend; Kafka blocks all.

**open-trade local pre-validation (5 steps):**
1. Market status ≠ HALT/CLOSE_ONLY
2. leverage ≤ params.max_leverage
3. entry_value (margin × leverage) ≥ params.min_position_btc
4. entry_value ≤ params.max_position_pct × pool_equity
5. entry_value ≤ max_long_btc (LONG) or max_short_btc (SHORT)

**open-lend:** checks only step 1.

- `open-trade`/`open-lend`: account Coin + on_chain; address not used for prior order without transfer
- `close-trade`: order FILLED (SETTLED/LIQUIDATE auto-unlocks). LIMIT needs positive `--execution-price`; no LIMIT+SLTP
- `cancel-trade`: see §4 flag patterns
- `query-trade`: active trader order. Side effect: failed order → auto-unlocked to Coin
- `query-lend`: active lend order. Side effect: failed → auto-unlocked to Coin
- `close-lend`: lend FILLED
- `unlock-close-order`: status SETTLED or LIQUIDATE
- `unlock-failed-order`: account Memo with no active order
- `history-trade`/`history-lend`: wallet loadable; signed query

**No wallet needed:** `order tx-hashes` (`--by` ∈ request|account|tx), all `market *` commands.
**Local DB history:** `history orders`/`transfers`: wallet_id resolved per §2; `--limit` def 50, `--offset` def 0.
**Portfolio:** `summary` (auto-unlocks settled); `risks` (needs live prices). Both need relayer.

## 11. Command Quick Reference

**wallet** commands:
- `wallet create` [N] — Create new wallet
- `wallet import` [N] — Import from mnemonic; optional `--btc-address`
- `wallet load` [N] — Load from DB (`--wallet-id` required)
- `wallet list` [N] — List all wallets
- `wallet balance` [W] — Show NYKS/SATS balance
- `wallet accounts` [W] — List ZkOS accounts; optional `--on-chain-only`
- `wallet info` [W] — Show wallet info (no chain call)
- `wallet export` [W] — Export to JSON (`--output`, default `wallet.json`)
- `wallet backup`/`restore` [W] — Full DB backup/restore (`backup` requires `--wallet-id`)
- `wallet unlock`/`lock` [N] — Session credential caching; `unlock --force` overwrites
- `wallet change-password` [W] — Change DB encryption password
- `wallet update-btc-address` [W] — Change BTC address (before registration only)
- `wallet sync-nonce` [W] — Sync nonce from chain
- `wallet send` [W] — Send NYKS/SATS (`--to`, `--amount`, `--denom`)
- `wallet register-btc` [W,M] — Register BTC for deposit (mainnet)
- `wallet deposit-btc` [W,M] — Send BTC to reserve; `--reserve-address` optional
- `wallet reserves` [W] — Show reserve addresses + QR for best reserve
- `wallet deposit-status` [W,M] — Check deposit/withdrawal status
- `wallet withdraw-btc` [W,M] — Request BTC withdrawal
- `wallet withdraw-status` [W,M] — Check withdrawal confirmations
- `wallet faucet` [W,T] — Get test tokens (testnet)

**zkaccount** commands (all need wallet):
- `zkaccount fund` — Fund new ZkOS account from on-chain wallet
- `zkaccount withdraw` — Withdraw ZkOS account back to on-chain wallet
- `zkaccount transfer` — Transfer to fresh account (required before reuse)
- `zkaccount split` — Split one account into multiple

**order** commands:
- `order open-trade` [W] — Open leveraged position; `--order-type MARKET` (default) or `LIMIT`
- `order close-trade` [W] — Close position (MARKET/LIMIT/SLTP)
- `order cancel-trade` [W] — Cancel pending or remove SL/TP triggers
- `order query-trade` [W] — Query order status (v1 with SL/TP/funding)
- `order unlock-close-order` [W] — Unlock settled order → Coin
- `order unlock-failed-order` [W] — Unlock failed order → Coin
- `order open-lend` [W] — Open lending position
- `order close-lend` [W] — Close lending position
- `order query-lend` [W] — Query lend order status
- `order history-trade` [W] — Historical trader orders (from relayer)
- `order history-lend` [W] — Historical lend orders (from relayer)
- `order funding-history` [W] — Funding payment history
- `order account-summary` [W] — Trading activity summary
- `order tx-hashes` [N] — Look up on-chain tx hashes by `--id`; `--by request` (default), `account`, or `tx`
- `order request-history` [W] — Look up tx hashes by account index (wallet resolves address)

**market** commands (no wallet needed):
- `market price` — Current BTC/USD price
- `market orderbook` — Open limit orders
- `market funding-rate` — Current funding rate
- `market fee-rate` — Current fee rates
- `market recent-trades` — Recent trades
- `market position-size` — Aggregate long/short sizes
- `market lend-pool` — Lending pool info
- `market pool-share-value` — Pool share value
- `market last-day-apy` — Last 24h APY
- `market open-interest` — Open interest
- `market market-stats` — Full market stats + funding rate
- `market server-time` — Relayer server time
- `market history-price` — Historical prices (requires `--from`, `--to`)
- `market candles` — OHLCV candles (requires `--since`)
- `market history-funding` — Historical funding rates (requires `--from`, `--to`)
- `market history-fees` — Historical fee rates (requires `--from`, `--to`)
- `market apy-chart` — APY chart data

**portfolio** commands:
- `portfolio summary` [W] — Full portfolio: balances, positions, PnL (auto-unlocks settled)
- `portfolio balances` [W] — Per-account balance breakdown (`--unit sats|mbtc|btc`)
- `portfolio risks` [W] — Liquidation risk for open positions

**history** commands (local DB, requires loadable wallet; wallet_id/password use same resolution as §2):
- `history orders` [W] — Order history (open/close/cancel events); opt `--account-index`, `--limit`, `--offset`
- `history transfers` [W] — Transfer history (fund/withdraw/transfer events); opt `--limit`, `--offset`

**bitcoin-wallet** commands (use BIP-84 BTC keys from mnemonic-backed wallet; `transfer` requires that key material):
- `bitcoin-wallet balance` — On-chain BTC balance (`--btc-address`, `--btc`, `--mbtc`). If no `--btc-address`: wallet needed
- `bitcoin-wallet receive` [W] — Show receive address + QR code, registration status
- `bitcoin-wallet transfer` [W] — Sign and broadcast BTC (`--to`, amount flags, `--fee-rate`)
- `bitcoin-wallet update-bitcoin-wallet` [W] — Re-derive BTC wallet from a new mnemonic (`--mnemonic` or TTY)
- `bitcoin-wallet history` [W] — BTC transfer history; auto-checks pending confirmations; `--status`, `--limit`

**verify-test** (testnet only, exits non-zero if any step fails):
- `verify-test wallet` — Exercise create, balance, faucet, send, etc.
- `verify-test market` — Relayer market RPC checks
- `verify-test zkaccount` — ZkOS account flows (needs on-chain balance / faucet)
- `verify-test order` — Order flows (needs funded ZkOS account)
- `verify-test all` — Runs wallet → market → zkaccount → order in order

[W]=wallet needed, [N]=no wallet, [M]=mainnet only, [T]=testnet only

## 12. Multiple Orders via Split

Split a funded account into multiple for parallel orders. **Split-created accounts** are immediately ready for orders. **Source account** keeps remaining balance but must be rotated (`zkaccount transfer`) before placing an order — however, it can be split again without rotating.

```bash
# Account 0 has 20,000 sats after funding
relayer-cli zkaccount fund --amount 20000

# Split into two new accounts (remaining 13,000 stays in account 0)
relayer-cli zkaccount split --account-index 0 --balances "5000,2000"
# Creates: account 1 (5,000 sats), account 2 (2,000 sats)

# Accounts 1 and 2 are ready for orders immediately
relayer-cli order open-trade --account-index 1 --side LONG --entry-price 65000 --leverage 5
relayer-cli order open-lend --account-index 2

# Account 0 can be split further WITHOUT rotating
relayer-cli zkaccount split --account-index 0 --balances "3000,4000"
# Creates: account 3 (3,000 sats), account 4 (4,000 sats)

# But to place an order FROM account 0, rotate first
relayer-cli zkaccount transfer --account-index 0
# Now the new account (index 5) with 6,000 sats is ready for orders
relayer-cli order open-trade --account-index 5 --side SHORT --entry-price 70000 --leverage 3
```

**Split rules:**
- Split-created accounts (new): can split again ✓, can place order immediately ✓
- Source account after split: can split again ✓, **cannot** place order — must `zkaccount transfer` first
- Source account after transfer: can split ✓, can place order ✓ (fresh address)

**Limit:** At most **8 accounts per split** due to transaction size limits. For more, split in multiple calls.

## 13. Critical Rules

1. **Account address reuse**: A ZkOS account address can only be used for **one order**. After close + unlock, you **must** `zkaccount transfer` before placing a new order.

2. **Reserve timing**: BTC reserves rotate every ~144 blocks (~24h). The reserve must be ACTIVE when your BTC tx confirms. Status: ACTIVE (>72 blocks) > WARNING (5-72) > CRITICAL (<=4, don't send) > EXPIRED (0, don't send).

3. **Registered address only**: BTC deposits must come from your registered BTC address. Sending from any other address will not be credited.

4. **Market status (server-side, two-layer)**: CLI pre-validates opens locally, but the **server** applies broader gates. **`HALT`** blocks **all** operations — opens, closes, cancels — except lend settlement. **`CLOSE_ONLY`** blocks new opens only; closes and cancels proceed. HALT triggers: manual **or** automatic when pool equity ≤ 0. Check with `market market-stats`.

5. **Hard 50× leverage ceiling**: Server enforces `leverage ≤ 50` on all order creation RPCs, independent of configurable `max_leverage`. Even if `market market-stats` shows higher, **50× is the absolute ceiling**.

6. **Price feed pause**: When price feed is stale >30s (or admin-paused), server rejects **all non-LEND operations**. **Lend operations exempt**. No CLI warning; rejection appears as `"Risk engine rejected order"`. If market status is HEALTHY but orders fail, price feed is likely stale — wait and retry.

7. **LIMIT vs SLTP conflict**: `--order-type LIMIT` cannot be combined with `--stop-loss` or `--take-profit` on `close-trade`.

8. **Unlock requires settlement**: `unlock-close-order` returns an error if the order is not `SETTLED` or `LIQUIDATE`. It does not silently skip.

9. **BTC wallet availability**: On-chain BTC signing and auto-send to reserves need BIP-84 keys from a **mnemonic**-backed wallet. Without: use `wallet deposit-btc` with manual payment to the reserve address, and plan fees yourself.

10. **Network restrictions**: `register-btc`, `deposit-btc`, `deposit-status`, `withdraw-btc`, `withdraw-status` are **mainnet only**. `faucet` and `verify-test` are **testnet only**.

11. **Relayer program file**: `RELAYER_PROGRAM_JSON_PATH` (default `./relayerprogram.json` relative to cwd). If file missing, binary falls back to **built-in default program JSON**. Override only when needed (e.g., updated circuits not yet bundled).

## 14. Error Recovery

- **Order submission failed, account stuck in Memo** → `order unlock-failed-order --account-index N`
- **Order settled but account still Memo** → `order unlock-close-order --account-index N`
- **Want to reuse account after order** → `zkaccount transfer --account-index N`
- **Nonce/sequence mismatch** → `wallet sync-nonce`
- **Deposit stuck as pending** → Check `wallet deposit-status`, ensure BTC tx has 1+ confirmations, wait for validators
- **"Value Witness Verification Failed" on open-trade** → Account address was already used — run `zkaccount transfer` first
- **`query-trade`/`query-lend` returns "order failed"** → Account was **auto-unlocked** back to Coin by the query itself — no manual unlock needed
- **All reserves expired** → Wait for rotation (~144 blocks). `wallet reserves` shows ETA
- **"Session already cached" on `wallet unlock`** → `wallet lock` or `wallet unlock --force`
- **Cannot script `wallet change-password`** → By design — interactive TTY only; env/session ignored
- **"Market is halted" on any operation** → **HALT blocks everything** (opens, closes, cancels) — not just opens. Run `market market-stats`; if reason is `PoolEquityInvalid`, pool equity ≤ 0 (auto-halt). Wait for HALT to be lifted
- **"close-only mode" on open-\*** → Only opens are blocked; closes and cancels still work. Run `market market-stats`; wait for CLOSE_ONLY to be lifted
- **Leverage or position size rejected** → Check `market market-stats` for `max_leverage`, min/max position rules
- **"Risk engine rejected order" (no CLI pre-error)** → Server-side gate. Check: (1) `market market-stats` for HALT, (2) price feed may be stale >30s — wait and retry, (3) Kafka may be unhealthy — transient, retry after a few seconds
- **Close/cancel rejected during HALT** → Unlike CLOSE_ONLY, **HALT blocks closes and cancels** too (§13 rule 4). Wait for HALT to be lifted
- **Lend operation fails with "risk engine rejected"** → Lend is exempt from price-feed pause but **not** from HALT or Kafka issues — check `market market-stats`

## 15. JSON Output

All commands support `--json` for machine-readable output. Common patterns:

```bash
# Commands that submit orders/actions return request_id
relayer-cli --json order open-trade ...          # {"request_id": "..."}
relayer-cli --json order close-trade ...         # {"request_id": "..."}
relayer-cli --json order cancel-trade ...        # {"request_id": "..."}

# Query commands return full JSON objects
relayer-cli --json order query-trade --account-index 0   # TraderOrderV1 JSON
relayer-cli --json order query-lend --account-index 0    # LendOrderV1 JSON
relayer-cli --json market price                          # price value

# Tx hash lookup (no wallet; talks to relayer only)
relayer-cli --json order tx-hashes --by request --id REQID
relayer-cli --json order tx-hashes --by account --id <twilight_or_zkos_address>
relayer-cli --json order tx-hashes --by tx --id <chain_tx_hash>

# Unlock commands return status
relayer-cli --json order unlock-close-order ...   # {"account_index": N, "order_status": "...", "request_id": "..."}
relayer-cli --json order unlock-failed-order ...  # {"account_index": N, "status": "unlocked"}
```

## 16. Further Reading

Agents with only this file should **not** depend on the paths below. Developers with the full `nyks-wallet` repo may use:
- [relayer-cli.md](relayer-cli.md) — Exhaustive per-command examples, every flag, output columns
- [cli-command-rules.md](cli-command-rules.md) — Same preconditions as §10 in prose tables; wallet-resolution footer is **wrong** vs §2 (trust §2)
- [order-lifecycle.md](order-lifecycle.md) — Extra diagrams for ZkAccount / order states
- [btc-onboarding.md](btc-onboarding.md) — BTC deposit/withdraw narrative and troubleshooting
