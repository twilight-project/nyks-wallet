use crate::{
    nyks_rpc::rpcclient::{
        method::{Method, MethodTypeURL},
        txrequest::{RpcBody, RpcRequest, TxParams},
        txresult::parse_tx_response,
    },
    relayer_module::{relayer_api::RelayerJsonRpcClient, relayer_types::TransactionHashArgs},
    zkos_accounts::ZkAccountDB,
    *,
};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use twilight_client_sdk::{
    relayer_rpcclient::method::UtxoDetailResponse,
    relayer_types::{OrderStatus, TxHash},
    zkvm::IOType,
};

// Retry configuration constants
const DEFAULT_UTXO_ATTEMPTS: u32 = 30;
const TXHASH_ATTEMPTS: u32 = 60;
const INITIAL_RETRY_DELAY_MS: u64 = 200;
const MAX_RETRY_DELAY_MS: u64 = 1_000;
const BACKOFF_FACTOR: f64 = 1.5;

/// Calculate retry delay with exponential backoff and jitter.
fn retry_delay(attempt: u32) -> Duration {
    let base = INITIAL_RETRY_DELAY_MS as f64 * BACKOFF_FACTOR.powi(attempt as i32);
    let capped = base.min(MAX_RETRY_DELAY_MS as f64);
    // Add ~30% jitter to avoid thundering herd
    let jitter = fastrand::f64() * capped * 0.1;
    Duration::from_millis((capped + jitter) as u64)
}

/// Constructs a `MsgMintBurnTradingBtc` for the given wallet/zk account, then signs it and
/// returns the base64-encoded transaction ready for broadcast.
pub fn build_and_sign_msg_mint_burn_trading_btc(
    wallet: &Wallet,
    zk_accounts: &ZkAccountDB,
    index: u64,
    sequence: u64,
    account_number: u64,
    amount: u64,
    mint_or_burn: bool,
) -> Result<String, String> {
    // Retrieve zk account (index is 1-based from setup)
    let account_idx = index;
    let zk_account = zk_accounts
        .get_account(&account_idx)
        .map_err(|e| e.to_string())?;

    // Build message
    let msg = MsgMintBurnTradingBtc {
        mint_or_burn,
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
pub async fn send_tx_to_chain(signed_tx: String, rpc_endpoint: &str) -> Result<TxResult, String> {
    // Prepare the RPC request body
    let method = Method::broadcast_tx_sync;
    let (tx_send, _): (RpcBody<TxParams>, String) =
        RpcRequest::new_with_data(TxParams::new(signed_tx.clone()), method, signed_tx);

    let rpc_endpoint = rpc_endpoint.to_string();

    // Execute the blocking HTTP request on a separate thread
    let response = tokio::task::spawn_blocking(move || tx_send.send(rpc_endpoint))
        .await
        .map_err(|e| format!("Failed to send RPC request: {}", e))?;

    let result = match response {
        Ok(rpc_response) => parse_tx_response(&method, rpc_response),
        Err(e) => {
            return Err(format!("Failed to get tx result: {}", e));
        }
    };
    match result {
        Ok(result) => {
            let tx_hash = result.get_tx_hash();
            let code = result.get_code();
            info!("tx hash: {} with code: {}", tx_hash, code);
            Ok(TxResult { tx_hash, code })
        }
        Err(e) => Err(format!("Failed to get tx result: {}", e)),
    }
}

/// Repeatedly queries the chain for UTXO details until success or `max_attempts` reached.
/// Uses exponential backoff with jitter between attempts.
pub async fn fetch_utxo_details_with_retry(
    account_id: String,
    io_type: IOType,
) -> Result<UtxoDetailResponse, String> {
    let mut attempts = 0;
    debug!("fetch_utxo_details_with_retry: account_id: {}", account_id);
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
                    if attempts >= DEFAULT_UTXO_ATTEMPTS {
                        error!(
                            "Failed to get utxo details after {} attempts: {} for account_id: {}",
                            DEFAULT_UTXO_ATTEMPTS, err, account_id
                        );
                        return Err(format!(
                            "Failed to get utxo details after {} attempts: {}",
                            DEFAULT_UTXO_ATTEMPTS, err
                        ));
                    }
                }
            },
            Err(e) => {
                attempts += 1;
                if attempts >= DEFAULT_UTXO_ATTEMPTS {
                    error!(
                        "Failed to spawn blocking task after {} attempts: {}",
                        DEFAULT_UTXO_ATTEMPTS, e
                    );
                    return Err(format!("Failed to spawn blocking task: {}", e));
                }
            }
        }
        sleep(retry_delay(attempts)).await;
    }
}
pub async fn fetch_utxo_details_with_once(
    account_id: String,
    io_type: IOType,
) -> Result<UtxoDetailResponse, String> {
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
                return Err(format!("Failed to get utxo details: {}", err));
            }
        },
        Err(e) => {
            return Err(format!("Failed to spawn blocking task: {}", e));
        }
    }
}

pub async fn fetch_tx_hash_with_retry(
    request_id: &str,
    relayer_api_client: &RelayerJsonRpcClient,
) -> Result<TxHash, String> {
    let mut attempts = 0;
    loop {
        let response = relayer_api_client
            .transaction_hashes(TransactionHashArgs::RequestId {
                id: request_id.to_string(),
                status: None,
                limit: None,
                offset: None,
            })
            .await
            .map_err(|e| e.to_string())?;
        if response.is_empty() {
            attempts += 1;
            if attempts >= TXHASH_ATTEMPTS {
                return Err(format!(
                    "Failed to get tx hash after {} attempts",
                    TXHASH_ATTEMPTS
                ));
            }
            sleep(retry_delay(attempts)).await;
        } else {
            let latest_tx = response
                .iter()
                .max_by_key(|tx| (tx.datetime.trim().parse::<i64>().unwrap_or(i64::MIN), tx.id))
                .unwrap_or(&response[0]);

            return Ok(latest_tx.clone());
        }
    }
}
pub async fn fetch_tx_hash_with_once(
    request_id: &str,
    relayer_api_client: &RelayerJsonRpcClient,
) -> Result<TxHash, String> {
    let response = relayer_api_client
        .transaction_hashes(TransactionHashArgs::RequestId {
            id: request_id.to_string(),
            status: None,
            limit: None,
            offset: None,
        })
        .await
        .map_err(|e| e.to_string())?;
    if response.is_empty() {
        return Err(
            "Failed to get tx hash, Order may be in the queue, try again later".to_string(),
        );
    } else {
        let latest_tx = response
            .iter()
            .max_by_key(|tx| (tx.datetime.trim().parse::<i64>().unwrap_or(i64::MIN), tx.id))
            .unwrap_or(&response[0]);

        return Ok(latest_tx.clone());
    }
}

pub async fn fetch_tx_hash_with_retry_with_close_order(
    request_id: &str,
    relayer_api_client: &RelayerJsonRpcClient,
    _order_type: twilight_client_sdk::relayer_types::OrderType,
) -> Result<TxHash, String> {
    let mut attempts = 0;
    loop {
        let response = relayer_api_client
            .transaction_hashes(TransactionHashArgs::RequestId {
                id: request_id.to_string(),
                status: None,
                limit: None,
                offset: None,
            })
            .await
            .map_err(|e| e.to_string())?;
        if response.is_empty() {
            attempts += 1;
            if attempts >= TXHASH_ATTEMPTS {
                return Err(format!(
                    "Failed to get tx hash after {} attempts",
                    TXHASH_ATTEMPTS
                ));
            }
            sleep(retry_delay(attempts)).await;
        } else {
            // Filter out specific cancelled order statuses
            let filtered_response: Vec<_> = response
                .iter()
                .filter(|tx| {
                    let status = tx.order_status.to_str();
                    status != "CancelledLimitClose"
                        && status != "CancelledStopLoss"
                        && status != "CancelledTakeProfit"
                })
                .collect();

            if filtered_response.is_empty() {
                // Do not return; continue to retry in the outer loop
                attempts += 1;
                if attempts >= TXHASH_ATTEMPTS {
                    return Err(format!(
                        "Failed to get tx hash after {} attempts",
                        TXHASH_ATTEMPTS
                    ));
                }
                sleep(retry_delay(attempts)).await;
                continue;
            }

            // Check if any transaction has order_status == SETTLED. If so, return it.
            let settled_tx = filtered_response
                .iter()
                .find(|tx| tx.order_status == OrderStatus::SETTLED)
                .copied();
            let latest_tx = if let Some(tx) = settled_tx {
                tx
            } else {
                filtered_response
                    .iter()
                    .max_by_key(|tx| (tx.datetime.trim().parse::<i64>().unwrap_or(i64::MIN), tx.id))
                    .copied()
                    .unwrap_or(filtered_response[0])
            };

            return Ok(latest_tx.clone());
        }
    }
}
pub async fn fetch_tx_hash_with_account_address_retry(
    account_address: &str,
    order_status: Option<OrderStatus>,
    relayer_api_client: &RelayerJsonRpcClient,
) -> Result<TxHash, String> {
    let mut attempts = 0;
    loop {
        let response = relayer_api_client
            .transaction_hashes(TransactionHashArgs::AccountId {
                id: account_address.to_string(),
                status: order_status.clone(),
                limit: None,
                offset: None,
            })
            .await
            .map_err(|e| e.to_string())?;
        if response.is_empty() {
            attempts += 1;
            if attempts >= TXHASH_ATTEMPTS {
                return Err(format!(
                    "Failed to get tx hash after {} attempts",
                    TXHASH_ATTEMPTS
                ));
            }
            sleep(retry_delay(attempts)).await;
        } else {
            let latest_tx = response
                .iter()
                .max_by_key(|tx| (tx.datetime.trim().parse::<i64>().unwrap_or(i64::MIN), tx.id))
                .unwrap_or(&response[0]);

            return Ok(latest_tx.clone());
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[must_use]
pub struct TxResult {
    pub tx_hash: String,
    pub code: u32,
}

/// Repeatedly queries the chain for UTXO details until the UTXO is removed (not found)
/// or `max_attempts` reached. Uses exponential backoff with jitter.
pub async fn fetch_removed_utxo_details_with_retry(
    account_id: String,
    io_type: IOType,
) -> Result<(), String> {
    let mut attempts = 0;
    debug!(
        "fetch_removed_utxo_details_with_retry: account_id: {}",
        account_id
    );
    loop {
        let account_id_clone = account_id.clone();
        match tokio::task::spawn_blocking(move || {
            twilight_client_sdk::chain::get_utxo_details_by_address(account_id_clone, io_type)
        })
        .await
        {
            Ok(response) => match response {
                Err(err) => {
                    if err.contains("UTXO not found") {
                        return Ok(());
                    } else {
                        return Err(format!("Failed to remove utxo details: {}", err));
                    }
                }
                Ok(_) => {
                    if attempts == 0 {
                        sleep(Duration::from_secs(2)).await;
                    }
                    attempts += 1;
                    if attempts >= DEFAULT_UTXO_ATTEMPTS {
                        return Err(format!(
                            "Failed to remove utxo details after {} attempts: {}",
                            DEFAULT_UTXO_ATTEMPTS, account_id
                        ));
                    }
                }
            },
            Err(e) => {
                attempts += 1;
                if attempts >= DEFAULT_UTXO_ATTEMPTS {
                    error!(
                        "Failed to spawn blocking task after {} attempts: {}",
                        DEFAULT_UTXO_ATTEMPTS, e
                    );
                    return Err(format!("Failed to spawn blocking task: {}", e));
                }
            }
        }
        sleep(retry_delay(attempts)).await;
    }
}
