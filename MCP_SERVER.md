# nyks-wallet MCP Server

The `nyks-wallet-mcp` binary exposes the full nyks-wallet feature set as an
[MCP (Model Context Protocol)](https://modelcontextprotocol.io) server.  An AI
assistant such as Claude can use it to create wallets, execute trades, query
market data, and manage a portfolio — all through natural-language conversation.

---

## How it works

The server speaks **MCP 2024-11-05** over **stdio** (line-delimited JSON-RPC
2.0).  The AI client launches the process, exchanges `initialize` /
`tools/list` messages, and then calls tools by name.

```
Claude ──JSON-RPC──▶  nyks-wallet-mcp (stdin)
Claude ◀──JSON-RPC──  nyks-wallet-mcp (stdout)
                          │
                    ┌─────┴──────┐
                    │  nyks-wallet│
                    │  library   │
                    └─────┬──────┘
                          │
              ┌───────────┴────────────┐
              │                        │
        Twilight node           Relayer API
        (LCD / RPC)             (JSON-RPC)
```

All log/debug output is written to **stderr** and never pollutes the protocol
stream.

---

## Quick start

### 1 – Build the binary

```bash
# Requires: Rust 1.86+, protobuf-compiler, libpq-dev
cargo build --release --bin nyks-wallet-mcp --features order-wallet
```

The binary is placed at `target/release/nyks-wallet-mcp`.

### 2 – Configure Claude Desktop (or any MCP client)

Add the server to your MCP client configuration.  For Claude Desktop, edit
`~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "nyks-wallet": {
      "command": "/path/to/nyks-wallet-mcp",
      "env": {
        "NYKS_LCD_BASE_URL": "http://localhost:1317",
        "NYKS_RPC_BASE_URL": "http://localhost:26657",
        "RELAYER_API_RPC_SERVER_URL": "http://localhost:8088/api",
        "ZKOS_SERVER_URL": "http://localhost:3030",
        "CHAIN_ID": "nyks"
      }
    }
  }
}
```

On Linux the config lives at `~/.config/Claude/claude_desktop_config.json`.

### 3 – Start a conversation

Once the server is registered, ask Claude anything:

> "Create a new wallet for me."
> "What's the current BTC price?"
> "Open a 10× long position at $65,000 with my wallet."
> "Show me my portfolio summary."

---

## Running with Docker

### Build the image

```bash
docker build -f Dockerfile.mcp -t nyks-wallet-mcp .
```

### Wire it into Claude Desktop

```json
{
  "mcpServers": {
    "nyks-wallet": {
      "command": "docker",
      "args": [
        "run", "--rm", "-i",
        "--volume", "nyks-wallet-data:/home/mcp/data",
        "-e", "NYKS_LCD_BASE_URL=http://host.docker.internal:1317",
        "-e", "NYKS_RPC_BASE_URL=http://host.docker.internal:26657",
        "-e", "RELAYER_API_RPC_SERVER_URL=http://host.docker.internal:8088/api",
        "-e", "ZKOS_SERVER_URL=http://host.docker.internal:3030",
        "-e", "CHAIN_ID=nyks",
        "nyks-wallet-mcp"
      ]
    }
  }
}
```

The named volume `nyks-wallet-data` persists the SQLite wallet database across
container restarts.

### Pass credentials securely at runtime

Never bake `NYKS_WALLET_PASSPHRASE` into an image.  Pass it at runtime:

```bash
docker run --rm -i \
  -e NYKS_WALLET_PASSPHRASE="my-secret" \
  -e NYKS_WALLET_ID="my_wallet" \
  --volume nyks-wallet-data:/home/mcp/data \
  nyks-wallet-mcp
```

---

## Environment variables

| Variable | Default | Description |
|---|---|---|
| `NYKS_LCD_BASE_URL` | `http://0.0.0.0:1317` | Cosmos LCD endpoint |
| `NYKS_RPC_BASE_URL` | `http://0.0.0.0:26657` | Tendermint RPC endpoint |
| `RELAYER_API_RPC_SERVER_URL` | `http://0.0.0.0:8088/api` | Twilight relayer JSON-RPC |
| `ZKOS_SERVER_URL` | `http://0.0.0.0:3030` | ZkOS server endpoint |
| `FAUCET_BASE_URL` | `http://0.0.0.0:6969` | Testnet faucet |
| `CHAIN_ID` | `nyks` | Chain identifier |
| `NETWORK_TYPE` | `mainnet` | `mainnet` or `testnet` |
| `DATABASE_URL` | `wallets.db` | SQLite path (or PostgreSQL URL) |
| `NYKS_WALLET_PASSPHRASE` | *(empty)* | Pre-set wallet password (optional) |
| `NYKS_WALLET_ID` | *(empty)* | Default wallet ID (optional) |
| `RUST_LOG` | `error` | Log level (output goes to stderr) |

---

## Session password

Avoid storing `NYKS_WALLET_PASSPHRASE` as a plain-text env var when running
the server interactively.  Instead, use the `wallet_set_password` tool at the
start of a conversation:

> "Set the wallet password to [my password]."

The password is held only in the server process memory and is gone when the
server exits.

---

## Available tools

### Market data (no authentication)

| Tool | Arguments | Description |
|---|---|---|
| `market_price` | — | Current BTC/USD price |
| `market_orderbook` | — | Limit order book (bids & asks) |
| `market_funding_rate` | — | Current perpetual funding rate |
| `market_fee_rate` | — | Trading fee schedule |
| `market_recent_trades` | — | Recently closed trades |
| `market_position_size` | — | Aggregate long / short sizes |
| `market_lend_pool` | — | Lending pool info |
| `market_stats` | — | Comprehensive market statistics (OI, volume, risk params) |
| `market_open_interest` | — | Current open interest (long & short exposure in BTC) |
| `market_server_time` | — | Relayer server UTC timestamp (connectivity check) |
| `market_candles` | `interval?`, `since?`, `limit?`, `offset?` | OHLCV candlestick data |
| `market_historical_price` | `from?`, `to?`, `limit?`, `offset?` | Historical BTC/USD price snapshots |
| `market_historical_funding` | `from?`, `to?`, `limit?`, `offset?` | Historical funding-rate entries |
| `market_pool_share` | — | Current lending-pool share value in sats |
| `market_apy` | — | Last-24-hour lending APY |
| `market_apy_chart` | `range?`, `step?`, `lookback?` | APY chart data over time |
| `market_account_summary` | `twilight_address?` or `wallet_id?` | Trading activity summary for an address |

**`interval`** (for `market_candles`): `ONE_MINUTE` | `FIVE_MINUTE` | `FIFTEEN_MINUTE` | `THIRTY_MINUTE` | `ONE_HOUR` | `FOUR_HOUR` | `EIGHT_HOUR` | `TWELVE_HOUR` | `ONE_DAY`

### Session password management

| Tool | Arguments | Description |
|---|---|---|
| `wallet_set_password` | `password` (required) | Cache password in memory |
| `wallet_clear_password` | — | Clear cached password |

### Wallet management

| Tool | Arguments | Description |
|---|---|---|
| `wallet_create` | `wallet_id?`, `password?` | Generate a new wallet (optionally persist to DB) |
| `wallet_import` | `mnemonic`, `wallet_id?`, `password?` | Import from BIP-39 phrase (optionally persist to DB) |
| `wallet_balance` | `wallet_id?`, `password?` | On-chain NYKS & SATS balance |
| `wallet_accounts` | `wallet_id?`, `password?` | List ZkOS trading accounts |
| `wallet_list` | — | List all wallets in the database |

### Trading (order) operations

| Tool | Required args | Optional args | Description |
|---|---|---|---|
| `order_fund` | `amount` | `wallet_id`, `password` | Fund a new ZkOS account |
| `order_withdraw` | `account_index` | `wallet_id`, `password` | Withdraw to on-chain |
| `order_open_trade` | `account_index`, `order_type`, `side`, `entry_price`, `leverage` | `wallet_id`, `password` | Open a leveraged position |
| `order_close_trade` | `account_index` | `order_type`, `execution_price`, `stop_loss`, `take_profit`, `wallet_id`, `password` | Close a position |
| `order_cancel_trade` | `account_index` | `wallet_id`, `password` | Cancel a pending order |
| `order_query_trade` | `account_index` | `wallet_id`, `password` | Query trade status + SLTP |
| `order_open_lend` | `account_index` | `wallet_id`, `password` | Open lending position |
| `order_close_lend` | `account_index` | `wallet_id`, `password` | Close lending position |
| `order_query_lend` | `account_index` | `wallet_id`, `password` | Query lend status + APR |
| `order_transfer` | `account_index` | `wallet_id`, `password` | Re-anonymise a ZkOS account via private transfer (returns new account index) |

**`order_type`**: `MARKET` | `LIMIT` | `SLTP`
**`side`**: `LONG` | `SHORT`

### Portfolio

| Tool | Arguments | Description |
|---|---|---|
| `portfolio_summary` | `wallet_id?`, `password?` | Full portfolio (PnL, margin, positions) |
| `portfolio_balances` | `wallet_id?`, `password?` | Per-account balances |
| `portfolio_risks` | `wallet_id?`, `password?` | Liquidation risk metrics |
| `portfolio_position_pnl` | `account_index`, `wallet_id?`, `password?` | Detailed PnL & risk for one trading position |
| `portfolio_lend_pnl` | `account_index`, `wallet_id?`, `password?` | Detailed PnL & APR for one lending position |

### History (requires database)

| Tool | Required args | Optional args | Description |
|---|---|---|---|
| `history_orders` | `wallet_id` | `password`, `account_index`, `limit`, `offset` | Order history |
| `history_transfers` | `wallet_id` | `password`, `limit`, `offset` | Transfer history |

---

## Password resolution order

For every tool that requires authentication, the password is resolved in this
order (first non-empty value wins):

1. `password` argument in the tool call
2. `NYKS_WALLET_PASSPHRASE` environment variable
3. In-process session password (set via `wallet_set_password`)

---

## Example conversation

```
User:   "What's the current BTC price?"
Claude: [calls market_price] → { "price": 65432.10, "timestamp": "..." }
        "BTC is currently trading at $65,432."

User:   "Set my wallet password to hunter2 and show my portfolio."
Claude: [calls wallet_set_password { password: "hunter2" }] → "Session password set."
        [calls portfolio_summary { wallet_id: "main" }]
        → { "wallet_balance_sats": 500000, "unrealized_pnl": 1250.5, ... }
        "Your wallet holds 0.005 BTC on-chain.  You have one open long position
         with an unrealised PnL of $1,250.50."

User:   "Open a 5× long at the current price."
Claude: [calls order_open_trade {
           account_index: 0,
           order_type: "MARKET",
           side: "LONG",
           entry_price: 65432,
           leverage: 5
        }]
        → { "request_id": "abc123", "account_index": 0 }
        "Order submitted.  Request ID: abc123."
```

---

## Security notes

- The MCP server process has the same filesystem and network access as the user
  that launched it.  Run it as a **non-root user** (the Docker image does this
  automatically with UID 1000).
- Never pass `NYKS_WALLET_PASSPHRASE` on the command line (it would appear in
  process listings).  Use an env file (`docker run --env-file`) or the
  `wallet_set_password` tool.
- The wallet database is encrypted at rest with AES-256-GCM + PBKDF2
  (600,000 iterations) — the password you provide is the only key.
- All log output goes to **stderr**; stdout carries only MCP protocol messages.
