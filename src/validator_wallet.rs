use super::{
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
use log::{debug, error, info};

use tokio::time::{Duration, sleep};
use twilight_client_sdk::{
    script,
    transaction::Transaction,
    util,
    zkvm::{IOType, Output},
};

// Re-export primary async helpers when the `validator-wallet` feature is enabled.
// These allow external callers to build & broadcast validator-side transactions.

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------
async fn setup_wallet() -> Result<Wallet, String> {
    info!("Creating new wallet with random BTC address");
    let mnemonic_file =
        std::env::var("VALIDATOR_WALLET_PATH").unwrap_or("validator.mnemonic".to_string());
    let mut wallet =
        Wallet::from_mnemonic_file(mnemonic_file.as_str()).map_err(|e| e.to_string())?;
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
    let msg = MsgTransferTx {
        tx_id,
        tx_byte_code,
        tx_fee,
        zk_oracle_address: wallet.twilightaddress.clone(),
    };

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
    let msg = MsgMintBurnTradingBtc {
        mint_or_burn,
        btc_value,
        qq_account,
        encrypt_scalar,
        twilight_address: wallet.twilightaddress.clone(),
    };

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
    let method = Method::broadcast_tx_sync;
    let (tx_send, _): (RpcBody<TxParams>, String) =
        RpcRequest::new_with_data(TxParams::new(signed_tx.clone()), method, signed_tx);
    let url = NYKS_RPC_BASE_URL.to_string();

    let response = tokio::task::spawn_blocking(move || tx_send.send(url))
        .await
        .map_err(|e| format!("Failed to send RPC request: {}", e))?;

    let result = match response {
        Ok(rpc_response) => parse_tx_response(&method, rpc_response),
        Err(e) => return Err(format!("Failed to get tx result: {}", e)),
    };

    let (tx_hash, tx_code) = match result {
        Ok(r) => (r.get_tx_hash(), r.get_code()),
        Err(e) => return Err(format!("Failed to get tx result: {}", e)),
    };

    info!("tx hash: {}", tx_hash);
    info!("tx code: {}", tx_code);

    Ok((tx_hash, tx_code))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------
/// Broadcast a MsgTransferTx transaction built from the provided parameters.
#[cfg(feature = "validator-wallet")]
pub async fn transfer_tx(
    tx_id: String,
    tx_byte_code: String,
    tx_fee: u64,
) -> Result<(String, u32), String> {
    dotenv::dotenv().ok();
    env_logger::init();

    let wallet = setup_wallet().await?;
    let signed_tx = build_and_sign_msg_transfer_tx(
        &wallet,
        tx_id,
        tx_byte_code,
        tx_fee,
        wallet.sequence,
        wallet
            .account_info
            .as_ref()
            .ok_or("Account info not available")?
            .account_number,
    )?;

    send_rpc_request(signed_tx).await
}

/// Broadcast a MsgMintBurnTradingBtc transaction to mint or burn BTC in zkOS trading.
#[cfg(feature = "validator-wallet")]
pub async fn mint_burn_trading_btc_tx(
    mint_or_burn: bool,
    btc_value: u64,
    qq_account: String,
    encrypt_scalar: String,
) -> Result<(String, u32), String> {
    dotenv::dotenv().ok();
    env_logger::init();

    let wallet = setup_wallet().await?;
    let signed_tx = build_and_sign_msg_mint_burn_trading_btc(
        &wallet,
        mint_or_burn,
        btc_value,
        qq_account,
        encrypt_scalar,
        wallet.sequence,
        wallet
            .account_info
            .as_ref()
            .ok_or("Account info not available")?
            .account_number,
    )?;

    send_rpc_request(signed_tx).await
}
