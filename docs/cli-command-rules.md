# CLI Command Rules & Preconditions

A complete reference of what each `relayer-cli` subcommand requires before it will execute.
Use this as a checklist before calling any command.

---

## Wallet Commands

### `wallet create`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id`, `--password`, `--btc-address` (all optional) |
| Preconditions | If `--btc-address` provided: must be valid native SegWit (`bc1q...` / `tb1q...`) |
| Action | Creates a new wallet, optionally with custom BTC address, and persists to database |

### `wallet import`

| Requirement | Details |
|---|---|
| Flags | `--mnemonic` (prompted if omitted), `--wallet-id`, `--password`, `--btc-address` (optional) |
| Preconditions | Mnemonic must not be empty |
| | If `--btc-address` provided: must be valid native SegWit |
| | BTC address (custom or derived) must not be registered to a different twilight address on-chain |
| Action | Imports wallet from BIP-39 mnemonic, derives BTC keys, persists to database |

### `wallet load`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id` (required), `--password`, `--db-url` (optional) |
| Preconditions | Database features must be enabled (`--features sqlite`) |
| | Wallet must exist in database |
| | Password must be correct |
| Action | Loads and displays wallet info from database |

### `wallet balance`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable (from args, env vars, or session) |
| | Chain must be reachable |
| Action | Fetches and displays NYKS and SATS balance from chain |

### `wallet list`

| Requirement | Details |
|---|---|
| Flags | `--db-url` (optional) |
| Preconditions | Database features must be enabled |
| Action | Lists all wallets stored in database |

### `wallet export`

| Requirement | Details |
|---|---|
| Flags | `--output` (default: `wallet.json`), `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| Action | Exports wallet to JSON file |

### `wallet accounts`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id`, `--password`, `--on-chain-only` (optional) |
| Preconditions | Wallet must be loadable |
| Action | Lists all ZkOS accounts for the wallet |

### `wallet info`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id`, `--password` (optional) |
| Preconditions | Database features must be enabled |
| | Wallet must be loadable |
| Action | Shows wallet address, BTC address, chain_id, account count, nonce info |

### `wallet backup`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id` (required), `--output` (default: `wallet_backup.json`), `--password` (optional) |
| Preconditions | Database features must be enabled |
| | Wallet must exist and be loadable |
| Action | Exports full database backup to JSON file |

### `wallet restore`

| Requirement | Details |
|---|---|
| Flags | `--input` (required), `--wallet-id` (required), `--password`, `--force` (optional) |
| Preconditions | Database features must be enabled |
| | Input file must exist and be valid backup JSON |
| | If not `--force`: backup wallet_id must match target wallet_id |
| Action | Restores wallet from backup JSON file |

### `wallet sync-nonce`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| | Chain must be reachable |
| Action | Syncs nonce/sequence manager from chain state |

### `wallet unlock`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id`, `--password`, `--force` (optional) |
| Preconditions | If session already exists and not `--force`: returns error |
| | wallet_id resolved from: flag -> `NYKS_WALLET_ID` env -> prompt |
| | password resolved from: flag -> `NYKS_WALLET_PASSPHRASE` env -> prompt |
| | wallet_id and password must not be empty |
| | Combination must be valid (verified by loading from DB) |
| Action | Caches wallet_id and password for the terminal session |

### `wallet lock`

| Requirement | Details |
|---|---|
| Flags | None |
| Preconditions | None |
| Action | Clears cached session credentials |

### `wallet change-password`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id` (optional) |
| Preconditions | Database features must be enabled |
| | wallet_id required (from flag, env, or session) |
| | Old password must be correct |
| | New password must not be empty |
| | New password confirmation must match |
| Action | Changes database encryption password, updates session if active |

### `wallet update-btc-address`

| Requirement | Details |
|---|---|
| Flags | `--btc-address` (required), `--wallet-id`, `--password` (optional) |
| Preconditions | Database features must be enabled |
| | New address must be valid native SegWit (`bc1q...` / `tb1q...`) |
| | Current BTC address must **not** be registered on-chain (if registered, cannot change) |
| | New address must not be linked to a different twilight address on-chain |
| Action | Updates BTC address, invalidates btc_wallet keys, persists to database |

### `wallet send`

| Requirement | Details |
|---|---|
| Flags | `--to` (required), `--amount` (required), `--denom` (default: `nyks`), `--wallet-id`, `--password` (optional) |
| Preconditions | Database features must be enabled |
| | Wallet must be loadable |
| | Wallet must have sufficient balance |
| Action | Sends tokens (nyks or sats) to another Twilight address |

### `wallet register-btc`

| Requirement | Details |
|---|---|
| Flags | `--amount` (required, in sats), `--staking-amount` (default: 10000), `--wallet-id`, `--password` (optional) |
| Preconditions | **Mainnet only** (use `wallet faucet` on testnet) |
| | Database features must be enabled |
| | BTC address must not already be registered to this wallet (if so: use `deposit-btc` instead) |
| | BTC address must not be registered to a different twilight address |
| | At least one BTC reserve must exist on-chain |
| | At least one reserve must be ACTIVE (> 4 blocks remaining) |
| | If btc_wallet available: confirmed BTC balance must cover `amount + estimated_fee` |
| | If no btc_wallet: confirmed BTC balance must cover `amount` (fees are user's responsibility) |
| Action | Registers BTC deposit address on-chain. If btc_wallet available, auto-sends to best reserve. Otherwise shows manual instructions. |

### `wallet reserves`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id`, `--password` (optional) |
| Preconditions | Database features must be enabled |
| | Wallet must be loadable |
| Action | Displays BTC reserve addresses with status and QR code for best reserve |

### `wallet deposit-btc`

| Requirement | Details |
|---|---|
| Flags | At least one of: `--amount`, `--amount-mbtc`, `--amount-btc` (required). `--reserve-address`, `--wallet-id`, `--password` (optional) |
| Preconditions | **Mainnet only** |
| | Database features must be enabled |
| | Exactly one amount format must be provided |
| | Amount must be > 0 |
| | BTC address must be registered on-chain to your twilight address |
| | If `--reserve-address` provided: reserve must exist and have > 4 blocks remaining |
| | If no reserve specified: at least one ACTIVE reserve must exist |
| | If btc_wallet available: confirmed BTC balance must cover `amount + estimated_fee` |
| Action | If btc_wallet available, auto-sends to target reserve. Otherwise records deposit intent and shows manual instructions. |

### `wallet deposit-status`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id`, `--password` (optional) |
| Preconditions | **Mainnet only** |
| | Database features must be enabled |
| | Wallet must be loadable |
| Action | Checks on-chain deposit/withdrawal status, updates local database |

### `wallet withdraw-btc`

| Requirement | Details |
|---|---|
| Flags | `--reserve-id` (required), `--amount` (required), `--wallet-id`, `--password` (optional) |
| Preconditions | **Mainnet only** |
| | Database features must be enabled |
| | BTC address must be registered on-chain |
| | Reserve must exist |
| | Wallet must have sufficient balance in the reserve |
| Action | Submits BTC withdrawal request, saves record to database |

### `wallet withdraw-status`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id`, `--password` (optional) |
| Preconditions | **Mainnet only** |
| | Database features must be enabled |
| | Wallet must be loadable |
| Action | Checks pending withdrawals against on-chain status, updates database |

### `wallet faucet`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id`, `--password` (optional) |
| Preconditions | **Testnet only** (rejected on mainnet) |
| | Database features must be enabled |
| | Wallet must be loadable |
| Action | Requests test tokens from faucet, displays updated balance |

---

## Bitcoin Wallet Commands

### `bitcoin-wallet balance`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id`, `--password`, `--btc-address`, `--btc`, `--mbtc` (all optional) |
| Preconditions | If `--btc-address` omitted: wallet must have a non-empty BTC address configured |
| | BTC address must be valid native SegWit format |
| Action | Fetches and displays on-chain BTC balance (confirmed, unconfirmed, total) |

### `bitcoin-wallet transfer`

| Requirement | Details |
|---|---|
| Flags | `--to` (required). At least one of: `--amount`, `--amount-mbtc`, `--amount-btc`. `--fee-rate`, `--wallet-id`, `--password` (optional) |
| Preconditions | Exactly one amount format must be provided |
| | Amount must be > 0 |
| | btc_wallet must be available (wallet must be created/imported from mnemonic) |
| | Destination must be valid native SegWit address (`bc1q...` / `tb1q...`) |
| | BTC balance must cover amount + fees |
| Action | Builds, signs, and broadcasts BTC transaction |

### `bitcoin-wallet receive`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| Action | Displays BTC receive address with QR code, network, derivation path, and registration status |

### `bitcoin-wallet update-bitcoin-wallet`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id`, `--password`, `--mnemonic` (all optional; mnemonic prompted if omitted) |
| Preconditions | Database features must be enabled |
| | Mnemonic must not be empty |
| | Current BTC address must **not** be registered on-chain (cannot change if registered) |
| | New BTC address derived from mnemonic must not be linked to a different twilight address |
| Action | Re-derives BTC wallet from new mnemonic, updates address and keys, persists to database |

### `bitcoin-wallet history`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id`, `--password`, `--status`, `--limit` (default: 50) (all optional) |
| Preconditions | Database features must be enabled |
| | Wallet must be loadable |
| Action | Displays BTC transfer history with confirmation counts |

---

## ZkOS Account Commands

### `zkaccount fund`

| Requirement | Details |
|---|---|
| Flags | At least one of: `--amount`, `--amount-mbtc`, `--amount-btc` (required). `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| | Amount must be > 0 |
| | Wallet must have sufficient on-chain sats balance |
| Action | Funds a new ZkOS trading account from the on-chain wallet |

### `zkaccount withdraw`

| Requirement | Details |
|---|---|
| Flags | `--account-index` (required), `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| | Account must exist and be `Coin` + `on_chain` |
| Action | Withdraws full balance from ZkOS account back to on-chain wallet |

### `zkaccount transfer`

| Requirement | Details |
|---|---|
| Flags | `--account-index` (required), `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| | Account must exist and be `Coin` + `on_chain` |
| Action | Transfers funds to a new ZkOS account (refreshes address for reuse after orders) |

### `zkaccount split`

| Requirement | Details |
|---|---|
| Flags | `--account-index` (required). At least one of: `--balances`, `--balances-mbtc`, `--balances-btc`. `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| | Account must exist and be `Coin` + `on_chain` |
| | Sum of split balances must not exceed account balance |
| | No zero-value balances allowed |
| Action | Splits account into multiple new accounts with specified balances |

---

## Order Commands

### `order open-trade`

| Requirement | Details |
|---|---|
| Flags | `--account-index`, `--side`, `--entry-price`, `--leverage` (required). `--order-type` (default: `MARKET`), `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| | Market must not be halted |
| | Account must be `Coin` + `on_chain` |
| | Account address must not have been used for a previous order (transfer first) |
| Action | Creates a trader order on the relayer. Sets account to `Memo` / `ORDERTX` |

### `order close-trade`

| Requirement | Details |
|---|---|
| Flags | `--account-index` (required). `--order-type` (default: `MARKET`), `--execution-price` (default: `0.0`), `--stop-loss`, `--take-profit`, `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| | Market must not be halted |
| | Order must be `FILLED` (if `SETTLED` or `LIQUIDATE`, auto-unlocks instead) |
| | `--order-type LIMIT` cannot be combined with `--stop-loss` or `--take-profit` |
| | If `--order-type LIMIT`: `--execution-price` must be > 0 |
| Action | **MARKET**: immediate close. **LIMIT**: sets settle_limit trigger. **SLTP** (via `--stop-loss`/`--take-profit`): sets SL/TP triggers |

### `order cancel-trade`

| Requirement | Details |
|---|---|
| Flags | `--account-index` (required). `--stop-loss`, `--take-profit`, `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| | Market must not be halted |
| | **Without flags**: order must be `PENDING` or have an active `settle_limit` (close limit). If pending, account is restored to `Coin`. If close limit, the settle_limit trigger is removed |
| | **With `--stop-loss` / `--take-profit`**: order must be `FILLED` and have active stop_loss or take_profit triggers. At least one trigger must exist on the order |
| Action | **Without flags**: cancels a pending order (restores to Coin) or removes a close limit on a filled order. **With flags**: cancels individual SL/TP triggers on a filled order without closing the position |

### `order query-trade`

| Requirement | Details |
|---|---|
| Flags | `--account-index` (required), `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| | Account must have an active order |
| Action | Queries trader order status via v1 endpoint (includes settle_limit, stop_loss, take_profit, funding_applied) |

### `order unlock-close-order`

| Requirement | Details |
|---|---|
| Flags | `--account-index` (required), `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| | Order must be `SETTLED` or `LIQUIDATE` (returns error otherwise) |
| Action | Fetches updated UTXO and restores account to `Coin` with new balance (initial margin +/- PnL) |

### `order unlock-failed-order`

| Requirement | Details |
|---|---|
| Flags | `--account-index` (required), `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| | Account must be stuck in `Memo` with no active order |
| Action | Fetches UTXO and restores account to `Coin` |

### `order open-lend`

| Requirement | Details |
|---|---|
| Flags | `--account-index` (required), `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| | Account must be `Coin` + `on_chain` |
| | Account address must not have been used for a previous order (transfer first) |
| Action | Deposits full account balance into lending pool. Sets account to `Memo` / `LENDTX` |

### `order close-lend`

| Requirement | Details |
|---|---|
| Flags | `--account-index` (required), `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| | Lend order must be `FILLED` |
| Action | Submits withdrawal request from lending pool |

### `order query-lend`

| Requirement | Details |
|---|---|
| Flags | `--account-index` (required), `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| | Account must have an active lend order |
| Action | Queries lend order status via v1 endpoint |

### `order history-trade`

| Requirement | Details |
|---|---|
| Flags | `--account-index` (required), `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable (signs the query) |
| Action | Queries historical trader orders from the relayer (not local DB) |

### `order history-lend`

| Requirement | Details |
|---|---|
| Flags | `--account-index` (required), `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable (signs the query) |
| Action | Queries historical lend orders from the relayer (not local DB) |

### `order funding-history`

| Requirement | Details |
|---|---|
| Flags | `--account-index` (required), `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| Action | Queries funding payment history for a position from the relayer |

### `order account-summary`

| Requirement | Details |
|---|---|
| Flags | `--from`, `--to`, `--since` (optional date filters), `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| Action | Queries trading activity summary (fills, settles, liquidations) from the relayer |

### `order tx-hashes`

| Requirement | Details |
|---|---|
| Flags | `--id` (required), `--by` (default: `request`), `--status`, `--limit`, `--offset`, `--reason` (optional) |
| Preconditions | **No wallet required** — queries relayer directly |
| | `--by` must be one of: `request`, `account`, `tx` |
| Action | Looks up on-chain transaction hashes by request ID, account address, or tx ID |

### `order request-history`

| Requirement | Details |
|---|---|
| Flags | `--account-index` (required), `--wallet-id`, `--password`, `--status`, `--limit`, `--offset`, `--reason` (optional) |
| Preconditions | Wallet must be loadable (resolves account address from wallet) |
| Action | Looks up transaction hashes for a wallet account by index (convenience wrapper around `tx-hashes --by account`) |

---

## History Commands

### `history orders`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id`, `--password` (required). `--account-index`, `--limit` (default: 50), `--offset` (default: 0) (optional) |
| Preconditions | Database features must be enabled |
| | Wallet must be loadable |
| Action | Displays order history (open, close, cancel events) from local database |

### `history transfers`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id`, `--password` (required). `--limit` (default: 50), `--offset` (default: 0) (optional) |
| Preconditions | Database features must be enabled |
| | Wallet must be loadable |
| Action | Displays transfer history (fund, withdraw, transfer events) from local database |

---

## Portfolio Commands

### `portfolio summary`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| | Relayer must be reachable (queries live positions) |
| Action | Shows full portfolio: balances, margin, PnL, open positions, lend positions. Auto-unlocks settled/liquidated accounts |

### `portfolio balances`

| Requirement | Details |
|---|---|
| Flags | `--unit` (default: `sats`), `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| Action | Shows per-account balance breakdown in chosen unit (sats, mbtc, btc) |

### `portfolio risks`

| Requirement | Details |
|---|---|
| Flags | `--wallet-id`, `--password` (optional) |
| Preconditions | Wallet must be loadable |
| | Relayer must be reachable (queries live price data) |
| Action | Shows liquidation risk for all open trader positions |

---

## Market Commands

All market commands query the relayer JSON-RPC API. **No wallet is needed.**

### `market price`

| Requirement | Details |
|---|---|
| Flags | None |
| Preconditions | Relayer must be reachable |
| Action | Gets current BTC/USD price |

### `market orderbook`

| Requirement | Details |
|---|---|
| Flags | None |
| Preconditions | Relayer must be reachable |
| Action | Gets current order book (open limit orders) |

### `market funding-rate`

| Requirement | Details |
|---|---|
| Flags | None |
| Preconditions | Relayer must be reachable |
| Action | Gets current funding rate |

### `market fee-rate`

| Requirement | Details |
|---|---|
| Flags | None |
| Preconditions | Relayer must be reachable |
| Action | Gets current fee rates |

### `market recent-trades`

| Requirement | Details |
|---|---|
| Flags | None |
| Preconditions | Relayer must be reachable |
| Action | Gets recent trade orders |

### `market position-size`

| Requirement | Details |
|---|---|
| Flags | None |
| Preconditions | Relayer must be reachable |
| Action | Gets aggregate long/short position sizes |

### `market lend-pool`

| Requirement | Details |
|---|---|
| Flags | None |
| Preconditions | Relayer must be reachable |
| Action | Gets lending pool information |

### `market pool-share-value`

| Requirement | Details |
|---|---|
| Flags | None |
| Preconditions | Relayer must be reachable |
| Action | Gets current pool share value |

### `market last-day-apy`

| Requirement | Details |
|---|---|
| Flags | None |
| Preconditions | Relayer must be reachable |
| Action | Gets last 24-hour APY |

### `market open-interest`

| Requirement | Details |
|---|---|
| Flags | None |
| Preconditions | Relayer must be reachable |
| Action | Gets open interest (long/short exposure) |

### `market market-stats`

| Requirement | Details |
|---|---|
| Flags | None |
| Preconditions | Relayer must be reachable |
| Action | Gets market risk statistics including funding rate and expected funding rate |

### `market server-time`

| Requirement | Details |
|---|---|
| Flags | None |
| Preconditions | Relayer must be reachable |
| Action | Gets relayer server UTC time |

### `market history-price`

| Requirement | Details |
|---|---|
| Flags | `--from`, `--to` (required). `--limit` (default: 50), `--offset` (default: 0) (optional) |
| Preconditions | Relayer must be reachable |
| Action | Queries historical BTC/USD prices over a date range |

### `market candles`

| Requirement | Details |
|---|---|
| Flags | `--since` (required). `--interval` (default: `1h`), `--limit` (default: 50), `--offset` (default: 0) (optional) |
| Preconditions | Relayer must be reachable |
| Action | Queries OHLCV candlestick data |

### `market history-funding`

| Requirement | Details |
|---|---|
| Flags | `--from`, `--to` (required). `--limit` (default: 50), `--offset` (default: 0) (optional) |
| Preconditions | Relayer must be reachable |
| Action | Queries historical funding rates |

### `market history-fees`

| Requirement | Details |
|---|---|
| Flags | `--from`, `--to` (required). `--limit` (default: 50), `--offset` (default: 0) (optional) |
| Preconditions | Relayer must be reachable |
| Action | Queries historical fee rates |

### `market apy-chart`

| Requirement | Details |
|---|---|
| Flags | `--range` (default: `7d`), `--step`, `--lookback` (optional) |
| Preconditions | Relayer must be reachable |
| Action | Queries APY chart data for the lend pool |

---

## Quick Reference

### Network Restrictions

| Network | Allowed Commands |
|---|---|
| **Mainnet only** | `register-btc`, `deposit-btc`, `deposit-status`, `withdraw-btc`, `withdraw-status` |
| **Testnet only** | `faucet` |
| **Both** | All other commands |

### Database Required

All commands except `wallet create`, `wallet lock`, and `bitcoin-wallet balance` (with `--btc-address`) require database features (`--features sqlite` or `--features postgresql`).

### BTC Wallet Availability

The `btc_wallet` (BIP-84 key material) is available only when the wallet was created or imported using a **mnemonic phrase**. If the wallet was created from a private key or a manual BTC address was set, the btc_wallet is `None`.

| btc_wallet available | btc_wallet NOT available |
|---|---|
| Auto-sends BTC to reserves | Must send BTC manually |
| Fee estimated automatically | User must account for fees |
| Balance checked including fees | Balance checked for amount only |

### Reserve Status Guide

| Status | Blocks Left | Safe to Send? |
|---|---|---|
| **ACTIVE** | > 72 | Yes |
| **WARNING** | 5 - 72 | Yes, if BTC tx confirms quickly |
| **CRITICAL** | <= 4 | **No** - do NOT send |
| **EXPIRED** | 0 | **No** - reserve is sweeping |

Reserves rotate every ~144 BTC blocks (~24 hours). The reserve must still be ACTIVE when your BTC transaction confirms on Bitcoin.

### Wallet Resolution Priority

For `--wallet-id` and `--password`, the CLI resolves values in this order:
1. CLI flag (`--wallet-id <id>`, `--password <pwd>`)
2. Environment variable (`NYKS_WALLET_ID`, `NYKS_WALLET_PASSPHRASE`)
3. Session cache (set via `wallet unlock`)
4. Interactive prompt (for `wallet unlock` only)
