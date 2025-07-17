use tokio::time::{Duration, sleep};

use nyks_wallet::{
    nyks_rpc::rpcclient::{
        method::{Method, MethodTypeURL},
        txrequest::{RpcBody, RpcRequest, TxParams},
    },
    zkos_accounts::{
        ZkAccountDB,
        encrypted_account::{DERIVATION_MESSAGE, KeyManager},
    },
    *,
};
use twilight_client_sdk::{
    script,
    transaction::Transaction,
    util,
    zkvm::{IOType, Output},
};

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenv::dotenv().ok();
    let mut wallet = match Wallet::create_new_with_random_btc_address().await {
        Ok(wallet) => wallet,
        Err(e) => {
            println!("Error: {}", e);
            return Err(e.to_string());
        }
    };
    match get_test_tokens(&mut wallet).await {
        Ok(_) => println!("Tokens received"),
        Err(e) => return Err(e.to_string()),
    }
    sleep(Duration::from_secs(5)).await;
    let chain_id = "nyks";
    let seed_signature = match generate_seed(
        &wallet.private_key,
        &wallet.twilightaddress,
        DERIVATION_MESSAGE,
        chain_id,
    ) {
        Ok(seed) => seed.get_signature(),
        Err(e) => return Err(format!("Failed to generate seed: {}", e)),
    };
    let mut zk_accounts = match ZkAccountDB::import_from_json("ZkAccounts.json") {
        Ok(db) => db,
        Err(e) => {
            println!("Failed to import from json: {}", e);
            ZkAccountDB::new()
        }
    };
    let index = zk_accounts
        .generate_new_account(wallet.balance_sats, seed_signature.clone())
        .unwrap();
    println!("index: {}", index);
    match zk_accounts.export_to_json("ZkAccounts.json") {
        Ok(_) => println!("ZkAccounts.json exported"),
        Err(e) => return Err(format!("Failed to export to json: {}", e)),
    }
    let account_details = match wallet.account_info().await {
        Ok(details) => details,
        Err(e) => {
            println!("Failed to get account info: {}", e);
            return Err(format!("Failed to get account info: {}", e));
        }
    };
    let sequence = match account_details.account.sequence.parse::<u64>() {
        Ok(seq) => seq,
        Err(e) => {
            println!("Failed to parse sequence: {}", e);
            return Err(format!("Failed to parse sequence: {}", e));
        }
    };
    let account_number = match account_details.account.account_number.parse::<u64>() {
        Ok(num) => num,
        Err(e) => {
            println!("Failed to parse account number: {}", e);
            return Err(format!("Failed to parse account number: {}", e));
        }
    };
    // Create test message
    let msg = MsgMintBurnTradingBtc {
        mint_or_burn: true,
        btc_value: wallet.balance_sats,
        qq_account: zk_accounts
            .get_account(&(index - 1))
            .unwrap()
            .qq_address
            .clone(),
        encrypt_scalar: zk_accounts
            .get_account(&(index - 1))
            .unwrap()
            .scalar
            .clone(),
        twilight_address: wallet.twilightaddress.clone(),
    };

    // Create method type and get Any message
    let method_type = MethodTypeURL::MsgMintBurnTradingBtc;
    let any_msg = method_type.type_url(msg);
    let sk = match wallet.signing_key() {
        Ok(sk) => sk,
        Err(e) => {
            println!("Failed to get signing key: {}", e);
            return Err(format!("Failed to get signing key: {}", e));
        }
    };
    let pk = match wallet.public_key() {
        Ok(pk) => pk,
        Err(e) => {
            println!("Failed to get public key: {}", e);
            return Err(format!("Failed to get public key: {}", e));
        }
    };
    // Sign the message
    let signed_tx = method_type
        .sign_msg::<MsgMintBurnTradingBtc>(any_msg, pk, sequence, account_number, sk)
        .unwrap();

    // Create RPC request
    let (tx_send, _data): (RpcBody<TxParams>, String) = RpcRequest::new_with_data(
        TxParams::new(signed_tx.clone()),
        Method::broadcast_tx_sync,
        signed_tx,
    );

    // Send RPC request
    // let response = tx_send.send("https://rpc.twilight.rest".to_string(), data);
    let url = "https://rpc.twilight.rest".to_string();
    let response = match tokio::task::spawn_blocking(move || tx_send.send(url)).await {
        Ok(response) => response,
        Err(e) => {
            println!("Failed to send RPC request: {}", e);
            return Err(format!("Failed to send RPC request: {}", e));
        }
    };

    println!("response: {:?}", response);
    let zk_account = match zk_accounts.get_account(&(index - 1)) {
        Some(account) => account.clone(),
        None => {
            println!("Failed to get zk account: {}", index);
            return Err(format!("Failed to get zk account: {}", index));
        }
    };
    let mut chain_attempt = 0;
    let utxo_output;
    let account_id = zk_account.clone().account.clone();
    loop {
        let account_id_clone = account_id.clone();
        match tokio::task::spawn_blocking(move || {
            twilight_client_sdk::chain::get_utxo_details_by_address(account_id_clone, IOType::Coin)
        })
        .await
        {
            Ok(response) => {
                match response {
                    Ok(utxo_detail) => {
                        println!("utxo_detail: {:?}, account_id: {}", utxo_detail, account_id);
                        utxo_output = utxo_detail;
                        break;
                    }
                    Err(arg) => {
                        chain_attempt += 1;

                        if chain_attempt == 100 {
                            // flag_chain_update = false;
                            return Err(format!("Failed to get utxo details: {}", arg));
                        }
                        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                    }
                }
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
    let seed_signature_clone = seed_signature.clone();
    let zk_account_clone = zk_account.clone();
    let zk_account_clone2 = zk_account.clone();
    let (tx, output_state) = match tokio::task::spawn_blocking(move || {
        deploy_relayer_initial_state(
            account_id.clone(),
            zk_account_clone.scalar.clone(),
            zk_account_clone.balance.clone(),
            seed_signature.clone(),
            index - 1,
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
    println!("tx: {:?},\n\n\n output_state: {:?}", tx, output_state);
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
    println!("Transaction broadcast result: {}", broadcast_result);

    let tx_bin = bincode::serialize(&tx).unwrap();
    let tx_hex = hex::encode(&tx_bin);
    let out_state_bin = bincode::serialize(&output_state).unwrap();
    let out_state_hex = hex::encode(&out_state_bin);
    // println!("tx_hex {:?}\n", tx_hex);
    println!("out_state_hex {:?}\n", out_state_hex);
    println!("secret_key: {:?}", seed_signature_clone);
    println!("index: {}", index - 1);
    let relayer_data = serde_json::json!({
        "zkaccount": zk_account_clone2,
        "index": index - 1,
        "out_state_hex": out_state_hex,
        "seed_signature": seed_signature_clone
    });

    match std::fs::write(
        "relayer_deployer.json",
        serde_json::to_string_pretty(&relayer_data).unwrap(),
    ) {
        Ok(_) => println!("Successfully wrote relayer data to relayer_deployer.json"),
        Err(e) => return Err(format!("Failed to write relayer data to file: {}", e)),
    }
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
