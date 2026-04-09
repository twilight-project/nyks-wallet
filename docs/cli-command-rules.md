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
