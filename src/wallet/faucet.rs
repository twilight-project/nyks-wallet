use crate::nyks_rpc::rpcclient::txrequest::{FAUCET_BASE_URL, NYKS_LCD_BASE_URL};

use super::super::MsgRegisterBtcDepositAddress;
use anyhow::anyhow;
use base64::{Engine as _, engine::general_purpose};
use cosmrs::crypto::{PublicKey, secp256k1::SigningKey};
use cosmrs::tendermint::chain::Id;
use cosmrs::{
    Coin,
    tx::{Body, Fee, SignDoc, SignerInfo},
};
use log::debug;
use prost::Message;
use reqwest::Client;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use std::{error::Error, str::FromStr};

pub fn create_register_btc_deposit_message(
    btc_address: String,
    btc_amount: u64,
    twilight_amount: u64,
    twilight_address: String,
) -> cosmrs::Any {
    let msg = MsgRegisterBtcDepositAddress {
        btc_deposit_address: btc_address,
        btc_satoshi_test_amount: btc_amount,
        twilight_staking_amount: twilight_amount,
        twilight_address,
    };

    let mut buf = Vec::new();
    msg.encode(&mut buf).expect("msg encoding failed");

    cosmrs::Any {
        type_url: "/twilightproject.nyks.bridge.MsgRegisterBtcDepositAddress".to_string(),
        value: buf,
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AccountResponse {
    pub account: Account,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Account {
    #[serde(rename = "@type")]
    pub account_type: String,
    pub address: String,
    // `pub_key` can be an object or null depending on whether the account has
    // a public key set on-chain. We do not use its inner fields in the current
    // code, so deserialize it into a generic `serde_json::Value` to avoid
    // strict type expectations that lead to parsing errors when it is an
    // object.
    pub pub_key: Option<Value>,
    pub account_number: String,
    pub sequence: String,
}

pub async fn fetch_account_details(address: &str) -> anyhow::Result<AccountResponse> {
    let url = format!(
        "{}/cosmos/auth/v1beta1/accounts/{}",
        NYKS_LCD_BASE_URL.as_str(),
        address
    );
    let client = Client::new();
    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        let account_response: AccountResponse = response.json().await?;
        Ok(account_response)
    } else {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "No response body".to_string());
        Err(anyhow!(
            "Failed to fetch account details. Status: {}, Error: {}",
            status,
            error_body
        ))
    }
}

pub async fn get_nyks(recipient_address: &str) -> Result<(), Box<dyn Error>> {
    let url = format!("{}/faucet", FAUCET_BASE_URL.as_str());
    let payload = json!({ "recipientAddress": recipient_address });
    let client = Client::new();
    let response = client.post(url).json(&payload).send().await?;

    if response.status().is_success() {
        debug!("Faucet response: {}", response.text().await?);
        Ok(())
    } else {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "No response body".to_string());
        Err(format!(
            "Failed to call /faucet. Status: {}, Error: {}",
            status, error_body
        )
        .into())
    }
}

pub async fn mint_sats(recipient_address: &str) -> Result<(), Box<dyn Error>> {
    let url = format!("{}/mint", FAUCET_BASE_URL.as_str());

    let payload = json!({ "recipientAddress": recipient_address });
    let client = Client::new();
    let response = client.post(url).json(&payload).send().await?;

    if response.status().is_success() {
        debug!("Mint response: {}", response.text().await?);
        Ok(())
    } else {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "No response body".to_string());
        Err(format!(
            "Failed to call /mint. Status: {}, Error: {}",
            status, error_body
        )
        .into())
    }
}
pub async fn mint_sats_5btc(recipient_address: &str) -> Result<(), Box<dyn Error>> {
    let url = format!("{}/mint-relayer-wallet", FAUCET_BASE_URL.as_str());

    let payload = json!({ "recipientAddress": recipient_address });
    let client = Client::new();
    let response = client.post(url).json(&payload).send().await?;

    if response.status().is_success() {
        debug!("Mint response: {}", response.text().await?);
        Ok(())
    } else {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "No response body".to_string());
        Err(format!(
            "Failed to call /mint. Status: {}, Error: {}",
            status, error_body
        )
        .into())
    }
}

pub async fn sign_and_send_reg_deposit_tx(
    signing_key: SigningKey,
    public_key: PublicKey,
    sender_account: String,
    btc_address: String,
) -> anyhow::Result<()> {
    // --- Msg & body
    let msg_any =
        create_register_btc_deposit_message(btc_address, 50_000, 10_000, sender_account.clone());
    let body = Body::new(vec![msg_any], "", 0u16);

    // --- On‑chain numbers
    let account_details = fetch_account_details(&sender_account).await?;
    let sequence = account_details.account.sequence.parse::<u64>()?;
    let account_number = account_details.account.account_number.parse::<u64>()?;

    // --- Fee & auth‑info
    let gas_limit = 200_000u64;
    let fee_amount = Coin {
        denom: cosmrs::Denom::from_str("nyks").map_err(|e| anyhow!("{}", e))?,
        amount: 1_000u64.into(),
    };
    let signer_info = SignerInfo::single_direct(Some(public_key), sequence);
    let auth_info = signer_info.auth_info(Fee::from_amount_and_gas(fee_amount, gas_limit));

    // --- Sign
    let chain_id = Id::try_from("nyks").map_err(|e| anyhow!("{}", e))?;
    let sign_doc =
        SignDoc::new(&body, &auth_info, &chain_id, account_number).map_err(|e| anyhow!("{}", e))?;
    let raw_tx = sign_doc.sign(&signing_key).map_err(|e| anyhow!("{}", e))?;

    // --- Encode & broadcast
    let tx_bytes = raw_tx.to_bytes().map_err(|e| anyhow!("{}", e))?;
    let tx_base64 = general_purpose::STANDARD.encode(&tx_bytes);
    let client = Client::new();
    let res = client
        .post(format!(
            "{}/cosmos/tx/v1beta1/txs",
            NYKS_LCD_BASE_URL.as_str()
        ))
        .json(&json!({ "tx_bytes": tx_base64, "mode": "BROADCAST_MODE_SYNC" }))
        .send()
        .await?;

    debug!("Broadcast response: {}", res.text().await?);
    Ok(())
}
