# Agent Skill: Relayer CLI

A complete guide for AI agents and developers to build, configure, and operate the Twilight `relayer-cli`. This document is self-contained — read it to go from zero to executing trades.

---

## 1. Quick Start (pre-built binary)

Download a pre-built binary from [GitHub releases](https://github.com/twilight-project/nyks-wallet/releases) — no build tools required.

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

### Minimal .env for mainnet

```bash
RUST_LOG=info
```

That's it. All defaults (`NETWORK_TYPE=mainnet`, endpoints, database path, relayer program path) are built into the binary. No other env vars are required.

### Minimal .env for testnet

```bash
RUST_LOG=info
NETWORK_TYPE=testnet
```

All testnet endpoints auto-resolve from `NETWORK_TYPE=testnet`. Override only if needed (e.g., local development).

### Key env vars

| Variable | Purpose | Default |
|---|---|---|
| `NETWORK_TYPE` | `mainnet` or `testnet` — controls endpoints and BIP-44 derivation | `mainnet` |
| `BTC_NETWORK_TYPE` | Bitcoin network for BTC keys/balance — falls back to hardcoded `mainnet` | `mainnet` |
| `NYKS_WALLET_ID` | Default wallet ID when `--wallet-id` is omitted | — |
| `NYKS_WALLET_PASSPHRASE` | Default password when `--password` is omitted | — |
| `DATABASE_URL_SQLITE` | SQLite database path | `./wallet_data.db` |

### Wallet credential resolution order

1. `--wallet-id` / `--password` CLI flags
2. Session cache (`wallet unlock`)
3. `NYKS_WALLET_ID` / `NYKS_WALLET_PASSPHRASE` env vars
4. Interactive prompt (for `wallet unlock` only)

---

## 4. Global CLI pattern

```bash
relayer-cli [--json] <command-group> <subcommand> [flags]
```

`--json` outputs JSON instead of formatted tables. Useful for scripting/parsing.

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

---

## 7. Account State Model

Each ZkOS account has three key fields:

| Field | Values | Meaning |
|---|---|---|
| `io_type` | `Coin`, `Memo`, `State` | Account state on ZkOS |
| `tx_type` | `ORDERTX`, `LENDTX`, `None` | Type of active order |
| `on_chain` | `true`, `false` | Whether account exists on-chain |

**State transitions:**
- `Coin` = idle, ready for orders or transfers
- `Memo` = locked in an active order
- After close + unlock: back to `Coin`, but **must transfer before reuse**

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

### Lend order

```
Coin → open-lend → Memo (PENDING) → fills → FILLED
  ├── close-lend → SETTLED → unlock → Coin
  └── failed     → unlock-failed-order → Coin
```

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
| `order tx-hashes` | Look up tx hashes by ID | **No** |
| `order request-history` | Look up tx hashes by account index | Yes |

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

2. **Reserve timing**: BTC reserves rotate every ~144 blocks (~24h). The reserve must be ACTIVE when your BTC tx confirms. Status: ACTIVE (>72 blocks) > WARNING (5-72) > CRITICAL (<=4, don't send) > EXPIRED (0, don't send).

3. **Registered address only**: BTC deposits must come from your registered BTC address. Sending from any other address will not be credited.

4. **Market halt**: Trading commands check if the market is halted before executing. If halted, the command returns an error.

5. **LIMIT vs SLTP conflict**: `--order-type LIMIT` cannot be combined with `--stop-loss` or `--take-profit` on `close-trade`.

6. **Unlock requires settlement**: `unlock-close-order` returns an error if the order is not `SETTLED` or `LIQUIDATE`. It does not silently skip.

7. **BTC wallet availability**: Auto-pay (register-btc, deposit-btc) only works when the wallet was created/imported from a **mnemonic**. Otherwise, manual BTC payment is required.

8. **Network restrictions**: `register-btc`, `deposit-btc`, `deposit-status`, `withdraw-btc`, `withdraw-status` are **mainnet only**. `faucet` is **testnet only**.

---

## 12. Error Recovery

| Situation | Command |
|---|---|
| Order submission failed, account stuck in Memo | `order unlock-failed-order --account-index N` |
| Order settled but account still Memo | `order unlock-close-order --account-index N` |
| Want to reuse account after order | `zkaccount transfer --account-index N` |
| Nonce/sequence mismatch | `wallet sync-nonce` |
| Deposit stuck as pending | Check `wallet deposit-status`, ensure BTC tx has 1+ confirmations, wait for validators |
| "Value Witness Verification Failed" on open-trade | Account address was already used — run `zkaccount transfer` first |
| All reserves expired | Wait for rotation (~144 blocks). `wallet reserves` shows ETA |

---

## 13. JSON Output

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

# Unlock commands return status
relayer-cli --json order unlock-close-order ...   # {"account_index": N, "order_status": "...", "request_id": "..."}
relayer-cli --json order unlock-failed-order ...  # {"account_index": N, "status": "unlocked"}
```

---

## 14. Further Reading

| Document | Contents |
|---|---|
| [relayer-cli.md](relayer-cli.md) | Full CLI reference — every flag, every output column |
| [cli-command-rules.md](cli-command-rules.md) | Preconditions and requirements for every command |
| [order-lifecycle.md](order-lifecycle.md) | ZkAccount states, trade/lend lifecycle diagrams |
| [btc-onboarding.md](btc-onboarding.md) | BTC deposit/withdrawal flow with troubleshooting |
