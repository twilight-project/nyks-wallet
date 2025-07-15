use super::MsgMintBurnTradingBtc;
use super::faucet::fetch_account_details;
use super::wallet::*;
use bip32::PrivateKey;
use curve25519_dalek::scalar::Scalar;
use fastrand::Rng;
use prost::Message;
use quisquislib::accounts::Account;
use quisquislib::keys::PublicKey;
use quisquislib::ristretto::{RistrettoPublicKey, RistrettoSecretKey};
use reqwest::header::HeaderMap;
use twilight_client_sdk::quisquislib;

use anyhow::anyhow;
use base64::{Engine as _, engine::general_purpose};
use cosmrs::tendermint::chain::Id;
use cosmrs::{
    crypto::secp256k1::SigningKey,
    proto::cosmos::base::v1beta1::Coin,
    tx::{self, Body, Fee, Raw, SignDoc, SignerInfo},
};
use prost::Message as _;
use reqwest::Client;
use serde_json::{Value, json};
use std::str::FromStr;
// ---------- constants you’ll want to tweak ----------
const RPC: &str = "https://rpc.twilight.rest"; // Nyks RPC, not LCD
const CHAIN_ID: &str = "nyks";
const DENOM: &str = "nyks"; // replace with Nyks micro‑denom
// ----------------------------------------------------

pub fn create_funiding_to_trading_tx_msg(
    mint_or_burn: bool,
    btc_value: u64,
    qq_account: String,
    encrypt_scalar: String,
    twilight_address: String,
) -> MsgMintBurnTradingBtc {
    let msg = MsgMintBurnTradingBtc {
        mint_or_burn,
        btc_value,
        qq_account,
        encrypt_scalar,
        twilight_address,
    };
    msg
    // let mut buf = Vec::new();
    // msg.encode(&mut buf).expect("msg encoding failed");
    // cosmrs::Any {
    //     type_url: "/twilightproject.nyks.zkos.MsgMintBurnTradingBtc".to_string(),
    //     value: buf,
    // }
}
pub fn get_quisquis_account() -> (String, String) {
    // let public_key = PublicKey::from_str(&public_key).unwrap();
    // let seed =
    //     "tX4LFWW05uhE/N+VKWY8ikY5+NnFm5IXbK1SnGMpdiZma5wLXOD5t5s8I/P895VdembmQYeHnWWDceovTBp2Qw==";
    // let account_sort = "0c0ac626dc24a96380b7640c90a03461a0aaf16e771839c95f584f82bb9975015c7a6c946aafa060c78424699190413702a6458f3b0262d42f510d7f93054841253378ff89";
    // let account = "0c5c36ec4eee3811e82e3e354e9ad96dd240831c5e0100dad43eb2c87508e3236204965efe7c2b93da826d0fd2cb95462cc785d9a9c698482c94d4628d24c50d16fc1f3b996aaaffe04c1b84ab116ffe01cd5c8952ab9be7152d65bb40be593d6cd5b5eb2c74a6baedb2b87bdc87af9b06afefe57c3afa0902b86911c30e676399a715c77d";

    // let scalar_sort = "f4ef453352a387869a31e898b690c637d8fad27bf0b1c79f482b16d2e6188707";
    // let scalar = "a4cae9ad74de209a2fc3512d30f22a6b3d33d87b9fb2e7bf0a56f32943a0cc04";

    let account = "0c7811a4496cb25be33fc96514bffba4d8c27befabc4fec7a17c86c31a66c38d53a6113f126c7cf9a6d718b21029dd7eb55b0397c9f1af7baa2f41e3bacab51313fe71e051c4301500aaab0a5ed4a00253fd339e7025a4a0b5954f369c0634656b872dc4029c5bc4abd85721ef90cc67188d0a95d5046d1db3cafa61cba9fc61eb319de03d";
    let scalar = "4fd91931c1700868734dcc26a1312dce47502771127bf7281f9a1187b183d30a";
    (account.to_string(), scalar.to_string())
}

pub async fn send_tx(msg: MsgMintBurnTradingBtc) -> anyhow::Result<()> {
    // 1. Load your key
    let wallet = Wallet::import_from_json("test.json")?;
    let sk = wallet.signing_key()?;
    let pk = wallet.public_key()?;
    let account_details = fetch_account_details(&wallet.twilightaddress).await?;
    let sequence = account_details.account.sequence.parse::<u64>()?;
    let account_number = account_details.account.account_number.parse::<u64>()?;
    // 2. Craft the custom message
    let msg = msg;
    let mut buf = Vec::new();
    msg.encode(&mut buf).expect("msg encoding failed");
    let any = cosmrs::Any {
        type_url: "/twilightproject.nyks.zkos.MsgMintBurnTradingBtc".to_string(),
        value: buf,
    };
    // 3. Put it into a TxBody
    let body = Body::new(vec![any], "", 0u16);

    // 4. Fee & signer info
    let fee = Fee::from_amount_and_gas(
        cosmrs::Coin {
            denom: cosmrs::Denom::from_str("nyks").map_err(|e| anyhow!("{}", e))?,
            amount: 1_000u64.into(),
        },
        200_0000u64,
    );
    let auth_info = SignerInfo::single_direct(Some(pk.into()), sequence).auth_info(fee);
    let chain_id = Id::try_from("nyks").map_err(|e| anyhow!("{}", e))?;

    let sign_doc =
        SignDoc::new(&body, &auth_info, &chain_id, account_number).map_err(|e| anyhow!("{}", e))?;

    let raw_tx = sign_doc.sign(&sk).map_err(|e| anyhow!("{}", e))?;

    // --- Encode & broadcast
    let tx_bytes = raw_tx.to_bytes().map_err(|e| anyhow!("{}", e))?;
    let tx_base64 = general_purpose::STANDARD.encode(&tx_bytes);
    println!("tx_base64: {}", tx_base64);

    let rpc_body = RpcBody::new(
        "2.0".to_string(),
        0,
        "broadcast_tx_sync".to_string(),
        tx_base64,
    );
    let tx = serde_json::to_string(&rpc_body).unwrap();
    println!("{}", tx);

    let client = reqwest::Client::new();
    let clint_clone = client.clone();
    let res = clint_clone
        .post("https://rpc.twilight.rest")
        .headers(construct_headers())
        .body(tx)
        .send();
    println!(
        "Broadcast response: {:?}",
        serde_json::from_slice::<Value>(&res.await.unwrap().bytes().await.unwrap()).unwrap()
    );
    // Broadcast response: Object {"id": Number(0), "jsonrpc": String("2.0"), "result": Object {"code": Number(0), "codespace": String(""), "data": String(""), "hash": String("ABE2D106BA6A2986E8E4EA0272D101507E0399FCFFA41A05748BBACB421BE356"), "log": String("[]")}}
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_funding_to_trading_tx_msg() {
        let (qq_account, encrypt_scalar) = get_quisquis_account();
        let twilight_address = "twilight1ykm5td5kw2hwafmhn7qha54p20veh9um05dpjn".to_string();

        let msg = create_funiding_to_trading_tx_msg(
            true,  // mint
            50000, // btc value in sats
            qq_account,
            encrypt_scalar,
            twilight_address,
        );

        // println!("Message type_url: {}", msg.type_url);
        // println!("Message value (base64): {}", base64::encode(&msg.value));
    }
    #[tokio::test]
    async fn test_send_tx() -> anyhow::Result<()> {
        let (qq_account, encrypt_scalar) = get_quisquis_account();
        let twilight_address = "twilight1ykm5td5kw2hwafmhn7qha54p20veh9um05dpjn".to_string();
        let msg = create_funiding_to_trading_tx_msg(
            true,
            40000,
            qq_account,
            encrypt_scalar,
            twilight_address,
        );
        send_tx(msg).await?;
        Ok(())
    }
}

use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RpcBody {
    /// JSON-RPC version
    pub jsonrpc: String,

    /// Identifier included in request
    pub id: i64,

    /// Request method
    pub method: String,

    /// Request parameters (i.e. request object)
    pub params: TxParams,
}
impl RpcBody {
    pub fn new(jsonrpc: String, id: i64, method: String, tx: String) -> Self {
        Self {
            jsonrpc,
            id,
            method,
            params: TxParams::new(tx),
        }
    }
}

use reqwest::header::{ACCEPT, ACCEPT_ENCODING, CONTENT_TYPE, HeaderValue, USER_AGENT};

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
