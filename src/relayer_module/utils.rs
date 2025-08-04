use crate::{
    nyks_rpc::rpcclient::{
        method::{Method, MethodTypeURL},
        txrequest::{NYKS_RPC_BASE_URL, RpcBody, RpcRequest, TxParams},
        txresult::parse_tx_response,
    },
    relayer_module::{relayer_api::RelayerJsonRpcClient, relayer_types::TransactionHashArgs},
    zkos_accounts::ZkAccountDB,
    *,
};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, sleep};
use twilight_client_sdk::{
    relayer_rpcclient::method::UtxoDetailResponse, relayer_types::TxHash, zkvm::IOType,
};
/// Constructs a `MsgMintBurnTradingBtc` for the given wallet/zk account, then signs it and
/// returns the base64-encoded transaction ready for broadcast.
pub fn build_and_sign_msg_mint_burn_trading_btc(
    wallet: &Wallet,
    zk_accounts: &ZkAccountDB,
    index: u64,
    sequence: u64,
    account_number: u64,
    amount: u64,
) -> Result<String, String> {
    // Retrieve zk account (index is 1-based from setup)
    let account_idx = index;
    let zk_account = zk_accounts
        .get_account(&account_idx)
        .map_err(|e| e.to_string())?;

    // Build message
    let msg = MsgMintBurnTradingBtc {
        mint_or_burn: true,
        btc_value: amount,
        qq_account: zk_account.qq_address.clone(),
        encrypt_scalar: zk_account.scalar.clone(),
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

/// Broadcasts the signed transaction to the NYKS RPC endpoint and logs the response.
pub async fn send_tx_to_chain(signed_tx: String) -> Result<TxResult, String> {
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
    // let result = parse_tx_response(tx_send.get_method(), response);
    match result {
        Ok(result) => {
            let tx_hash = result.get_tx_hash();
            let code = result.get_code();
            info!("tx hash: {} with code: {}", tx_hash, code);
            Ok(TxResult { tx_hash, code })
        }
        Err(e) => {
            return Err(format!("Failed to get tx result: {}", e));
        }
    }
}

/// Repeatedly queries the chain for UTXO details until success or `max_attempts` reached.
pub async fn fetch_utxo_details_with_retry(
    account_id: String,
    max_attempts: u32,
    delay_ms: u64,
    io_type: IOType,
) -> Result<UtxoDetailResponse, String> {
    let mut attempts = 0;
    loop {
        let account_id_clone = account_id.clone();
        match tokio::task::spawn_blocking(move || {
            twilight_client_sdk::chain::get_utxo_details_by_address(account_id_clone, io_type)
        })
        .await
        {
            Ok(response) => match response {
                Ok(utxo_detail) => {
                    debug!("utxo_detail: {:?}, account_id: {}", utxo_detail, account_id);
                    return Ok(utxo_detail);
                }
                Err(err) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        error!(
                            "Failed to get utxo details after {} attempts: {}",
                            max_attempts, err
                        );
                        return Err(format!(
                            "Failed to get utxo details after {} attempts: {}",
                            max_attempts, err
                        ));
                    }
                }
            },
            Err(e) => {
                attempts += 1;
                if attempts >= max_attempts {
                    error!(
                        "Failed to spawn blocking task after {} attempts: {}",
                        max_attempts, e
                    );
                    return Err(format!("Failed to spawn blocking task: {}", e));
                }
            }
        }
        sleep(Duration::from_millis(delay_ms)).await;
    }
}

pub async fn fetch_tx_hash_with_retry(
    request_id: &str,
    max_attempts: u32,
    delay_ms: u64,
) -> Result<TxHash, String> {
    let mut attempts = 0;
    loop {
        let relayer_connection =
            RelayerJsonRpcClient::new("https://relayer.twilight.rest/api").unwrap();
        let response = relayer_connection
            .transaction_hashes(TransactionHashArgs::RequestId {
                id: request_id.to_string(),
                status: None,
            })
            .await
            .map_err(|e| e.to_string())?;
        if response.len() == 0 {
            attempts += 1;
            if attempts >= max_attempts {
                return Err(format!(
                    "Failed to get tx hash after {} attempts",
                    max_attempts
                ));
            }
            sleep(Duration::from_millis(delay_ms)).await;
        } else {
            return Ok(response[0].clone());
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TxResult {
    pub tx_hash: String,
    pub code: u32,
}
