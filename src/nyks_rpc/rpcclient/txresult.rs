use crate::nyks_rpc::rpcclient::method::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::convert::TryFrom;
// use tendermint_rpc::{
//     Client, HttpClient,
//     endpoint::broadcast::{tx_commit::Response as CommitResp, tx_sync::Response as SyncResp},
// };

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)] // <- let Serde decide by field‑shape
pub enum TxResponse {
    BroadcastTxSync(TxResult),
    BroadcastTxAsync(TxResult),
    BroadcastTxCommit(TxResultTxCommit),
}

impl TxResponse {
    pub fn get_tx_hash(&self) -> String {
        match self {
            TxResponse::BroadcastTxSync(tx) => tx.hash.clone(),
            TxResponse::BroadcastTxAsync(tx) => tx.hash.clone(),
            TxResponse::BroadcastTxCommit(tx) => tx.hash.clone(),
        }
    }
    pub fn get_code(&self) -> u32 {
        match self {
            TxResponse::BroadcastTxSync(tx) => tx.code.clone(),
            TxResponse::BroadcastTxAsync(tx) => tx.code.clone(),
            TxResponse::BroadcastTxCommit(tx) => tx.deliver_tx.code.clone(),
        }
    }
}

// use crate::nyks_rpc::rpcclient::{
//     method::Method,
//     txrequest::RpcResponse,
//     txresult::{
//         TxResult, // you already have
//         TxResultTxCommit,
//     },
// };
use jsonrpc_core::{Error, ErrorCode};
// use serde_json::Value;

pub fn parse_tx_response(method: &Method, resp: RpcResponse<Value>) -> Result<TxResponse, Error> {
    match (method, resp.result) {
        (Method::broadcast_tx_sync, Ok(val)) => TxResult::try_from(val)
            .map(TxResponse::BroadcastTxSync)
            .map_err(parse_err),
        (Method::broadcast_tx_commit, Ok(val)) => TxResultTxCommit::try_from(val)
            .map(TxResponse::BroadcastTxCommit)
            .map_err(parse_err),
        (_, Ok(_)) => Err(Error {
            code: ErrorCode::InvalidParams,
            message: "method not supported by parse_tx_response".into(),
            data: None,
        }),
        (_, Err(e)) => Err(e),
    }
}

fn parse_err(e: serde_json::Error) -> Error {
    Error {
        code: ErrorCode::ParseError,
        message: e.to_string(),
        data: None,
    }
}

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
    pub codespace: Option<String>,
    pub height: Option<u64>,
    pub data: Option<String>,
    pub hash: String,
    pub txhash: Option<String>,
    pub logs: Option<Vec<LogEntry>>,
    pub raw_log: Option<String>,
    pub info: Option<String>,
    pub gas_wanted: Option<String>,
    pub gas_used: Option<String>,
    pub tx: Option<Tx>,
    pub timestamp: Option<String>,
    pub events: Option<Vec<Event>>,
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
/// use nyks_wallet::nyks_rpc::rpcclient::txrequest::RpcResponse;
/// use nyks_wallet::nyks_rpc::rpcclient::txresult::from_rpc_response;
/// use serde_json::{Value, json};
/// use jsonrpc_core::{Id, Version};
/// use nyks_wallet::nyks_rpc::rpcclient::txresult;
/// let resp: RpcResponse<Value> = RpcResponse {
///     jsonrpc: Version::V2,
///     id: Id::Num(0),
///     result: Ok(json!({
///         "code": 0,
///         "codespace": "",
///         "data": "",
///         "hash": "ABE2D…",
///         "log": "[]"
///     }))
/// };
/// match txresult::from_rpc_response(resp) {
///     Ok(tx) => println!("tx hash {:?} succeeded", tx.hash),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogEntry {
    pub msg_index: u32,
    pub log: Option<String>,
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Event {
    #[serde(rename = "type")]
    pub event_type: String,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Attribute {
    pub key: String,
    pub value: String,
    pub index: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tx {
    #[serde(rename = "@type")]
    pub tx_type: String,
    pub body: TxBody,
    pub auth_info: AuthInfo,
    pub signatures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TxBody {
    pub messages: Vec<serde_json::Value>,
    pub memo: Option<String>,
    pub timeout_height: String,
    pub extension_options: Vec<serde_json::Value>,
    pub non_critical_extension_options: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthInfo {
    pub signer_infos: Vec<SignerInfo>,
    pub fee: Fee,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignerInfo {
    pub public_key: PublicKey,
    pub mode_info: ModeInfo,
    pub sequence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicKey {
    #[serde(rename = "@type")]
    pub key_type: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModeInfo {
    pub single: SingleMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SingleMode {
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Fee {
    pub amount: Vec<Coin>,
    pub gas_limit: String,
    pub payer: Option<String>,
    pub granter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Coin {
    pub denom: String,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TxResultTxCommit {
    pub check_tx: TxPart,
    pub deliver_tx: TxPart,
    pub hash: String,
    pub height: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TxPart {
    pub code: u32,
    pub codespace: Option<String>,
    pub data: Option<String>,
    pub events: Option<Vec<Event>>, // optional and may be empty
    pub gas_used: Option<String>,
    pub gas_wanted: Option<String>,
    pub info: Option<String>,
    #[serde(default, deserialize_with = "deserialize_log")]
    pub log: Option<Vec<StructuredLog>>,
}

pub fn from_rpc_response_tx_commit(
    resp: RpcResponse<Value>,
) -> Result<TxResultTxCommit, jsonrpc_core::Error> {
    match resp.result {
        Ok(val) => TxResultTxCommit::try_from(val).map_err(|e| jsonrpc_core::Error {
            code: jsonrpc_core::ErrorCode::ParseError,
            message: format!("failed to parse TxResult: {e}"),
            data: None,
        }),
        Err(err) => Err(err),
    }
}

impl TryFrom<Value> for TxResultTxCommit {
    type Error = serde_json::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

impl<'a> TryFrom<&'a Value> for TxResultTxCommit {
    type Error = serde_json::Error;

    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value.clone())
    }
}
use serde::de::Deserializer;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructuredLog {
    pub events: Vec<Event>,
}

fn deserialize_log<'de, D>(deserializer: D) -> Result<Option<Vec<StructuredLog>>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: Option<String> = Option::deserialize(deserializer)?;
    if let Some(json_str) = raw {
        let parsed: Result<Vec<StructuredLog>, _> = serde_json::from_str(&json_str);
        return parsed.map(Some).map_err(serde::de::Error::custom);
    }
    Ok(None)
}
