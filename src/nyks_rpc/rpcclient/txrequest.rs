use super::json_rpc::id::Id;
use super::method::Method;
// use curve25519_dalek::digest::Output;
use jsonrpc_core::Version;
use jsonrpc_core::response::Output;
use serde::{Deserialize, Serialize};
// use super::method::Method;
use reqwest::blocking::Response;
use reqwest::header::{ACCEPT, ACCEPT_ENCODING, CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
use std::fs::File;
use std::io::prelude::*;
// pub type TransactionStatusId = String;
// use crate::nyks_rpc::rpcclient::method::ByteRec;
lazy_static! {
    pub static ref FAUCET_BASE_URL: String =
        std::env::var("FAUCET_BASE_URL").expect("missing environment variable FAUCET_BASE_URL");
    pub static ref LCD_BASE_URL: String =
        std::env::var("LCD_BASE_URL").expect("missing environment variable LCD_BASE_URL");
}

fn construct_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("reqwest"));
    headers.insert(
        ACCEPT_ENCODING,
        HeaderValue::from_static("gzip, deflate, br"),
    );
    headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TxParams {
    tx: String,
}
impl TxParams {
    pub fn new(tx: String) -> Self {
        Self { tx }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RpcBody<T> {
    /// JSON-RPC version
    pub jsonrpc: Version,

    /// Identifier included in request
    pub id: Id,

    /// Request method
    pub method: Method,

    /// Request parameters (i.e. request object)
    pub params: T,
}

pub trait RpcRequest<T>
where
    Self: Sized,
{
    // fn remove(&mut self, order: T, cmd: RpcCommand) -> Result<T, std::io::Error>;
    fn new(request: T, method: Method) -> Self;

    fn new_with_id(id: Id, request: T, method: Method) -> Self;

    fn new_with_data(request: T, method: Method, data: String) -> (Self, String);

    fn id(&self) -> &Id;

    fn params(&self) -> &T;

    fn get_method(&self) -> &Method;

    fn into_json(self) -> String;

    // fn send(self, url: String) -> Result<Response, reqwest::Error>;
    fn send(self, url: String) -> Result<RpcResponse<serde_json::Value>, reqwest::Error>;
    // fn response(resp: Result<Response, reqwest::Error>);
    // // -> Result<jsonrpc_core::Response, jsonrpc_core::Error>;
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RpcResponse<T> {
    pub jsonrpc: Version,

    /// Identifier included in request
    pub id: jsonrpc_core::Id,
    pub result: Result<T, jsonrpc_core::Error>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resp {
    /// Protocol version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsonrpc: Option<Version>,
    /// Result
    pub result: String,
    /// Correlation id
    pub id: Id,
}

pub fn rpc_response(
    resp: Result<Response, reqwest::Error>,
) -> Result<RpcResponse<serde_json::Value>, reqwest::Error> {
    match resp {
        Ok(response) => {
            // if response.status().is_success() {
            let output: Output = serde_json::from_slice(&response.bytes().unwrap()).unwrap();
            let rpc_response = match output {
                Output::Success(s) => RpcResponse {
                    jsonrpc: s.jsonrpc.unwrap(),
                    id: s.id,
                    result: Ok(s.result),
                },
                Output::Failure(f) => RpcResponse {
                    jsonrpc: f.jsonrpc.unwrap(),
                    id: f.id,
                    result: Err(f.error),
                },
            };
            return Ok(rpc_response);

            // } else { };
        }
        Err(arg) => Err(arg),
    }
}

impl RpcRequest<TxParams> for RpcBody<TxParams> {
    fn new(request: TxParams, method: Method) -> Self {
        Self::new_with_id(Id::uuid_v4(), request, method)
    }

    fn new_with_id(id: Id, request: TxParams, method: Method) -> Self {
        Self {
            jsonrpc: Version::V2,
            id,
            method: method,
            params: request,
        }
    }
    fn new_with_data(request: TxParams, method: Method, data: String) -> (Self, String) {
        (
            Self {
                jsonrpc: Version::V2,
                id: Id::uuid_v4(),
                method: method,
                params: request,
            },
            data,
        )
    }

    fn id(&self) -> &Id {
        &self.id
    }

    fn params(&self) -> &TxParams {
        &self.params
    }
    fn into_json(self) -> String {
        let tx = serde_json::to_string(&self).unwrap();
        let mut file = File::create("foo.txt").unwrap();
        file.write_all(&serde_json::to_vec_pretty(&tx.clone()).unwrap())
            .unwrap();
        tx
    }

    fn get_method(&self) -> &Method {
        &self.method
    }

    fn send(
        self,
        url: std::string::String,
    ) -> Result<RpcResponse<serde_json::Value>, reqwest::Error> {
        match self.method {
            Method::broadcast_tx_sync => {
                let client = reqwest::blocking::Client::new();
                let clint_clone = client.clone();
                let res = clint_clone
                    .post(url)
                    .headers(construct_headers())
                    .body(self.into_json())
                    .send();

                return rpc_response(res);
            }
            _ => {
                let client = reqwest::blocking::Client::new();
                let clint_clone = client.clone();
                let res = clint_clone
                    .post(url)
                    .headers(construct_headers())
                    .body(self.into_json())
                    .send();

                return rpc_response(res);
            }
        }
    }
}
