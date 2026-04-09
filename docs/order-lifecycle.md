# ZkAccount, Trade & Lend Order Lifecycle

How ZkOS accounts are created, managed, and transition through states during trade and lend operations.

---

## ZkAccount Lifecycle

A ZkAccount is a privacy-preserving trading account on the ZkOS layer. Each account holds a balance, a cryptographic commitment (QuisQuis account), and state metadata.

### Account Fields

| Field        | Description                                                      |
| ------------ | ---------------------------------------------------------------- |
| `index`      | Unique integer identifier within the wallet                      |
| `balance`    | Balance in satoshis                                              |
| `qq_address` | Hex-encoded QuisQuis encrypted account (public key + commitment) |
| `account`    | Derived account address string (used for on-chain lookups)       |
| `scalar`     | Hex-encoded randomness scalar used in the ElGamal commitment     |
| `io_type`    | Current state: `Coin`, `Memo`, or `State`                        |
| `tx_type`    | Order type when in Memo: `ORDERTX`, `LENDTX`, or `None`          |
| `on_chain`   | Whether the account exists on the ZkOS chain                     |

### Full ZkAccount Lifecycle

```
                          ┌──────────────────────────────────────────┐
                          │           On-Chain Wallet                │
                          │         (Twilight address)               │
                          │        holds NYKS + SATS balance         │
                          └──────────┬───────────────────────────────┘
                                     │
                                     │  zkaccount fund --amount <sats>
                                     │  (MintBurnTradingBtc tx)
                                     ▼
                   ┌─────────────────────────────────────┐
                   │        ZkAccount [Coin, on_chain]    │
                   │   balance = funded amount             │
                   │   tx_type = None                      │
                   └──────┬──────────┬──────────┬─────────┘
                          │          │          │
          ┌───────────────┘          │          └──────────────────┐
          │                          │                             │
          ▼                          ▼                             ▼
    open-trade               open-lend                   zkaccount transfer
    ┌──────────┐             ┌──────────┐                ┌────────────────┐
    │  Memo    │             │  Memo    │                │ Old account:   │
    │  ORDERTX │             │  LENDTX  │                │   balance=0    │
    └──────────┘             └──────────┘                │   on_chain=F   │
          │                        │                     │ New account:   │
          │ (order lifecycle)      │ (order lifecycle)   │   balance=amt  │
          │                        │                     │   on_chain=T   │
          ▼                        ▼                     │   Coin state   │
    unlock-close-order       unlock-close-order          └────────────────┘
    ┌──────────┐             ┌──────────┐                        │
    │  Coin    │             │  Coin    │                        ▼
    │  ORDERTX │             │  LENDTX  │              (ready for new order
    └──────────┘             └──────────┘               or another transfer)
          │                        │
          │  Must transfer before  │
          │  reusing for new order │
          ▼                        ▼
    zkaccount transfer       zkaccount transfer
          │                        │
          ▼                        ▼
    (new Coin account,       (new Coin account,
     ready for orders)        ready for orders)
```

### Creating an Account: `zkaccount fund`

Moves sats from the on-chain wallet to a new ZkOS trading account.

1. Checks on-chain wallet has sufficient sats balance.
2. Generates a new ZkAccount with a deterministic key derived from the wallet seed + account index.
3. Creates a `MintBurnTradingBtc` transaction (mint = true) signed by the on-chain wallet.
4. Broadcasts the transaction to the Twilight chain.
5. Sets `on_chain = true` on the new account.

The account is now in **Coin** state, on-chain, and ready for orders or transfers.

```bash
relayer-cli zkaccount fund --amount 50000
relayer-cli zkaccount fund --amount-mbtc 0.5
relayer-cli zkaccount fund --amount-btc 0.0005
```

### Transferring Between Accounts: `zkaccount transfer`

Creates a new ZkOS account and privately transfers the full balance from an existing account. This is required after closing an order because the same account address cannot be reused.

1. Syncs the source account state (fetches latest UTXO).
2. Generates a new ZkAccount with the same balance.
3. Creates a ZkOS private transfer transaction (QuisQuis protocol).
4. Broadcasts the transaction.
5. Fetches the new UTXO for the destination account.
6. Marks old account: `on_chain = false`, `balance = 0`.
7. Marks new account: `on_chain = true`, with updated QQ address and scalar.

```bash
relayer-cli zkaccount transfer --account-index 1
```

### Splitting an Account: `zkaccount split`

Splits one account into multiple new accounts with specified balances. Useful for creating multiple trading positions from a single funded account.

1. Validates sender has sufficient balance for sum of all requested balances.
2. Generates N new ZkAccounts (max 8 per call due to tx size limits).
3. Creates a single ZkOS multi-receiver private transfer transaction.
4. Broadcasts the transaction.
5. Fetches UTXOs for all new accounts and updates the sender.

```bash
relayer-cli zkaccount split --account-index 0 --balances "10000,20000,30000"
relayer-cli zkaccount split --account-index 0 --balances-mbtc "0.1,0.2,0.3"
```

### Withdrawing to On-Chain Wallet: `zkaccount withdraw`

Moves funds from a ZkOS trading account back to the on-chain wallet.

1. Transfers to a fresh account first (via `trading_to_trading`).
2. Creates a burn transaction on ZkOS (destroys the ZkOS account).
3. Waits for the UTXO to be removed from ZkOS.
4. Creates a `MintBurnTradingBtc` transaction (mint = false) to credit the on-chain wallet.
5. Marks the account: `on_chain = false`, `balance = 0`.

```bash
relayer-cli zkaccount withdraw --account-index 1
```

### Account Address Reuse Constraint

A ZkOS account address can only be used for **one order**. After an order is closed and unlocked, the account retains its balance but the address is "spent". You must transfer to get a fresh address before placing a new order:

```
open-trade → close-trade → unlock-close-order → transfer → open-trade (new address)
```

This is a fundamental property of the QuisQuis privacy protocol — each transaction produces new output accounts with fresh commitments.

---

## Account State Model

Each ZkOS account tracks three key fields:

| Field      | Values                   | Description                                   |
| ---------- | ------------------------ | --------------------------------------------- |
| `io_type`  | `Coin`, `Memo`, `State`  | Current account state on the ZkOS layer       |
| `tx_type`  | `ORDERTX`, `LENDTX`, `-` | Type of active order (set when entering Memo) |
| `on_chain` | `true`, `false`          | Whether the account exists on-chain           |

- **Coin** — idle, available for new orders or transfers.
- **Memo** — locked in an active order (trade or lend). `tx_type` indicates which kind.
- **State** — on-chain initialization state (used during account setup).

## Trade Order Lifecycle

```
  Coin (ORDERTX: None)
    │
    ▼  open-trade
  Memo (ORDERTX)
    │
    ├─── Order fills ──► FILLED (position is live)
    │                       │
    │                       ├── close-trade ──► SETTLED ──► unlock-close-order ──► Coin
    │                       │
    │                       ├── SLTP triggered ──► SETTLED ──► unlock-close-order ──► Coin
    │                       │
    │                       └── Liquidation ──► LIQUIDATE ──► unlock-close-order ──► Coin
    │
    ├─── Order rejected/failed ──► unlock-failed-order ──► Coin
    │
    └─── cancel-trade (if PENDING) ──► Coin
```

### Steps

1. **`open-trade`** — Account must be `Coin` + `on_chain`. Creates a trader order on the relayer. Sets `io_type = Memo`, `tx_type = ORDERTX`.

2. **Order fills** — The relayer matches the order. Status moves to `FILLED`. The position is now live with entry price, leverage, and margin.

3. **`close-trade`** — Submits a close request (MARKET, LIMIT, or SLTP). The relayer settles the position and computes PnL.

4. **`unlock-close-order`** — After settlement (`SETTLED` or `LIQUIDATE`), fetches the updated UTXO and restores the account to `Coin` state with the new balance (initial margin +/- PnL). Clears `tx_type`.

5. **`cancel-trade`** — Only works while order is `PENDING` (not yet filled). Restores account to `Coin`.

6. **`unlock-failed-order`** — If the order submission itself failed (relayer rejected it, network error, etc.), the account is stuck in `Memo` but has no active order. This command fetches the UTXO and restores to `Coin`.

### SLTP (Stop-Loss / Take-Profit)

When closing with `--stop-loss` or `--take-profit`, the relayer monitors the position and settles automatically when the trigger price is hit. The account remains in `Memo` until settlement, after which `unlock-close-order` reclaims it.

### Important

After an order is closed and unlocked, the account address has been used. **You must transfer the account before placing a new order** — the same account address cannot be reused for a new order:

```bash
relayer-cli zkaccount transfer --account-index <N>
```

## Lend Order Lifecycle

```
  Coin (LENDTX: None)
    │
    ▼  open-lend
  Memo (LENDTX)
    │
    ├─── Order fills ──► FILLED (funds in lending pool)
    │                       │
    │                       └── close-lend ──► SETTLED ──► unlock-close-order ──► Coin
    │
    └─── Order rejected/failed ──► unlock-failed-order ──► Coin
```

### Steps

1. **`open-lend`** — Account must be `Coin` + `on_chain`. Creates a lend order on the relayer. Sets `io_type = Memo`, `tx_type = LENDTX`. The full account balance is deposited into the lending pool.

2. **Order fills** — The relayer accepts the deposit. Status moves to `FILLED`. The funds earn yield from the DeFi pool based on pool share.

3. **`close-lend`** — Submits a withdrawal request. The relayer settles and returns the deposit plus any accrued interest/payment.

4. **`unlock-close-order`** — After settlement (`SETTLED`), fetches the updated UTXO and restores the account to `Coin` with the new balance (deposit + yield). Clears `tx_type`.

5. **`unlock-failed-order`** — Same as for trades — reclaims the account if the lend submission failed.

### Lend Position Metrics

| Metric       | Description                                |
| ------------ | ------------------------------------------ |
| `deposit`    | Original amount deposited into the pool    |
| `balance`    | Current value (deposit + accrued yield)    |
| `npoolshare` | Fractional share of the total lending pool |
| `payment`    | Cumulative interest/yield payment received |
| `apr`        | Annualised percentage rate (from v1 API)   |

## How `unlock-close-order` Detects Order Type

When an account is in `Memo` state, `tx_type` tells the CLI which unlock path to use:

| `tx_type` | Action                                                                                                             |
| --------- | ------------------------------------------------------------------------------------------------------------------ |
| `ORDERTX` | Calls `unlock_trader_order` — queries trader order, verifies `SETTLED`/`LIQUIDATE`, fetches UTXO, restores to Coin |
| `LENDTX`  | Calls `unlock_lend_order` — queries lend order, verifies `SETTLED`, fetches UTXO, restores to Coin                 |
| `None`    | Falls back to trader order unlock (backward compatibility with accounts created before `tx_type` was added)        |

## Portfolio Summary Integration

The portfolio scan (`portfolio summary`) automatically handles settled orders:

- **Settled trade orders** (`ORDERTX` + `SETTLED`/`LIQUIDATE`) are auto-unlocked and appear under "Closed Positions" or "Liquidated Positions".
- **Settled lend orders** (`LENDTX` + `SETTLED`) are auto-unlocked and appear under "Closed Lend Positions".
- Active positions appear under "Trader Positions" or "Lend Positions".
- Accounts in `Coin` state contribute to `total_trading_balance`.

## Order Status Reference

| Status      | Description                                       |
| ----------- | ------------------------------------------------- |
| `PENDING`   | Order submitted, waiting to be filled             |
| `FILLED`    | Order matched/accepted, position is active        |
| `SETTLED`   | Position closed and settled, ready to unlock      |
| `LIQUIDATE` | Position liquidated (trade only), ready to unlock |
| `CANCELLED` | Order was cancelled before filling                |

## CLI Quick Reference

```bash
# Trade lifecycle
relayer-cli order open-trade --account-index 1 --side LONG --entry-price 65000 --leverage 5
relayer-cli order query-trade --account-index 1
relayer-cli order close-trade --account-index 1
relayer-cli order unlock-close-order --account-index 1
relayer-cli zkaccount transfer --account-index 1    # before reusing

# Lend lifecycle
relayer-cli order open-lend --account-index 2
relayer-cli order query-lend --account-index 2
relayer-cli order close-lend --account-index 2
relayer-cli order unlock-close-order --account-index 2
relayer-cli zkaccount transfer --account-index 2    # before reusing

# Recovery
relayer-cli order unlock-failed-order --account-index 3

# Check account states
relayer-cli wallet accounts
```
