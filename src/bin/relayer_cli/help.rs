// ---------------------------------------------------------------------------
// Help
// ---------------------------------------------------------------------------

pub(crate) fn print_global_help() {
    println!(
        r#"Twilight Relayer CLI — manage wallets and orders from the command line.

USAGE:
    relayer-cli [--json] <COMMAND>

COMMANDS:
    wallet          Wallet management (create, import, load, list, balance, accounts,
                    export, backup, restore, unlock/lock, change-password, info,
                    update-btc-address, sync-nonce, send, register-btc, deposit-btc,
                    reserves, deposit-status, withdraw-btc, withdraw-status, faucet)
    bitcoin-wallet  On-chain BTC operations (balance, transfer, receive, history)
    zkaccount       ZkOS account operations (fund, withdraw, transfer, split)
    order           Trading and lending orders (open/close/cancel/query trade & lend,
                    unlock-close-order, unlock-failed-order, history-trade,
                    history-lend, funding-history, account-summary, tx-hashes)
    market          Market data (price, orderbook, funding-rate, fee-rate, recent-trades,
                    position-size, lend-pool, pool-share-value, last-day-apy,
                    open-interest, market-stats, server-time, history-price,
                    candles, history-funding, history-fees, apy-chart)
    history         Local DB history (orders, transfers)
    portfolio       Portfolio tracking (summary, balances, risks)
    verify-test     Run verification tests against testnet (testnet only)

GLOBAL FLAGS:
    --json      Output results as JSON (for scripting)

RESOLUTION PRIORITY (wallet-id & password):
    --flag  >  session cache (wallet unlock)  >  env var

ENVIRONMENT:
    NYKS_WALLET_ID          Default wallet ID
    NYKS_WALLET_PASSPHRASE  Default password
    BTC_NETWORK_TYPE        Bitcoin network (mainnet/testnet, falls back to mainnet)

Run `relayer-cli help <COMMAND>` for details on a specific command group."#
    );
}

pub(crate) fn print_wallet_help() {
    println!(
        r#"Wallet management commands.

USAGE:
    relayer-cli wallet <SUBCOMMAND>

SUBCOMMANDS:
    create              Create a new wallet (persisted to DB)
    import              Import wallet from BIP-39 mnemonic
    load                Load a wallet from the database
    list                List all wallets in the database
    balance             Show wallet balance (on-chain query)
    info                Show wallet info (no chain calls)
    accounts            List all ZkOS accounts for a wallet (INDEX, BALANCE, ON-CHAIN, IO-TYPE, TX-TYPE, ACCOUNT)
    export              Export wallet to a JSON file
    backup              Full database backup to JSON
    restore             Restore wallet from backup JSON
    unlock              Cache wallet-id + password for this terminal session
    lock                Clear cached session
    change-password     Change the DB encryption password
    update-btc-address  Update the BTC deposit address
    sync-nonce          Sync nonce/sequence from chain state
    send                Send tokens (nyks or sats) to a Twilight address
    register-btc        Register BTC deposit address on-chain (mainnet only)
    deposit-btc         Record a BTC deposit after registration (mainnet only)
    reserves            Show available BTC reserve addresses
    deposit-status      Check BTC deposit & confirmation status (mainnet only)
    withdraw-btc        Submit a BTC withdrawal request (mainnet only)
    withdraw-status     Check pending BTC withdrawal status (mainnet only)
    faucet              Get test tokens from faucet (testnet only)

EXAMPLES:
    relayer-cli wallet create --btc-address bc1q...
    relayer-cli wallet unlock                         # interactive prompt
    relayer-cli wallet balance                        # uses session cache
    relayer-cli wallet accounts --on-chain-only
    relayer-cli wallet send --to twilight1... --amount 1000
    relayer-cli wallet register-btc --amount 50000    # mainnet: register for 50k sats deposit
    relayer-cli wallet deposit-btc --amount 50000 --reserve-address bc1q...
    relayer-cli wallet reserves                       # see where to send BTC
    relayer-cli wallet deposit-status                 # check if confirmed by validators
    relayer-cli wallet withdraw-btc --reserve-id 1 --amount 50000
    relayer-cli wallet withdraw-status                # check pending withdrawals
    relayer-cli wallet faucet                         # testnet only: get test tokens"#
    );
}

pub(crate) fn print_zkaccount_help() {
    println!(
        r#"ZkOS account operations — fund, withdraw, transfer, and split trading accounts.

USAGE:
    relayer-cli zkaccount <SUBCOMMAND>

SUBCOMMANDS:
    fund        Fund a new ZkOS trading account from on-chain wallet
    withdraw    Withdraw from ZkOS account back to on-chain wallet
    transfer    Transfer balance between ZkOS trading accounts
    split       Split one account into multiple new accounts

AMOUNTS:
    fund/split accept amounts in multiple units (pick one):
        --amount <sats>           Satoshis
        --amount-mbtc <mbtc>      Milli-BTC (1 mBTC = 100,000 sats)
        --amount-btc <btc>        BTC (1 BTC = 100,000,000 sats)

EXAMPLES:
    relayer-cli zkaccount fund --amount 50000
    relayer-cli zkaccount withdraw --account-index 1
    relayer-cli zkaccount transfer --account-index 1
    relayer-cli zkaccount split --account-index 0 --balances "10000,20000,30000""#
    );
}

pub(crate) fn print_order_help() {
    println!(
        r#"Trading and lending order commands.

USAGE:
    relayer-cli order <SUBCOMMAND>

TRADING:
    open-trade          Open a leveraged position (MARKET/LIMIT)
    close-trade         Close a position (MARKET/LIMIT/SLTP)
    cancel-trade        Cancel a pending order
    query-trade         Query current order status
    unlock-close-order  Unlock a settled order (trade or lend) based on account's TXType
    unlock-failed-order Unlock a failed order (reclaim account when submission failed)

LENDING:
    open-lend           Open a lend order
    close-lend          Close a lend order
    query-lend          Query lend order status

HISTORY & ANALYTICS (from relayer):
    history-trade       Historical trader orders for an account
    history-lend        Historical lend orders for an account
    funding-history     Funding payment history for a position
    account-summary     Trading activity summary (fills, settles, liquidations)
    tx-hashes           Look up on-chain tx hashes by request/account ID

EXAMPLES:
    relayer-cli order open-trade --account-index 1 --side LONG --entry-price 65000 --leverage 5
    relayer-cli order close-trade --account-index 1
    relayer-cli order query-trade --account-index 1
    relayer-cli order history-trade --account-index 1
    relayer-cli order account-summary --from 2024-01-01 --to 2024-12-31

NOTE:
    If the account was previously used for a open/closed order. You must transfer the account
    first before placing a new order, as an order cannot be placed with the same
    account address twice. If the order was pending to fill and later cancelled, you can reuse the
    account.
    Use: relayer-cli zkaccount transfer --account-index <index>"#
    );
}

pub(crate) fn print_market_help() {
    println!(
        r#"Market data queries (no wallet required).

USAGE:
    relayer-cli market <SUBCOMMAND>

LIVE DATA:
    price               Current BTC/USD price
    orderbook           Open limit orders
    funding-rate        Current funding rate
    fee-rate            Current fee rate
    recent-trades       Recent trade orders
    position-size       Position size summary
    lend-pool           Lend pool info
    pool-share-value    Current pool share value
    last-day-apy        Last 24h annualized yield
    open-interest       Long/short exposure
    market-stats        Comprehensive risk statistics
    server-time         Relayer server time

HISTORICAL DATA:
    history-price       Historical prices over a date range
    candles             OHLCV candlestick data
    history-funding     Historical funding rates
    history-fees        Historical fee rates
    apy-chart           APY chart data for lend pool

EXAMPLES:
    relayer-cli market price
    relayer-cli market candles --interval 1h --since 2024-01-01
    relayer-cli market history-price --from 2024-01-01 --to 2024-01-31
    relayer-cli market apy-chart --range 30d --step 1d"#
    );
}

pub(crate) fn print_history_help() {
    println!(
        r#"Local database history queries (requires DB feature).

USAGE:
    relayer-cli history <SUBCOMMAND>

SUBCOMMANDS:
    orders      Show order history (open, close, cancel events)
    transfers   Show transfer history (fund, withdraw, transfer events)

EXAMPLES:
    relayer-cli history orders --limit 20
    relayer-cli history transfers --limit 10"#
    );
}

pub(crate) fn print_portfolio_help() {
    println!(
        r#"Portfolio and position tracking.

USAGE:
    relayer-cli portfolio <SUBCOMMAND>

SUBCOMMANDS:
    summary     Full portfolio summary (balances, positions, PnL)
    balances    Per-account balance breakdown (--unit sats|mbtc|btc)
    risks       Liquidation risk for open positions

EXAMPLES:
    relayer-cli portfolio summary
    relayer-cli portfolio balances --unit btc
    relayer-cli portfolio risks"#
    );
}

pub(crate) fn print_bitcoin_wallet_help() {
    println!(
        r#"On-chain Bitcoin operations — check balance, transfer BTC, view receive address, and transfer history.

USAGE:
    relayer-cli bitcoin-wallet <SUBCOMMAND>

SUBCOMMANDS:
    balance     Check on-chain BTC balance (confirmed + unconfirmed)
    transfer    Send BTC to a native SegWit address
    receive     Show BTC receive address and wallet details
    history     Show BTC transfer history with confirmation status

AMOUNTS (transfer):
    --amount <sats>           Satoshis (priority 1)
    --amount-mbtc <mbtc>      Milli-BTC — 1 mBTC = 100,000 sats (priority 2)
    --amount-btc <btc>        BTC — 1 BTC = 100,000,000 sats (priority 3)

DISPLAY UNIT (balance):
    --btc                     Show balance in BTC
    --mbtc                    Show balance in mBTC
    (default: sats)

EXAMPLES:
    relayer-cli bitcoin-wallet balance
    relayer-cli bitcoin-wallet balance --btc
    relayer-cli bitcoin-wallet balance --btc-address bc1q...
    relayer-cli bitcoin-wallet transfer --to bc1q... --amount 50000
    relayer-cli bitcoin-wallet transfer --to bc1q... --amount-mbtc 0.5 --fee-rate 5
    relayer-cli bitcoin-wallet receive
    relayer-cli bitcoin-wallet history
    relayer-cli bitcoin-wallet history --status confirmed"#
    );
}

pub(crate) fn print_verify_test_help() {
    println!(
        r#"Run verification tests against testnet (testnet only).

USAGE:
    relayer-cli verify-test <SUBCOMMAND>

SUBCOMMANDS:
    wallet      Verify wallet commands (create, balance, faucet, send, etc.)
    market      Verify market data queries
    zkaccount   Verify ZkOS account commands (requires funded wallet)
    order       Verify order commands (requires funded ZkOS account)
    all         Run all verification tests in sequence

EXAMPLES:
    NETWORK_TYPE=testnet relayer-cli verify-test all
    NETWORK_TYPE=testnet relayer-cli verify-test wallet
    NETWORK_TYPE=testnet relayer-cli verify-test market"#
    );
}

pub(crate) fn print_subcommand_help(group: &str) {
    match group.to_lowercase().replace('-', "").as_str() {
        "wallet" => print_wallet_help(),
        "bitcoinwallet" => print_bitcoin_wallet_help(),
        "zkaccount" => print_zkaccount_help(),
        "order" => print_order_help(),
        "market" => print_market_help(),
        "history" => print_history_help(),
        "portfolio" => print_portfolio_help(),
        "verifytest" => print_verify_test_help(),
        _ => {
            eprintln!("Unknown command group: '{}'\n", group);
            print_global_help();
        }
    }
}
