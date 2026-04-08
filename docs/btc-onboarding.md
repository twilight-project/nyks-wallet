# BTC Onboarding: Bitcoin Network to Twilight Protocol

How to bring Bitcoin from the Bitcoin network into the Twilight protocol, deposit it into reserves, trade with it, and withdraw it back to Bitcoin.

---

## Overview

```
 Bitcoin Network                    Twilight Protocol
 ~~~~~~~~~~~~~~                    ~~~~~~~~~~~~~~~~~~

 +--------------+    register-btc    +-------------------+
 | Your BTC     | -----------------> | On-chain wallet   |
 | Wallet       |    deposit-btc     | (NYKS + SATS)     |
 | (bc1q...)    | ======BTC=======>  |                   |
 +--------------+                    +--------+----------+
       ^                                      |
       |                                      | zkaccount fund
       |                                      v
       |                             +-------------------+
       |    withdraw-btc             | ZkOS Trading      |
       +<--------------------------- | Account (Coin)    |
                                     | trade / lend      |
                                     +-------------------+
```

The flow has four phases:

1. **Registration** -- link your BTC address to your Twilight wallet on-chain
2. **Deposit** -- send BTC to a reserve address; validators credit your Twilight wallet
3. **Trading** -- fund ZkOS accounts, open trade/lend orders (see [order-lifecycle.md](order-lifecycle.md))
4. **Withdrawal** -- request BTC back to your registered Bitcoin address

---

## Prerequisites

| Requirement | How to check | How to set up |
|---|---|---|
| Twilight wallet | `wallet info` | `wallet create` or `wallet import` |
| BTC address configured | `bitcoin-wallet receive` | Set during create/import, or `wallet update-btc-address` |
| Mainnet network | Check `NETWORK_TYPE` in `.env` | `NETWORK_TYPE=mainnet` |
| Sufficient BTC balance | `bitcoin-wallet balance` | Send BTC to your address (shown by `bitcoin-wallet receive`) |
| BTC wallet (for auto-pay) | `bitcoin-wallet receive` shows "available (keys loaded)" | Create/import wallet from mnemonic |

> **BTC wallet vs BTC address:** Every wallet has a BTC address. But the BTC wallet (private keys for signing Bitcoin transactions) is only available when the wallet was created or imported from a **mnemonic phrase**. If you set a manual BTC address or imported from a private key, you must send BTC manually.

> **Don't want to share your mnemonic?** You don't need a BTC wallet for the full flow to work. Instead, set your own BTC address (one you control in any external wallet) and handle payments manually:
>
> ```bash
> # Set your external BTC address
> relayer-cli wallet update-btc-address --btc-address bc1q<your_address>
>
> # Register and deposit in one step -- the CLI will show reserve addresses
> # to send BTC to manually from your external wallet
> relayer-cli wallet register-btc --amount 50000
> ```
>
> This way your private keys never leave your hardware wallet or external software. The only trade-off is that you must copy-paste reserve addresses and send BTC yourself instead of the CLI doing it automatically.

---

## Phase 1: Registration

Register your BTC deposit address on the Twilight chain. This is a one-time step per BTC address.

```bash
relayer-cli wallet register-btc --amount 50000
```

### What happens

1. Checks your BTC address isn't already registered
2. Fetches reserve addresses and verifies at least one is ACTIVE
3. If BTC wallet available: estimates the real transaction fee and checks your balance covers `amount + fee`
4. Submits a registration transaction to the Twilight chain
5. **Auto-pay (BTC wallet available):** Automatically sends the deposit amount to the best reserve (latest expiry). Done -- skip to Phase 3.
6. **Manual (no BTC wallet):** Shows the reserve address(es) to send BTC to. Proceed to Phase 2.

### Reserve selection

The CLI picks the **best reserve** -- the one with the most blocks remaining before expiry. This maximizes the time window for your Bitcoin transaction to confirm.

If only one active reserve exists, it is automatically selected even in manual mode.

### Checking registration status

```bash
# Shows BTC address details, registration status, and QR code
relayer-cli bitcoin-wallet receive

# Or check via wallet info (no chain call, shows "BTC registered: true/false")
relayer-cli wallet info
```

---

## Phase 2: Deposit (additional deposits after registration)

If `register-btc` auto-paid to the reserve, your first deposit is already done — skip to Phase 3. Use `deposit-btc` for subsequent deposits to the same registered address:

```bash
# Auto-selects best reserve, auto-sends if BTC wallet available
relayer-cli wallet deposit-btc --amount 50000

# Or specify a reserve explicitly
relayer-cli wallet deposit-btc --amount 50000 --reserve-address bc1q...

# Amount in other units
relayer-cli wallet deposit-btc --amount-mbtc 0.5
relayer-cli wallet deposit-btc --amount-btc 0.0005
```

### What happens

1. Verifies your BTC address is registered on-chain
2. Resolves the target reserve (explicit `--reserve-address` or best available)
3. **BTC wallet available:** Estimates fee, checks balance, sends BTC automatically. Deposit saved as `sent`.
4. **No BTC wallet:** Shows the reserve address. You must send BTC manually from your registered address. Deposit saved as `pending`.

### Important rules

- **Send ONLY from your registered BTC address.** Bitcoin sent from any other address will not be credited.
- **Send to an ACTIVE reserve only.** The reserve must still be active when your BTC transaction confirms (~10 min).
- **Reserve IDs are saved** in the deposit record for use during withdrawals.

### Checking reserves

```bash
relayer-cli wallet reserves
```

Shows all reserves with status and a QR code for the recommended one. Status key:

| Status | Blocks Left | Safe? |
|---|---|---|
| ACTIVE | > 72 | Yes |
| WARNING | 5--72 | Only if tx confirms quickly |
| CRITICAL | <= 4 | No |
| EXPIRED | 0 | No |

Reserves rotate every ~144 BTC blocks (~24 hours).

---

## Phase 3: Confirmation

After BTC is sent to a reserve, wait for:

1. **Bitcoin confirmation** -- ~10 minutes for 1 confirmation
2. **Validator confirmation** -- validators detect the deposit and credit your Twilight wallet (can take 1+ hours)

```bash
# Check deposit and withdrawal status
relayer-cli wallet deposit-status
```

This command shows:
- Confirmed deposits (from the Twilight indexer)
- Pending deposits (from local DB, not yet confirmed on-chain)
- Auto-updates local records when a pending deposit is confirmed

Once confirmed, your on-chain SATS balance increases:

```bash
relayer-cli wallet balance
```

---

## Phase 4: Trading

With SATS in your on-chain wallet, fund a ZkOS trading account:

```bash
relayer-cli zkaccount fund --amount 50000
```

Then trade or lend. See [order-lifecycle.md](order-lifecycle.md) for the full trading flow.

---

## Phase 5: Withdrawal

To withdraw BTC back to the Bitcoin network, you must first have SATS in your on-chain wallet. If your funds are in a ZkOS trading account, withdraw them first:

```bash
relayer-cli zkaccount withdraw --account-index 0
```

### Step 1: Know your reserve ID

The reserve ID is saved in your deposit records. You can also find it via:

```bash
relayer-cli wallet reserves
relayer-cli wallet deposit-status
```

### Step 2: Submit withdrawal

```bash
relayer-cli wallet withdraw-btc --reserve-id 1 --amount 50000
```

BTC is always withdrawn to your registered BTC address (the same `bc1q...` used for deposits).

### Step 3: Check status

```bash
relayer-cli wallet withdraw-status
```

Shows all withdrawals with confirmation status. The command auto-updates local records when validators confirm the withdrawal.

---

## Complete Example

```bash
# 1. Create wallet (from mnemonic for auto-pay support)
relayer-cli wallet import

# 2. Check your BTC address and QR code
relayer-cli bitcoin-wallet receive

# 3. Send BTC to your address (external wallet or exchange)
#    Then verify balance:
relayer-cli bitcoin-wallet balance

# 4. Register and auto-deposit (one command if BTC wallet available)
relayer-cli wallet register-btc --amount 50000

# 5. Wait for confirmation
relayer-cli wallet deposit-status

# 6. Check Twilight balance
relayer-cli wallet balance

# 7. Fund a trading account
relayer-cli zkaccount fund --amount 50000

# 8. Trade!
relayer-cli order open-trade --account-index 0 --side LONG --entry-price 65000 --leverage 5

# ... later ...

# 9. Close and withdraw
relayer-cli order close-trade --account-index 0
relayer-cli order unlock-close-order --account-index 0
relayer-cli zkaccount withdraw --account-index 0
relayer-cli wallet withdraw-btc --reserve-id 1 --amount 50000
relayer-cli wallet withdraw-status
```

---

## Troubleshooting

### "BTC address is already registered to your wallet"
Your address is already registered. Check `wallet deposit-status` to see if a deposit is in progress, or use `wallet deposit-btc` for a new deposit.

### "All reserves are expired or critical"
Wait for the next reserve rotation (~144 BTC blocks). The error shows an ETA.

### "Insufficient BTC balance"
Send more BTC to your address. The error shows the exact shortfall including estimated fees.

### "BTC wallet not available"
Your wallet was created from a private key or manual address, not a mnemonic. You must send BTC manually. To get auto-pay, re-import with a mnemonic:
```bash
relayer-cli bitcoin-wallet update-bitcoin-wallet --mnemonic "<phrase>"
```

### Deposit shows "pending" for a long time
1. Ensure BTC was sent to an ACTIVE reserve (not expired)
2. Check the Bitcoin transaction has at least 1 confirmation
3. Validator confirmation can take 1+ hours
4. Run `wallet deposit-status` periodically to refresh

### "Failed to estimate fee"
The CLI extracts the fee from BDK's error even when the wallet has insufficient funds. For other errors (e.g., network issues syncing UTXOs), it falls back to a 2,000 sat buffer. If your funds just arrived and aren't confirmed yet, wait for 1 confirmation and retry.

---

## BTC Wallet Management

### Viewing your address

```bash
relayer-cli bitcoin-wallet receive
```

Shows address details and a scannable QR code.

### Checking balance

```bash
relayer-cli bitcoin-wallet balance
relayer-cli bitcoin-wallet balance --btc     # display in BTC
relayer-cli bitcoin-wallet balance --mbtc    # display in mBTC
```

### Transferring BTC (between Bitcoin addresses)

```bash
relayer-cli bitcoin-wallet transfer --to bc1q... --amount 50000
```

### Updating BTC wallet keys

If you need to change the mnemonic backing your BTC wallet:

```bash
relayer-cli bitcoin-wallet update-bitcoin-wallet --mnemonic "<new phrase>"
```

**Restriction:** Cannot update if the current BTC address is already registered on-chain.

### Transfer history

```bash
relayer-cli bitcoin-wallet history
relayer-cli bitcoin-wallet history --status confirmed
```
