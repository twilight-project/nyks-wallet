use log::{debug, error, info};
use nyks_wallet::{
    nyks_rpc::rpcclient::{
        method::{Method, MethodTypeURL},
        txrequest::{RpcBody, RpcRequest, TxParams},
        txresult::{from_rpc_response, parse_tx_response},
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

/// Initializes a new wallet, requests test tokens from the faucet, and waits until the
/// balance is reflected on-chain.
async fn setup_wallet() -> Result<Wallet, String> {
    info!("Creating new wallet with random BTC address");
    let mut wallet = Wallet::create_new_with_random_btc_address()
        .await
        .map_err(|e| e.to_string())?;
    // info!("importing wallet from json");
    // let mut wallet = Wallet::import_from_json("test.json").map_err(|e| e.to_string())?;
    info!("Getting test tokens from faucet");
    match get_test_tokens(&mut wallet).await {
        Ok(_) => info!("Tokens received successfully"),
        Err(e) => return Err(e.to_string()),
    }

    // Give the faucet some time to finalize and the indexer to catch up.
    sleep(Duration::from_secs(5)).await;

    Ok(wallet)
}

/// Generates a seed signature, loads or initializes the `ZkAccountDB`, creates a new zk
/// account, persists the DB to disk, and returns `(zk_accounts, index, seed_signature)`.
fn setup_zk_accounts(
    wallet: &Wallet,
    chain_id: &str,
) -> Result<(ZkAccountDB, u64, String), String> {
    // Generate seed signature
    let seed_signature = generate_seed(
        &wallet.private_key,
        &wallet.twilightaddress,
        DERIVATION_MESSAGE,
        chain_id,
    )
    .map_err(|e| format!("Failed to generate seed: {}", e))?
    .get_signature();

    // Load or create db
    info!("Loading ZkAccountDB ...");
    let mut zk_accounts = ZkAccountDB::import_from_json("ZkAccounts.json").unwrap_or_else(|_e| {
        info!("    Old ZkAccountDB not found. Creating new DB...");
        ZkAccountDB::new()
    });

    // Create new zk account
    let index = zk_accounts
        .generate_new_account(wallet.balance_sats, seed_signature.clone())
        .map_err(|e| format!("Failed to generate new zk account: {}", e))?;
    info!("    New zk account generated with index: {}", index);
    // Persist DB
    info!("    Exporting ZkAccountDB to file...");
    zk_accounts
        .export_to_json("ZkAccounts.json")
        .map_err(|e| format!("Failed to export to json: {}", e))?;

    Ok((zk_accounts, index, seed_signature))
}

/// Constructs a `MsgMintBurnTradingBtc` for the given wallet/zk account, then signs it and
/// returns the base64-encoded transaction ready for broadcast.
fn build_and_sign_msg(
    wallet: &Wallet,
    zk_accounts: &ZkAccountDB,
    index: u64,
    sequence: u64,
    account_number: u64,
) -> Result<String, String> {
    // Retrieve zk account (index is 1-based from setup)
    let account_idx = index;
    let zk_account = zk_accounts
        .get_account(&account_idx)
        .ok_or_else(|| format!("Failed to get zk account: {}", account_idx))?;

    // Build message
    let msg = MsgMintBurnTradingBtc {
        mint_or_burn: true,
        btc_value: wallet.balance_sats,
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
async fn send_rpc_request(signed_tx: String) -> Result<(), String> {
    // Prepare the RPC request body
    let method = Method::broadcast_tx_sync;
    let (tx_send, _): (RpcBody<TxParams>, String) =
        RpcRequest::new_with_data(TxParams::new(signed_tx.clone()), method, signed_tx);

    // RPC endpoint URL (consider moving to an env var later)
    let url = "https://rpc.twilight.rest".to_string();

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
            info!(
                "tx hash: {} with code: {}",
                result.get_tx_hash(),
                result.get_code()
            );
        }
        Err(e) => {
            return Err(format!("Failed to get tx result: {}", e));
        }
    }
    Ok(())
}

/// Repeatedly queries the chain for UTXO details until success or `max_attempts` reached.
async fn fetch_utxo_details_with_retry(
    account_id: String,
    max_attempts: u32,
    delay_ms: u64,
) -> Result<(), String> {
    let mut attempts = 0;
    loop {
        let account_id_clone = account_id.clone();
        match tokio::task::spawn_blocking(move || {
            twilight_client_sdk::chain::get_utxo_details_by_address(account_id_clone, IOType::Coin)
        })
        .await
        {
            Ok(response) => match response {
                Ok(utxo_detail) => {
                    debug!("utxo_detail: {:?}, account_id: {}", utxo_detail, account_id);
                    return Ok(());
                }
                Err(err) => {
                    attempts += 1;
                    if attempts >= max_attempts {
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
                    return Err(format!("Failed to spawn blocking task: {}", e));
                }
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
    }
}

/// Serializes and writes relayer deployment data to `relayer_deployer.json`.
fn export_relayer_data(
    zk_account: &ZkAccount,
    index: u64,
    output_state: &Output,
    seed_signature: &str,
) -> Result<(), String> {
    let out_state_bin = bincode::serialize(output_state)
        .map_err(|e| format!("Failed to serialize output_state: {}", e))?;
    let out_state_hex = hex::encode(&out_state_bin);

    debug!("out_state_hex {:?}", out_state_hex);
    debug!("index: {}", index);

    let relayer_data = serde_json::json!({
        "zkaccount": zk_account,
        "index": index,
        "out_state_hex": out_state_hex,
        "seed_signature": seed_signature,
    });

    let json_str = serde_json::to_string_pretty(&relayer_data)
        .map_err(|e| format!("Failed to serialize relayer data to JSON: {}", e))?;

    std::fs::write("relayer_deployer.json", json_str)
        .map_err(|e| format!("Failed to write relayer data to file: {}", e))?;

    info!("Successfully wrote relayer data to relayer_deployer.json");
    info!("You can now use this file to deploy the relayer-core");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenv::dotenv().ok();
    // Initialize logger (controlled via RUST_LOG env var). Defaults to 'info' level.
    env_logger::init();
    let wallet = setup_wallet().await?;
    let chain_id = "nyks";
    let (mut zk_accounts, index, seed_signature) = setup_zk_accounts(&wallet, chain_id)?;
    info!("Fetching account info from nyks chain...");
    let account_details = match wallet.account_info().await {
        Ok(details) => details,
        Err(e) => {
            error!("Failed to get account info: {}", e);
            return Err(format!("Failed to get account info: {}", e));
        }
    };
    let sequence = match account_details.account.sequence.parse::<u64>() {
        Ok(seq) => seq,
        Err(e) => {
            error!("Failed to parse sequence: {}", e);
            return Err(format!("Failed to parse sequence: {}", e));
        }
    };
    let account_number = match account_details.account.account_number.parse::<u64>() {
        Ok(num) => num,
        Err(e) => {
            error!("Failed to parse account number: {}", e);
            return Err(format!("Failed to parse account number: {}", e));
        }
    };
    // Build and sign message
    info!("Building and signing message for Funding to zk account transfer");
    let signed_tx = build_and_sign_msg(&wallet, &zk_accounts, index, sequence, account_number)?;
    info!("    Message signed successfully");
    // Broadcast the transaction
    info!("Broadcasting transaction...");
    send_rpc_request(signed_tx.clone()).await?;
    info!("    Transaction broadcasted successfully");

    // Retrieve the zk account and ensure it exists
    let zk_account = match zk_accounts.get_account(&(index)) {
        Some(account) => account.clone(),
        None => {
            error!("Failed to get zk account: {}", index);
            return Err(format!("Failed to get zk account: {}", index));
        }
    };

    // Wait for UTXO details to appear on-chain
    info!("    Waiting for UTXO details to appear on-chain...");
    let account_id = zk_account.clone().account.clone();
    fetch_utxo_details_with_retry(account_id.clone(), 100, 500).await?;

    // Deploy the relayer initial state
    let seed_signature_clone = seed_signature.clone();
    let zk_account_clone = zk_account.clone();
    info!("Creating relayer initial state transaction");
    let (tx, output_state) = match tokio::task::spawn_blocking(move || {
        deploy_relayer_initial_state(
            account_id.clone(),
            zk_account_clone.scalar.clone(),
            zk_account_clone.balance.clone(),
            seed_signature.clone(),
            index,
        )
    })
    .await
    {
        Ok(result) => match result {
            Ok((tx, output_state)) => (tx, output_state),
            Err(e) => {
                return Err(format!("Failed to deploy relayer initial state: {}", e));
            }
        },
        Err(e) => {
            return Err(format!("Failed to spawn blocking task: {}", e));
        }
    };

    // Broadcast the deployment transaction
    info!("    Broadcasting deployment transaction...");
    let tx_clone = tx.clone();
    let broadcast_result = match tokio::task::spawn_blocking(move || {
        twilight_client_sdk::chain::tx_commit_broadcast_transaction(tx_clone)
    })
    .await
    {
        Ok(result) => match result {
            Ok(hash) => hash,
            Err(e) => {
                return Err(format!("Failed to broadcast transaction: {}", e));
            }
        },
        Err(e) => {
            return Err(format!("Failed to spawn blocking task: {}", e));
        }
    };
    info!(
        "    Transaction broadcast result with transaction hash: {}",
        broadcast_result
    );
    info!(
        "    Updating ZkAccountDB on index : {} with io_type: State",
        index
    );
    match zk_accounts.get_mut_account(&index) {
        Some(account) => {
            account.io_type = IOType::State;
            zk_accounts.export_to_json("ZkAccounts.json")?;
        }
        None => {
            return Err(format!("Failed to get zk account: {}", index));
        }
    }
    let updated_zk_account = zk_accounts.get_account(&index).unwrap();
    info!("    Exporting relayer data to file...");
    export_relayer_data(
        &updated_zk_account,
        index,
        &output_state,
        &seed_signature_clone,
    )?;

    Ok(())
}

pub fn deploy_relayer_initial_state(
    account: String,
    scalar: String,
    balance: u64,
    seed: String,
    index: u64,
) -> Result<(Transaction, Output), String> {
    let key_manager = KeyManager::from_cosmos_signature(seed.as_bytes());

    let secret_key = key_manager.derive_child_key(index);
    let encryption_commitment_scalar = match util::hex_to_scalar(scalar.to_string()) {
        Some(scalar) => scalar,
        None => {
            return Err("Failed to convert scalar_str to scalar".to_string());
        }
    };
    let program_json_path: &str = "./relayerprogram.json";
    let chain_net = address::Network::default();
    let state_variables: Vec<u64> = vec![balance.clone() / 100];
    let program_tag: String = "RelayerInitializer".to_string();
    let pool_share = balance.clone() / 100;
    let tx = script::create_contract_deploy_transaction(
        secret_key,
        balance,
        pool_share,
        account,
        encryption_commitment_scalar,
        program_json_path,
        chain_net,
        state_variables,
        program_tag,
        1u64,
    );
    let (tx, out_state) = match tx {
        Ok((tx, out_state)) => (tx, out_state),
        Err(e) => {
            return Err(e.to_string());
        }
    };
    Ok((tx, out_state))
}
