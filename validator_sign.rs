use log::{debug, error, info};
use nyks_wallet::{
    nyks_rpc::rpcclient::{
        method::{Method, MethodTypeURL},
        txrequest::{NYKS_RPC_BASE_URL, RpcBody, RpcRequest, TxParams},
        txresult::parse_tx_response,
    },
    zkos_accounts::{
        ZkAccount, ZkAccountDB,
        encrypted_account::{DERIVATION_MESSAGE, KeyManager},
    },
    *,
};

use tokio::time::{Duration, sleep};
use twilight_client_sdk::{
    script,
    transaction::Transaction,
    util,
    zkvm::{IOType, Output},
};

async fn setup_wallet() -> Result<Wallet, String> {
    info!("Creating new wallet with random BTC address");
    let mut wallet =
        Wallet::from_mnemonic_file("validator-self.mnemonic").map_err(|e| e.to_string())?;
    wallet
        .update_account_info()
        .await
        .map_err(|e| e.to_string())?;
    Ok(wallet)
}
fn build_and_sign_msg_transfer_tx(
    wallet: &Wallet,
    tx_id: String,
    tx_byte_code: String,
    tx_fee: u64,
    sequence: u64,
    account_number: u64,
) -> Result<String, String> {
    // Retrieve zk account (index is 1-based from setup)

    // Build message
    let msg = MsgTransferTx {
        tx_id: tx_id,
        tx_byte_code: tx_byte_code,
        tx_fee: tx_fee,
        zk_oracle_address: wallet.twilightaddress.clone(),
    };

    // Serialize into Any and sign
    let method_type = MethodTypeURL::MsgTransferTx;
    let any_msg = method_type.type_url(msg);

    let sk = wallet
        .signing_key()
        .map_err(|e| format!("Failed to get signing key: {}", e))?;
    let pk = wallet
        .public_key()
        .map_err(|e| format!("Failed to get public key: {}", e))?;

    let signed_tx = method_type
        .sign_msg::<MsgTransferTx>(any_msg, pk, sequence, account_number, sk)
        .map_err(|e| e.to_string())?;

    Ok(signed_tx)
}

fn build_and_sign_msg_mint_burn_trading_btc(
    wallet: &Wallet,
    mint_or_burn: bool,
    btc_value: u64,
    qq_account: String,
    encrypt_scalar: String,
    sequence: u64,
    account_number: u64,
) -> Result<String, String> {
    // Build message
    let msg = MsgMintBurnTradingBtc {
        mint_or_burn: mint_or_burn,
        btc_value: btc_value,
        qq_account: qq_account,
        encrypt_scalar: encrypt_scalar,
        twilight_address: wallet.twilightaddress.clone(),
    };

    // Serialize into Any and sign
    let method_type = MethodTypeURL::MsgMintBurnTradingBtc;
    let any_msg = method_type.type_url(msg);

    let sk = wallet
        .signing_key()
        .map_err(|e| format!("Failed to get signing key: {}", e))?;
    let pk = wallet
        .public_key()
        .map_err(|e| format!("Failed to get public key: {}", e))?;

    let signed_tx = method_type
        .sign_msg::<MsgMintBurnTradingBtc>(any_msg, pk, sequence, account_number, sk)
        .map_err(|e| e.to_string())?;

    Ok(signed_tx)
}

async fn send_rpc_request(signed_tx: String) -> Result<(String, u32), String> {
    // Prepare the RPC request body
    let method = Method::broadcast_tx_sync;
    let (tx_send, _): (RpcBody<TxParams>, String) =
        RpcRequest::new_with_data(TxParams::new(signed_tx.clone()), method, signed_tx);

    // RPC endpoint URL (consider moving to an env var later)
    let url = NYKS_RPC_BASE_URL.to_string();

    // Execute the blocking HTTP request on a separate thread
    let response = tokio::task::spawn_blocking(move || tx_send.send(url))
        .await
        .map_err(|e| format!("Failed to send RPC request: {}", e))?;

    let result = match response {
        Ok(rpc_response) => parse_tx_response(&method, rpc_response),
        Err(e) => {
            return Err(format!("Failed to get tx result: {}", e));
        }
    };
    let tx_hash;
    let tx_code;
    // let result = parse_tx_response(tx_send.get_method(), response);
    match result {
        Ok(result) => {
            info!(
                "tx hash: {} with code: {}",
                result.get_tx_hash(),
                result.get_code()
            );
            tx_hash = result.get_tx_hash();
            tx_code = result.get_code();
        }
        Err(e) => {
            return Err(format!("Failed to get tx result: {}", e));
        }
    }
    Ok((tx_hash, tx_code))
}
pub async fn transfer_tx(
    tx_id: String,
    tx_byte_code: String,
    tx_fee: u64,
) -> Result<(String, u32), String> {
    dotenv::dotenv().ok();
    // Initialize logger (controlled via RUST_LOG env var). Defaults to 'info' level.
    env_logger::init();
    let wallet = setup_wallet().await.map_err(|e| e.to_string())?;

    // println!("wallet: {:?}", wallet);
    let signed_tx =
        build_and_sign_msg_transfer_tx(&wallet, tx_id, tx_byte_code, tx_fee, wallet.sequence, {
            let account_info = wallet
                .account_info
                .as_ref()
                .ok_or("Account info not available")?;
            account_info.account_number
        })?;
    let (tx_hash, tx_code) = send_rpc_request(signed_tx).await?;
    info!("tx hash: {}", tx_hash);
    info!("tx code: {}", tx_code);
    Ok((tx_hash, tx_code))
}

pub async fn mint_burn_trading_btc_tx(
    mint_or_burn: bool,
    btc_value: u64,
    qq_account: String,
    encrypt_scalar: String,
) -> Result<(String, u32), String> {
    dotenv::dotenv().ok();
    // Initialize logger (controlled via RUST_LOG env var). Defaults to 'info' level.
    env_logger::init();
    let wallet = setup_wallet().await.map_err(|e| e.to_string())?;
    let signed_tx = build_and_sign_msg_mint_burn_trading_btc(
        &wallet,
        mint_or_burn,
        btc_value,
        qq_account,
        encrypt_scalar,
        wallet.sequence,
        {
            let account_info = wallet
                .account_info
                .as_ref()
                .ok_or("Account info not available")?;
            account_info.account_number
        },
    )?;
    let (tx_hash, tx_code) = send_rpc_request(signed_tx).await?;
    info!("tx hash: {}", tx_hash);
    info!("tx code: {}", tx_code);
    Ok((tx_hash, tx_code))
}
