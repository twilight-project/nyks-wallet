use cosmrs::AccountId;
use cosmrs::crypto::PublicKey;
use cosmrs::crypto::secp256k1::SigningKey;
use reqwest::Client;
use ripemd::Ripemd160;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{error::Error, str::FromStr};

use base64;
use cosmrs::tendermint::chain::Id;
use cosmrs::{
    Any, Coin,
    tx::{Body, Fee, SignDoc, SignerInfo},
};
use prost::Message;
use reqwest;
use serde::Deserialize;
use tokio::time::{Duration, sleep};

pub mod nyks {
    pub mod module {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/twilightproject.nyks.bridge.rs"));
        }
    }
}
use nyks::module::v1::MsgRegisterBtcDepositAddress;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (priv_key, pub_key, address) = generate_cosmos_account("twilight")?;
    println!("Address:     {}", address.to_string());

    match get_nyks(&address.to_string()).await {
        Ok(_) => {
            println!("Successfully called get_nyks for address: {}", address);
        }
        Err(e) => {
            eprintln!("Failed to call get_nyks: {}", e);
            return Ok(());
        }
    };

    sleep(Duration::from_secs(2 * 5)).await;

    sign_and_send_reg_deposit_tx(
        priv_key,
        pub_key,
        address.to_string(),
        "bc1qxdlfjgffe9a4sc9yswdvnaxtjz8e46jnu3vkqu".to_string(),
    )
    .await?;
    println!(
        "Successfully registered BTC deposit address for {}",
        address
    );

    sleep(Duration::from_secs(2 * 5)).await;

    match mint_sats(&address.to_string()).await {
        Ok(_) => {
            println!("Successfully called mint_sats for address: {}", address);
        }
        Err(e) => {
            eprintln!("Failed to call get_nyks: {}", e);
            return Ok(());
        }
    };

    Ok(())
}
/// Generates a new Cosmos account using a custom Bech32 prefix.
///
/// Returns the private key (hex), public key (hex), and Bech32 address.
pub fn generate_cosmos_account(prefix: &str) -> anyhow::Result<(SigningKey, PublicKey, AccountId)> {
    // Generate a new secp256k1 private key
    let signing_key = SigningKey::random();
    let public_key = signing_key.public_key();
    let sha256_hash = Sha256::digest(public_key.clone().to_bytes());
    let ripemd160_hash = Ripemd160::digest(&sha256_hash);

    // Generate a Cosmos AccountId (Bech32 address) with custom prefix
    let account_id = AccountId::new(prefix, &ripemd160_hash).map_err(|e| anyhow::anyhow!(e))?;

    Ok((
        signing_key, // private key
        public_key,  // public key
        account_id,  // address (Bech32)
    ))
}

fn create_register_btc_deposit_message(
    btc_address: String,
    btc_amount: u64,
    twilight_amount: u64,
    twilight_address: String,
) -> Any {
    let msg = MsgRegisterBtcDepositAddress {
        btc_deposit_address: btc_address,
        btc_satoshi_test_amount: btc_amount,
        twilight_staking_amount: twilight_amount,
        twilight_address,
    };

    let mut buf = Vec::new();
    msg.encode(&mut buf).expect("msg encoding failed");

    Any {
        type_url: "/twilightproject.nyks.bridge.MsgRegisterBtcDepositAddress".to_string(),
        value: buf,
    }
}

#[derive(Deserialize, Debug)]
struct AccountResponse {
    account: Account,
}

#[derive(Deserialize, Debug)]
struct Account {
    #[serde(rename = "@type")]
    account_type: String,
    address: String,
    pub_key: Option<String>,
    account_number: String,
    sequence: String,
}

async fn fetch_account_details(address: &str) -> Result<AccountResponse, Box<dyn Error>> {
    // Define the endpoint URL
    let url = format!(
        "https://lcd.twilight.rest/cosmos/auth/v1beta1/accounts/{}",
        address
    );

    // Create an HTTP client
    let client = Client::new();

    // Send the GET request
    let response = client.get(&url).send().await?;

    // Check if the response is successful
    if response.status().is_success() {
        // Parse the JSON response
        let account_response: AccountResponse = response.json().await?;
        Ok(account_response)
    } else {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "No response body".to_string());
        Err(format!(
            "Failed to fetch account details. Status: {}, Error: {}",
            status, error_body
        )
        .into())
    }
}

async fn sign_and_send_reg_deposit_tx(
    signing_key: SigningKey,
    public_key: PublicKey,
    sender_account: String,
    btc_address: String,
) -> anyhow::Result<()> {
    // --- Build the Msg
    let msg_any =
        create_register_btc_deposit_message(btc_address, 50000, 10000, sender_account.clone());

    // --- Build Tx Body
    let body = Body::new(vec![msg_any], "", 0u16);

    let account_details = match fetch_account_details(&sender_account).await {
        Ok(account_response) => account_response, // Store the value in `account_details`
        Err(e) => {
            eprintln!("Error fetching account details: {}", e);
            return Err(anyhow::anyhow!("Failed to fetch account details"));
        }
    };

    let sequence = account_details
        .account
        .sequence
        .parse::<u64>()
        .map_err(|e| anyhow::anyhow!(e))?;
    let account_number = account_details
        .account
        .account_number
        .parse::<u64>()
        .map_err(|e| anyhow::anyhow!(e))?;
    let gas_limit = 200_000u64;
    let fee_amount = Coin {
        denom: cosmrs::Denom::from_str("nyks").map_err(|e| anyhow::anyhow!(e))?,
        amount: 1000u64.into(),
    };
    let signer_info = SignerInfo::single_direct(Some(public_key), sequence);
    let auth_info = signer_info.auth_info(Fee::from_amount_and_gas(fee_amount, gas_limit));

    // --- Sign
    let chain_id = Id::try_from("nyks").map_err(|e| anyhow::anyhow!(e))?;
    let sign_doc = SignDoc::new(&body, &auth_info, &chain_id, account_number)
        .map_err(|e| anyhow::anyhow!(e))?;
    let raw_tx = sign_doc
        .sign(&signing_key)
        .map_err(|e| anyhow::anyhow!(e))?;

    // --- Encode Tx to Base64
    let tx_bytes = raw_tx.to_bytes().map_err(|e| anyhow::anyhow!(e))?;
    let tx_base64 = base64::encode(tx_bytes);

    let client = reqwest::Client::new();
    let res = client
        .post("https://lcd.twilight.rest/cosmos/tx/v1beta1/txs")
        .json(&serde_json::json!({
            "tx_bytes": tx_base64,
            "mode": "BROADCAST_MODE_SYNC"
        }))
        .send()
        .await?;

    let response_text = res.text().await?;
    println!("Broadcast response:\n{}", response_text);

    Ok(())
}

pub async fn get_nyks(recipient_address: &str) -> Result<(), Box<dyn Error>> {
    // Define the endpoint URL
    let url = "https://faucet-rpc.twilight.rest/faucet";

    // Create the JSON payload
    let payload = json!({
        "recipientAddress": recipient_address
    });

    // Create an HTTP client
    let client = Client::new();

    // Send the POST request
    let response = client.post(url).json(&payload).send().await?;

    // Handle the response
    if response.status().is_success() {
        let body = response.text().await?;
        println!("Response: {}", body);
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
    // Define the endpoint URL
    let url = "https://faucet-rpc.twilight.rest/mint";

    // Create the JSON payload
    let payload = json!({
        "recipientAddress": recipient_address
    });

    // Create an HTTP client
    let client = Client::new();

    // Send the POST request
    let response = client.post(url).json(&payload).send().await?;

    // Handle the response
    if response.status().is_success() {
        let body = response.text().await?;
        println!("Response: {}", body);
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
