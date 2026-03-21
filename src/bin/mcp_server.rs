//! nyks-wallet MCP Server
//!
//! Implements the Model Context Protocol (2024-11-05) over stdio.
//! An AI assistant (e.g. Claude) can use this server to interact with
//! a Twilight / NYKS wallet without writing any Rust code.
//!
//! Communication: line-delimited JSON-RPC 2.0 on stdin / stdout.
//! All log/debug output goes to stderr so it never pollutes the protocol stream.
//!
//! Session password:
//!   Use the `wallet_set_password` tool to cache a password in memory for the
//!   lifetime of this server process.  Password resolution order for every
//!   authenticated tool call:
//!     1. `password` argument in the tool call
//!     2. `NYKS_WALLET_PASSPHRASE` environment variable
//!     3. In-process session password (set via `wallet_set_password`)

use std::sync::Arc;

use nyks_wallet::relayer_module::order_wallet::OrderWallet;
use nyks_wallet::relayer_module::relayer_api::RelayerJsonRpcClient;
use nyks_wallet::relayer_module::transaction_history::{OrderHistoryFilter, TransferHistoryFilter};
use secrecy::SecretString;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

// ---------------------------------------------------------------------------
// Server state
// ---------------------------------------------------------------------------

struct ServerState {
    /// Password cached in memory for the lifetime of this server process.
    session_password: Mutex<Option<String>>,
}

impl ServerState {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            session_password: Mutex::new(None),
        })
    }

    /// Resolve password: tool arg → env var → session cache.
    async fn resolve_password(&self, tool_arg: Option<String>) -> Option<String> {
        tool_arg
            .or_else(|| std::env::var("NYKS_WALLET_PASSPHRASE").ok())
            .or(self.session_password.lock().await.clone())
    }
}

// ---------------------------------------------------------------------------
// MCP protocol types (minimal, sufficient for 2024-11-05)
// ---------------------------------------------------------------------------

fn ok_response(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn err_response(id: &Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}

fn tool_ok(text: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": text.into() }] })
}

fn tool_err(text: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": text.into() }], "isError": true })
}

fn tool_json<T: serde::Serialize>(v: &T) -> Value {
    let text = serde_json::to_string_pretty(v)
        .unwrap_or_else(|e| format!("serialization error: {e}"));
    tool_ok(text)
}

// ---------------------------------------------------------------------------
// Tool catalogue
// ---------------------------------------------------------------------------

fn tools_list() -> Value {
    json!([
        // ── Market (no auth) ───────────────────────────────────────────────
        {
            "name": "market_price",
            "description": "Get the current BTC/USD price from the Twilight relayer.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        },
        {
            "name": "market_orderbook",
            "description": "Get the current limit-order book (bids and asks).",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        },
        {
            "name": "market_funding_rate",
            "description": "Get the current perpetual funding rate.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        },
        {
            "name": "market_fee_rate",
            "description": "Get the current trading fee schedule.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        },
        {
            "name": "market_recent_trades",
            "description": "Get recently closed trades.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        },
        {
            "name": "market_position_size",
            "description": "Get aggregate long and short position sizes across all traders.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        },
        {
            "name": "market_lend_pool",
            "description": "Get lending pool information (total locked value, pool share, pending orders).",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        },

        // ── Session password management ────────────────────────────────────
        {
            "name": "wallet_set_password",
            "description": "Cache a wallet password in memory for this MCP server session. \
                            All subsequent authenticated tool calls will use this password \
                            automatically unless overridden by a `password` argument.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "password": { "type": "string", "description": "Wallet encryption password" }
                },
                "required": ["password"]
            }
        },
        {
            "name": "wallet_clear_password",
            "description": "Clear the in-memory session password.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        },

        // ── Wallet management ──────────────────────────────────────────────
        {
            "name": "wallet_create",
            "description": "Create a new Twilight wallet. Returns the Twilight address, \
                            BTC bridge address, and mnemonic. Store the mnemonic safely – \
                            it will not be shown again.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        },
        {
            "name": "wallet_import",
            "description": "Import a wallet from a BIP-39 mnemonic phrase.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "mnemonic": { "type": "string", "description": "BIP-39 mnemonic (12 or 24 words)" }
                },
                "required": ["mnemonic"]
            }
        },
        {
            "name": "wallet_balance",
            "description": "Get the on-chain NYKS and SATS balance of a wallet.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "wallet_id": { "type": "string", "description": "Wallet ID stored in DB" },
                    "password": { "type": "string", "description": "DB decryption password (optional if session password is set)" }
                },
                "required": []
            }
        },
        {
            "name": "wallet_accounts",
            "description": "List all ZkOS trading accounts for a wallet.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "wallet_id": { "type": "string", "description": "Wallet ID stored in DB" },
                    "password": { "type": "string", "description": "DB decryption password" }
                },
                "required": []
            }
        },
        {
            "name": "wallet_list",
            "description": "List all wallets stored in the local database.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        },

        // ── Order / trading ────────────────────────────────────────────────
        {
            "name": "order_fund",
            "description": "Move SATS from the on-chain wallet into a new ZkOS trading account.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "amount": { "type": "integer", "description": "Amount in satoshis" },
                    "wallet_id": { "type": "string" },
                    "password": { "type": "string" }
                },
                "required": ["amount"]
            }
        },
        {
            "name": "order_withdraw",
            "description": "Withdraw SATS from a ZkOS trading account back to the on-chain wallet.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account_index": { "type": "integer", "description": "ZkOS account index" },
                    "wallet_id": { "type": "string" },
                    "password": { "type": "string" }
                },
                "required": ["account_index"]
            }
        },
        {
            "name": "order_open_trade",
            "description": "Open a leveraged trade (LONG or SHORT). \
                            order_type: MARKET | LIMIT | SLTP. side: LONG | SHORT.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account_index": { "type": "integer" },
                    "order_type": { "type": "string", "enum": ["MARKET", "LIMIT", "SLTP"] },
                    "side": { "type": "string", "enum": ["LONG", "SHORT"] },
                    "entry_price": { "type": "number", "description": "Entry price in USD" },
                    "leverage": { "type": "number", "description": "Leverage multiplier (e.g. 10)" },
                    "wallet_id": { "type": "string" },
                    "password": { "type": "string" }
                },
                "required": ["account_index", "order_type", "side", "entry_price", "leverage"]
            }
        },
        {
            "name": "order_close_trade",
            "description": "Close an open trade. Optionally set stop_loss / take_profit \
                            prices (automatically switches to SLTP close).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account_index": { "type": "integer" },
                    "order_type": { "type": "string", "enum": ["MARKET", "LIMIT", "SLTP"], "default": "MARKET" },
                    "execution_price": { "type": "number", "description": "Execution price (optional)" },
                    "stop_loss": { "type": "number", "description": "Stop-loss price (optional)" },
                    "take_profit": { "type": "number", "description": "Take-profit price (optional)" },
                    "wallet_id": { "type": "string" },
                    "password": { "type": "string" }
                },
                "required": ["account_index"]
            }
        },
        {
            "name": "order_cancel_trade",
            "description": "Cancel a pending trade order.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account_index": { "type": "integer" },
                    "wallet_id": { "type": "string" },
                    "password": { "type": "string" }
                },
                "required": ["account_index"]
            }
        },
        {
            "name": "order_query_trade",
            "description": "Query the current status of a trade order (includes SLTP triggers and funding).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account_index": { "type": "integer" },
                    "wallet_id": { "type": "string" },
                    "password": { "type": "string" }
                },
                "required": ["account_index"]
            }
        },
        {
            "name": "order_open_lend",
            "description": "Open a lending position from a ZkOS account to earn yield.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account_index": { "type": "integer" },
                    "wallet_id": { "type": "string" },
                    "password": { "type": "string" }
                },
                "required": ["account_index"]
            }
        },
        {
            "name": "order_close_lend",
            "description": "Close a lending position and redeem funds.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account_index": { "type": "integer" },
                    "wallet_id": { "type": "string" },
                    "password": { "type": "string" }
                },
                "required": ["account_index"]
            }
        },
        {
            "name": "order_query_lend",
            "description": "Query the current status of a lending position (includes unrealised PnL and APR).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account_index": { "type": "integer" },
                    "wallet_id": { "type": "string" },
                    "password": { "type": "string" }
                },
                "required": ["account_index"]
            }
        },

        // ── Portfolio ──────────────────────────────────────────────────────
        {
            "name": "portfolio_summary",
            "description": "Get a full portfolio summary: balances, unrealised PnL, \
                            margin utilisation, trading and lending positions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "wallet_id": { "type": "string" },
                    "password": { "type": "string" }
                },
                "required": []
            }
        },
        {
            "name": "portfolio_balances",
            "description": "Get per-account balances for all ZkOS accounts.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "wallet_id": { "type": "string" },
                    "password": { "type": "string" }
                },
                "required": []
            }
        },
        {
            "name": "portfolio_risks",
            "description": "Get liquidation risk metrics for all open trading positions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "wallet_id": { "type": "string" },
                    "password": { "type": "string" }
                },
                "required": []
            }
        },

        // ── History ────────────────────────────────────────────────────────
        {
            "name": "history_orders",
            "description": "Retrieve order history from the local database.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "wallet_id": { "type": "string" },
                    "password": { "type": "string" },
                    "account_index": { "type": "integer", "description": "Filter by account index (optional)" },
                    "limit": { "type": "integer", "default": 50 },
                    "offset": { "type": "integer", "default": 0 }
                },
                "required": ["wallet_id"]
            }
        },
        {
            "name": "history_transfers",
            "description": "Retrieve fund transfer history from the local database.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "wallet_id": { "type": "string" },
                    "password": { "type": "string" },
                    "limit": { "type": "integer", "default": 50 },
                    "offset": { "type": "integer", "default": 0 }
                },
                "required": ["wallet_id"]
            }
        }
    ])
}

// ---------------------------------------------------------------------------
// Helper: resolve wallet from DB (or create ephemeral)
// ---------------------------------------------------------------------------

async fn load_wallet(
    wallet_id: Option<String>,
    password: Option<String>,
) -> Result<OrderWallet, String> {
    match wallet_id.or_else(|| std::env::var("NYKS_WALLET_ID").ok()) {
        Some(wid) => {
            let pwd = password.map(|p| SecretString::new(p.into()));
            OrderWallet::load_from_db(wid, pwd, None)
        }
        None => OrderWallet::new(None).map_err(|e| e.to_string()),
    }
}

fn relayer_client() -> Result<RelayerJsonRpcClient, String> {
    let endpoint = std::env::var("RELAYER_API_RPC_SERVER_URL")
        .unwrap_or_else(|_| "http://0.0.0.0:8088/api".to_string());
    RelayerJsonRpcClient::new(&endpoint).map_err(|e| e.to_string())
}

fn parse_order_type(
    s: &str,
) -> Result<twilight_client_sdk::relayer_types::OrderType, String> {
    use twilight_client_sdk::relayer_types::OrderType;
    match s.to_uppercase().as_str() {
        "MARKET" => Ok(OrderType::MARKET),
        "LIMIT"  => Ok(OrderType::LIMIT),
        "SLTP"   => Ok(OrderType::SLTP),
        other    => Err(format!("Unknown order type '{other}'. Use MARKET, LIMIT, or SLTP")),
    }
}

fn parse_position_type(
    s: &str,
) -> Result<twilight_client_sdk::relayer_types::PositionType, String> {
    use twilight_client_sdk::relayer_types::PositionType;
    match s.to_uppercase().as_str() {
        "LONG"  => Ok(PositionType::LONG),
        "SHORT" => Ok(PositionType::SHORT),
        other   => Err(format!("Unknown position side '{other}'. Use LONG or SHORT")),
    }
}

fn arg_str(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn arg_u64(args: &Value, key: &str) -> Option<u64> {
    args.get(key).and_then(|v| v.as_u64())
}

fn arg_f64(args: &Value, key: &str) -> Option<f64> {
    args.get(key).and_then(|v| v.as_f64())
}

// ---------------------------------------------------------------------------
// Tool dispatcher
// ---------------------------------------------------------------------------

async fn call_tool(state: Arc<ServerState>, name: &str, args: &Value) -> Value {
    match name {
        // ── Market ─────────────────────────────────────────────────────────
        "market_price" => {
            let c = match relayer_client() { Err(e) => return tool_err(e), Ok(c) => c };
            match c.btc_usd_price().await {
                Ok(p) => tool_json(&p),
                Err(e) => tool_err(e.to_string()),
            }
        }

        "market_orderbook" => {
            let c = match relayer_client() { Err(e) => return tool_err(e), Ok(c) => c };
            match c.open_limit_orders().await {
                Ok(ob) => tool_json(&ob),
                Err(e) => tool_err(e.to_string()),
            }
        }

        "market_funding_rate" => {
            let c = match relayer_client() { Err(e) => return tool_err(e), Ok(c) => c };
            match c.get_funding_rate().await {
                Ok(fr) => tool_json(&fr),
                Err(e) => tool_err(e.to_string()),
            }
        }

        "market_fee_rate" => {
            let c = match relayer_client() { Err(e) => return tool_err(e), Ok(c) => c };
            match c.get_fee_rate().await {
                Ok(fr) => tool_json(&fr),
                Err(e) => tool_err(e.to_string()),
            }
        }

        "market_recent_trades" => {
            let c = match relayer_client() { Err(e) => return tool_err(e), Ok(c) => c };
            match c.recent_trade_orders().await {
                Ok(rt) => tool_json(&rt),
                Err(e) => tool_err(e.to_string()),
            }
        }

        "market_position_size" => {
            let c = match relayer_client() { Err(e) => return tool_err(e), Ok(c) => c };
            match c.position_size().await {
                Ok(ps) => tool_json(&ps),
                Err(e) => tool_err(e.to_string()),
            }
        }

        "market_lend_pool" => {
            let c = match relayer_client() { Err(e) => return tool_err(e), Ok(c) => c };
            match c.lend_pool_info().await {
                Ok(lp) => tool_json(&lp),
                Err(e) => tool_err(e.to_string()),
            }
        }

        // ── Session password ────────────────────────────────────────────────
        "wallet_set_password" => {
            match arg_str(args, "password") {
                None => tool_err("Missing required argument: password"),
                Some(p) if p.is_empty() => tool_err("Password must not be empty"),
                Some(p) => {
                    *state.session_password.lock().await = Some(p);
                    tool_ok("Session password set. All authenticated tool calls will use it automatically.")
                }
            }
        }

        "wallet_clear_password" => {
            *state.session_password.lock().await = None;
            tool_ok("Session password cleared.")
        }

        // ── Wallet management ───────────────────────────────────────────────
        "wallet_create" => {
            match OrderWallet::new(None) {
                Err(e) => tool_err(e.to_string()),
                Ok(ow) => {
                    // Mnemonic is printed to tty in the library; capture the addresses here.
                    tool_ok(serde_json::to_string_pretty(&json!({
                        "twilight_address": ow.wallet.twilightaddress,
                        "btc_address": ow.wallet.btc_address,
                        "note": "The mnemonic was printed to the terminal that started this MCP server. \
                                 Save it securely – it will not be shown again."
                    })).unwrap())
                }
            }
        }

        "wallet_import" => {
            match arg_str(args, "mnemonic") {
                None => tool_err("Missing required argument: mnemonic"),
                Some(mnemonic) => {
                    match OrderWallet::import_from_mnemonic(mnemonic.trim(), None) {
                        Err(e) => tool_err(e.to_string()),
                        Ok(ow) => tool_ok(serde_json::to_string_pretty(&json!({
                            "twilight_address": ow.wallet.twilightaddress,
                            "btc_address": ow.wallet.btc_address,
                        })).unwrap()),
                    }
                }
            }
        }

        "wallet_balance" => {
            let pwd = state.resolve_password(arg_str(args, "password")).await;
            let wid = arg_str(args, "wallet_id");
            match load_wallet(wid, pwd).await {
                Err(e) => tool_err(e),
                Ok(mut ow) => {
                    match ow.wallet.update_balance().await {
                        Err(e) => tool_err(e.to_string()),
                        Ok(balance) => tool_ok(serde_json::to_string_pretty(&json!({
                            "address": ow.wallet.twilightaddress,
                            "btc_address": ow.wallet.btc_address,
                            "nyks": balance.nyks,
                            "sats": balance.sats,
                        })).unwrap()),
                    }
                }
            }
        }

        "wallet_accounts" => {
            let pwd = state.resolve_password(arg_str(args, "password")).await;
            let wid = arg_str(args, "wallet_id");
            match load_wallet(wid, pwd).await {
                Err(e) => tool_err(e),
                Ok(ow) => {
                    let accounts: Vec<_> = ow.zk_accounts.get_all_accounts()
                        .into_iter()
                        .map(|a| json!({
                            "index": a.index,
                            "balance": a.balance,
                            "on_chain": a.on_chain,
                            "io_type": format!("{:?}", a.io_type),
                            "account": a.account,
                        }))
                        .collect();
                    tool_ok(serde_json::to_string_pretty(&json!({ "accounts": accounts })).unwrap())
                }
            }
        }

        "wallet_list" => {
            match OrderWallet::get_wallet_list_from_db(None) {
                Err(e) => tool_err(e.to_string()),
                Ok(wallets) => {
                    let list: Vec<_> = wallets.iter()
                        .map(|w| json!({ "wallet_id": w.wallet_id, "created_at": w.created_at }))
                        .collect();
                    tool_ok(serde_json::to_string_pretty(&json!({ "wallets": list })).unwrap())
                }
            }
        }

        // ── Order / trading ─────────────────────────────────────────────────
        "order_fund" => {
            let amount = match arg_u64(args, "amount") {
                None => return tool_err("Missing required argument: amount"),
                Some(a) => a,
            };
            let pwd = state.resolve_password(arg_str(args, "password")).await;
            match load_wallet(arg_str(args, "wallet_id"), pwd).await {
                Err(e) => tool_err(e),
                Ok(mut ow) => {
                    match ow.funding_to_trading(amount).await {
                        Err(e) => tool_err(e.to_string()),
                        Ok((tx, account_index)) => tool_ok(serde_json::to_string_pretty(&json!({
                            "tx_hash": tx.tx_hash,
                            "tx_code": tx.code,
                            "account_index": account_index,
                        })).unwrap()),
                    }
                }
            }
        }

        "order_withdraw" => {
            let account_index = match arg_u64(args, "account_index") {
                None => return tool_err("Missing required argument: account_index"),
                Some(i) => i,
            };
            let pwd = state.resolve_password(arg_str(args, "password")).await;
            match load_wallet(arg_str(args, "wallet_id"), pwd).await {
                Err(e) => tool_err(e),
                Ok(mut ow) => {
                    match ow.trading_to_funding(account_index).await {
                        Err(e) => tool_err(e.to_string()),
                        Ok(()) => tool_ok(format!("Withdrawal from account {account_index} successful.")),
                    }
                }
            }
        }

        "order_open_trade" => {
            let account_index = match arg_u64(args, "account_index") {
                None => return tool_err("Missing required argument: account_index"),
                Some(i) => i,
            };
            let order_type_str = arg_str(args, "order_type").unwrap_or_else(|| "MARKET".to_string());
            let ot = match parse_order_type(&order_type_str) {
                Err(e) => return tool_err(e),
                Ok(v) => v,
            };
            let side_str = match arg_str(args, "side") {
                None => return tool_err("Missing required argument: side"),
                Some(s) => s,
            };
            let pt = match parse_position_type(&side_str) {
                Err(e) => return tool_err(e),
                Ok(v) => v,
            };
            let entry_price = match arg_u64(args, "entry_price")
                .or_else(|| arg_f64(args, "entry_price").map(|f| f as u64))
            {
                None => return tool_err("Missing required argument: entry_price"),
                Some(p) => p,
            };
            let leverage = match arg_u64(args, "leverage")
                .or_else(|| arg_f64(args, "leverage").map(|f| f as u64))
            {
                None => return tool_err("Missing required argument: leverage"),
                Some(l) => l,
            };
            let pwd = state.resolve_password(arg_str(args, "password")).await;
            match load_wallet(arg_str(args, "wallet_id"), pwd).await {
                Err(e) => tool_err(e),
                Ok(mut ow) => {
                    match ow.open_trader_order(account_index, ot, pt, entry_price, leverage).await {
                        Err(e) => tool_err(e.to_string()),
                        Ok(request_id) => tool_ok(serde_json::to_string_pretty(&json!({
                            "request_id": request_id,
                            "account_index": account_index,
                        })).unwrap()),
                    }
                }
            }
        }

        "order_close_trade" => {
            let account_index = match arg_u64(args, "account_index") {
                None => return tool_err("Missing required argument: account_index"),
                Some(i) => i,
            };
            let execution_price = arg_f64(args, "execution_price");
            let stop_loss = arg_f64(args, "stop_loss");
            let take_profit = arg_f64(args, "take_profit");
            let pwd = state.resolve_password(arg_str(args, "password")).await;

            match load_wallet(arg_str(args, "wallet_id"), pwd).await {
                Err(e) => tool_err(e),
                Ok(mut ow) => {
                    let exec_price = execution_price.unwrap_or(0.0);
                    let result = if stop_loss.is_some() || take_profit.is_some() {
                        ow.close_trader_order_sltp(
                            account_index,
                            twilight_client_sdk::relayer_types::OrderType::SLTP,
                            exec_price,
                            stop_loss,
                            take_profit,
                        ).await
                    } else {
                        let order_type_str = arg_str(args, "order_type").unwrap_or_else(|| "MARKET".to_string());
                        match parse_order_type(&order_type_str) {
                            Err(e) => return tool_err(e),
                            Ok(ot) => ow.close_trader_order(account_index, ot, exec_price).await,
                        }
                    };
                    match result {
                        Err(e) => tool_err(e.to_string()),
                        Ok(request_id) => tool_ok(serde_json::to_string_pretty(&json!({
                            "request_id": request_id,
                            "account_index": account_index,
                        })).unwrap()),
                    }
                }
            }
        }

        "order_cancel_trade" => {
            let account_index = match arg_u64(args, "account_index") {
                None => return tool_err("Missing required argument: account_index"),
                Some(i) => i,
            };
            let pwd = state.resolve_password(arg_str(args, "password")).await;
            match load_wallet(arg_str(args, "wallet_id"), pwd).await {
                Err(e) => tool_err(e),
                Ok(mut ow) => {
                    match ow.cancel_trader_order(account_index).await {
                        Err(e) => tool_err(e.to_string()),
                        Ok(request_id) => tool_ok(serde_json::to_string_pretty(&json!({
                            "request_id": request_id,
                        })).unwrap()),
                    }
                }
            }
        }

        "order_query_trade" => {
            let account_index = match arg_u64(args, "account_index") {
                None => return tool_err("Missing required argument: account_index"),
                Some(i) => i,
            };
            let pwd = state.resolve_password(arg_str(args, "password")).await;
            match load_wallet(arg_str(args, "wallet_id"), pwd).await {
                Err(e) => tool_err(e),
                Ok(mut ow) => {
                    match ow.query_trader_order_v1(account_index).await {
                        Err(e) => tool_err(e.to_string()),
                        Ok(order) => tool_json(&order),
                    }
                }
            }
        }

        "order_open_lend" => {
            let account_index = match arg_u64(args, "account_index") {
                None => return tool_err("Missing required argument: account_index"),
                Some(i) => i,
            };
            let pwd = state.resolve_password(arg_str(args, "password")).await;
            match load_wallet(arg_str(args, "wallet_id"), pwd).await {
                Err(e) => tool_err(e),
                Ok(mut ow) => {
                    match ow.open_lend_order(account_index).await {
                        Err(e) => tool_err(e.to_string()),
                        Ok(request_id) => tool_ok(serde_json::to_string_pretty(&json!({
                            "request_id": request_id,
                        })).unwrap()),
                    }
                }
            }
        }

        "order_close_lend" => {
            let account_index = match arg_u64(args, "account_index") {
                None => return tool_err("Missing required argument: account_index"),
                Some(i) => i,
            };
            let pwd = state.resolve_password(arg_str(args, "password")).await;
            match load_wallet(arg_str(args, "wallet_id"), pwd).await {
                Err(e) => tool_err(e),
                Ok(mut ow) => {
                    match ow.close_lend_order(account_index).await {
                        Err(e) => tool_err(e.to_string()),
                        Ok(request_id) => tool_ok(serde_json::to_string_pretty(&json!({
                            "request_id": request_id,
                        })).unwrap()),
                    }
                }
            }
        }

        "order_query_lend" => {
            let account_index = match arg_u64(args, "account_index") {
                None => return tool_err("Missing required argument: account_index"),
                Some(i) => i,
            };
            let pwd = state.resolve_password(arg_str(args, "password")).await;
            match load_wallet(arg_str(args, "wallet_id"), pwd).await {
                Err(e) => tool_err(e),
                Ok(mut ow) => {
                    match ow.query_lend_order_v1(account_index).await {
                        Err(e) => tool_err(e.to_string()),
                        Ok(order) => tool_json(&order),
                    }
                }
            }
        }

        // ── Portfolio ───────────────────────────────────────────────────────
        "portfolio_summary" => {
            let pwd = state.resolve_password(arg_str(args, "password")).await;
            match load_wallet(arg_str(args, "wallet_id"), pwd).await {
                Err(e) => tool_err(e),
                Ok(mut ow) => {
                    match ow.get_portfolio_summary().await {
                        Err(e) => tool_err(e.to_string()),
                        Ok(portfolio) => tool_json(&portfolio),
                    }
                }
            }
        }

        "portfolio_balances" => {
            let pwd = state.resolve_password(arg_str(args, "password")).await;
            match load_wallet(arg_str(args, "wallet_id"), pwd).await {
                Err(e) => tool_err(e),
                Ok(ow) => {
                    let balances = ow.get_account_balances();
                    tool_json(&balances)
                }
            }
        }

        "portfolio_risks" => {
            let pwd = state.resolve_password(arg_str(args, "password")).await;
            match load_wallet(arg_str(args, "wallet_id"), pwd).await {
                Err(e) => tool_err(e),
                Ok(mut ow) => {
                    match ow.get_liquidation_risks().await {
                        Err(e) => tool_err(e.to_string()),
                        Ok(risks) => tool_json(&risks),
                    }
                }
            }
        }

        // ── History ─────────────────────────────────────────────────────────
        "history_orders" => {
            let wallet_id = match arg_str(args, "wallet_id")
                .or_else(|| std::env::var("NYKS_WALLET_ID").ok())
            {
                None => return tool_err("Missing required argument: wallet_id"),
                Some(w) => w,
            };
            let pwd = state.resolve_password(arg_str(args, "password")).await;
            let pwd_secret = pwd.map(|p| SecretString::new(p.into()));
            match OrderWallet::load_from_db(wallet_id, pwd_secret, None) {
                Err(e) => tool_err(e.to_string()),
                Ok(ow) => {
                    let filter = OrderHistoryFilter {
                        account_index: arg_u64(args, "account_index"),
                        limit: Some(arg_u64(args, "limit").unwrap_or(50) as i64),
                        offset: Some(arg_u64(args, "offset").unwrap_or(0) as i64),
                    };
                    match ow.get_order_history(filter) {
                        Err(e) => tool_err(e.to_string()),
                        Ok(entries) => tool_json(&entries),
                    }
                }
            }
        }

        "history_transfers" => {
            let wallet_id = match arg_str(args, "wallet_id")
                .or_else(|| std::env::var("NYKS_WALLET_ID").ok())
            {
                None => return tool_err("Missing required argument: wallet_id"),
                Some(w) => w,
            };
            let pwd = state.resolve_password(arg_str(args, "password")).await;
            let pwd_secret = pwd.map(|p| SecretString::new(p.into()));
            match OrderWallet::load_from_db(wallet_id, pwd_secret, None) {
                Err(e) => tool_err(e.to_string()),
                Ok(ow) => {
                    let filter = TransferHistoryFilter {
                        limit: Some(arg_u64(args, "limit").unwrap_or(50) as i64),
                        offset: Some(arg_u64(args, "offset").unwrap_or(0) as i64),
                    };
                    match ow.get_transfer_history(filter) {
                        Err(e) => tool_err(e.to_string()),
                        Ok(entries) => tool_json(&entries),
                    }
                }
            }
        }

        unknown => tool_err(format!("Unknown tool: {unknown}")),
    }
}

// ---------------------------------------------------------------------------
// Request dispatcher
// ---------------------------------------------------------------------------

async fn handle_request(state: Arc<ServerState>, line: &str) -> Option<Value> {
    let req: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => {
            return Some(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32700, "message": format!("Parse error: {e}") }
            }));
        }
    };

    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = match req.get("method").and_then(|m| m.as_str()) {
        Some(m) => m,
        None => return Some(err_response(&id, -32600, "Missing method")),
    };

    match method {
        // ── MCP handshake ──────────────────────────────────────────────────
        "initialize" => Some(ok_response(&id, json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "nyks-wallet-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        }))),

        // Client signals it has processed initialize – no response needed.
        "notifications/initialized" => None,

        // ── Tool endpoints ─────────────────────────────────────────────────
        "tools/list" => Some(ok_response(&id, json!({ "tools": tools_list() }))),

        "tools/call" => {
            let params = req.get("params").cloned().unwrap_or(json!({}));
            let tool_name = match params.get("name").and_then(|n| n.as_str()) {
                Some(n) => n.to_string(),
                None => return Some(err_response(&id, -32602, "Missing tool name")),
            };
            let tool_args = params.get("arguments").cloned().unwrap_or(json!({}));

            let result = call_tool(state, &tool_name, &tool_args).await;
            Some(ok_response(&id, result))
        }

        // ── Unknown method ─────────────────────────────────────────────────
        other => {
            eprintln!("[mcp] unhandled method: {other}");
            Some(err_response(&id, -32601, &format!("Method not found: {other}")))
        }
    }
}

// ---------------------------------------------------------------------------
// Main: stdio event loop
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    // Keep library logs out of the protocol stream.
    if std::env::var("RUST_LOG").is_err() {
        // SAFETY: single-threaded at this point, before tokio spawns workers.
        unsafe { std::env::set_var("RUST_LOG", "error") };
    }
    env_logger::Builder::from_env(env_logger::Env::default()).target(env_logger::Target::Stderr).init();

    let state = ServerState::new();
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin).lines();

    eprintln!("[nyks-wallet-mcp] server started (MCP 2024-11-05), waiting for messages…");

    loop {
        match reader.next_line().await {
            Ok(Some(line)) if !line.trim().is_empty() => {
                eprintln!("[mcp] ← {line}");
                if let Some(response) = handle_request(Arc::clone(&state), &line).await {
                    let mut out = serde_json::to_string(&response).unwrap();
                    out.push('\n');
                    eprintln!("[mcp] → {}", out.trim_end());
                    if let Err(e) = stdout.write_all(out.as_bytes()).await {
                        eprintln!("[mcp] write error: {e}");
                        break;
                    }
                    if let Err(e) = stdout.flush().await {
                        eprintln!("[mcp] flush error: {e}");
                        break;
                    }
                }
            }
            Ok(Some(_)) => { /* blank line – ignore */ }
            Ok(None) => {
                eprintln!("[nyks-wallet-mcp] stdin closed, shutting down.");
                break;
            }
            Err(e) => {
                eprintln!("[mcp] read error: {e}");
                break;
            }
        }
    }
}
