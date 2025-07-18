use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::convert::TryFrom;

/// Success payload returned by `broadcast_tx_sync` (and friends).
///
/// Example JSON returned by Tendermint:
/// ```json
/// {
///   "jsonrpc": "2.0",
///   "id": 0,
///   "result": {
///     "code": 0,
///     "codespace": "",
///     "data": "",
///     "hash": "ABE2D…",
///     "log": "[]"
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TxResult {
    pub code: u32,
    pub codespace: String,
    pub data: String,
    pub hash: String,
    pub log: String,
}

impl TryFrom<Value> for TxResult {
    type Error = serde_json::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

impl<'a> TryFrom<&'a Value> for TxResult {
    type Error = serde_json::Error;

    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value.clone())
    }
}

/// Helper that maps a `RpcResponse<Value>` to `Result<TxResult, jsonrpc_core::Error>`.
///
/// Usage:
/// ```rust
/// let resp: RpcResponse<Value> = ...;
/// match txresult::from_rpc_response(resp) {
///     Ok(tx) => println!("tx hash {} succeeded", tx.hash),
///     Err(e) => eprintln!("tx failed: {e}")
/// }
/// ```
use crate::nyks_rpc::rpcclient::txrequest::RpcResponse;

pub fn from_rpc_response(resp: RpcResponse<Value>) -> Result<TxResult, jsonrpc_core::Error> {
    match resp.result {
        Ok(val) => TxResult::try_from(val).map_err(|e| jsonrpc_core::Error {
            code: jsonrpc_core::ErrorCode::ParseError,
            message: format!("failed to parse TxResult: {e}"),
            data: None,
        }),
        Err(err) => Err(err),
    }
}
