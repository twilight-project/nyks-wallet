#[allow(unused_imports, unused_variables, dead_code)]
#[cfg(test)]
mod tests {
    use crate::nyks_fn::MsgMintBurnTradingBtc;
    use crate::nyks_rpc::rpcclient::method::{Method, MethodTypeURL};
    use crate::nyks_rpc::rpcclient::txrequest::{RpcBody, RpcRequest, TxParams};
    use crate::nyks_rpc::rpcclient::txresult::from_rpc_response;
    use crate::seed_signer::{build_sign_doc, sign_adr036, signature_bundle};
    use crate::wallet::*;
    use crate::zkos_accounts::ZkAccountDB;
    use crate::zkos_accounts::encrypted_account::DERIVATION_MESSAGE;
    use cosmrs::crypto::secp256k1::SigningKey;
    use log::warn;
    use secrecy::SecretString;
    use serial_test::serial;

    use std::sync::Once;
    use tokio::sync::OnceCell;

    static INIT: Once = Once::new();
    static INIT_ASYNC: OnceCell<()> = OnceCell::const_new();

    // This function initializes the logger for the tests.
    fn init_logger() {
        INIT.call_once(|| {
            // `is_test(true)` keeps the default filter at `trace`
            // and respects RUST_LOG if you set it.
            env_logger::builder().is_test(true).try_init().ok();
        });
    }
    async fn global_setup() {
        INIT_ASYNC
            .get_or_init(|| async {
                match create_and_export_randmon_wallet_account("test").await {
                    Ok(_) => println!("wallet created successfully"),
                    Err(_) => warn!(
                        "error: {:?}",
                        "wallet creation failed, wallet already exists"
                    ),
                }
            })
            .await;
    }

    // This test creates a new wallet and exports it to a JSON file.
    // RUST_LOG=debug cargo test --package nyks-wallet --lib --all-features -- test::tests::test_create_wallet --exact --show-output
    #[tokio::test]
    #[serial]
    async fn test_create_wallet() {
        init_logger();
        dotenv::dotenv().ok();
        match create_and_export_randmon_wallet_account("test1").await {
            Ok(_) => println!("wallet created successfully"),
            Err(_) => println!(
                "error: {:?}",
                "wallet creation failed, wallet already exists"
            ),
        }
    }

    // This test imports a wallet from a JSON file and prints the wallet details.
    // RUST_LOG=debug cargo test --package nyks-wallet --lib --all-features -- test::tests::test_wallet_import --exact --show-output
    #[tokio::test]
    #[serial]
    async fn test_wallet_import() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
        init_logger();
        global_setup().await;

        let wallet = Wallet::import_from_json("test.json")?;
        println!("wallet: {:?}", wallet);
        Ok(())
    }

    // This test creates a new wallet, gets test tokens, updates the balance and account info,
    // and exports the wallet to a JSON file.
    // RUST_LOG=debug cargo test --package nyks-wallet --lib --all-features -- test::tests::test_wallet_complete_flow --exact --show-output
    #[tokio::test]
    #[serial]
    async fn test_wallet_complete_flow() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
        init_logger();
        global_setup().await;

        let mut wallet = Wallet::import_from_json("test.json")?;
        println!("wallet: {:?}", wallet);
        get_test_tokens(&mut wallet).await?;
        wallet.update_balance().await?;
        wallet.update_account_info().await?;
        wallet.export_to_json("test.json")?;

        println!("wallet: {:?}", wallet);
        Ok(())
    }

    // This test creates a new wallet, gets test tokens and prints the wallet details.
    // RUST_LOG=debug cargo test --package nyks-wallet --lib --all-features -- test::tests::test_get_tokens --exact --show-output
    #[tokio::test]
    #[serial]
    async fn test_get_tokens() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
        init_logger();
        global_setup().await;

        let mut wallet = Wallet::import_from_json("test.json")?;
        get_test_tokens(&mut wallet).await?;
        println!("Updated wallet: {:?}", wallet);

        Ok(())
    }

    // This test creates a new wallet from a mnemonic and prints the wallet details.
    // RUST_LOG=debug cargo test --package nyks-wallet --lib --all-features -- test::tests::test_wallet_from_mnemonic --exact --show-output
    #[tokio::test]
    #[serial]
    async fn test_wallet_from_mnemonic() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
        init_logger();
        let wallet = Wallet::from_mnemonic(
            "test test test test test test test test test test test junk",
            None,
        )?;
        println!("wallet: {:?}", wallet);
        Ok(())
    }

    // This test creates a new wallet from a private key and prints the wallet details.
    // RUST_LOG=debug cargo test --package nyks-wallet --lib --all-features -- test::tests::test_wallet_from_private_key --exact --show-output
    #[tokio::test]
    #[serial]
    async fn test_wallet_from_private_key() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
        init_logger();
        let random_key = SigningKey::random();
        let pubkey_bytes = random_key.public_key().to_bytes();
        let btc_address = format!(
            "bc1q{}",
            hex::encode(&pubkey_bytes[..19])
                .chars()
                .take(38)
                .collect::<String>()
        );
        let wallet = Wallet::from_private_key(
            "e64e7928d4f6c06f01fefd31f760c51f59a16426e792761cd00529b76501c8a0",
            &btc_address,
            None,
        )?;
        println!("wallet: {:?}", wallet);
        Ok(())
    }

    // This test fetches the account details and prints them.
    // RUST_LOG=debug cargo test --package nyks-wallet --lib --all-features -- test::tests::test_fetch_account_details --exact --show-output
    #[tokio::test]
    #[serial]
    async fn test_fetch_account_details() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
        init_logger();
        global_setup().await;
        let wallet = Wallet::import_from_json("test.json")?;
        let account_details =
            fetch_account_details(&wallet.twilightaddress, &wallet.chain_config.lcd_endpoint)
                .await?;
        println!("Account details: {:?}", account_details);
        Ok(())
    }

    // This test checks the balance of the wallet and prints it.
    // RUST_LOG=debug cargo test --package nyks-wallet --lib --all-features -- test::tests::test_check_balance --exact --show-output
    #[tokio::test]
    #[serial]
    async fn test_check_balance() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
        init_logger();
        global_setup().await;
        let wallet = Wallet::import_from_json("test.json")?;
        let balance =
            check_balance(&wallet.twilightaddress, &wallet.chain_config.lcd_endpoint).await?;
        println!("Balance: {:?}", balance);
        Ok(())
    }

    // This test broadcasts a mint/burn transaction and prints the response.
    // RUST_LOG=debug cargo test --package nyks-wallet --lib --all-features -- test::tests::test_broadcast_tx_sync_mint_burn --exact --show-output
    #[tokio::test]
    #[serial]
    async fn test_broadcast_tx_sync_mint_burn() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
        init_logger();
        global_setup().await;
        let wallet = Wallet::import_from_json("test.json")?;
        let sk = wallet.signing_key()?;
        let pk = wallet.public_key()?;
        let account_details = wallet.account_info().await?;
        let sequence = account_details.account.sequence;
        let account_number = account_details.account.account_number;
        // Create test message
        let msg = MsgMintBurnTradingBtc {
            mint_or_burn: true,
            btc_value: 1000,
            qq_account: "test_creator".to_string(),
            encrypt_scalar: "test_scalar".to_string(),
            twilight_address: wallet.twilightaddress.clone(),
        };

        // Create method type and get Any message
        let method_type = MethodTypeURL::MsgMintBurnTradingBtc;
        let any_msg = method_type.type_url(msg);

        // Sign the message
        let signed_tx = method_type
            .sign_msg::<MsgMintBurnTradingBtc>(any_msg, pk, sequence, account_number, sk)
            .unwrap();

        // Create RPC request
        let (tx_send, data): (RpcBody<TxParams>, String) = RpcRequest::new_with_data(
            TxParams::new(signed_tx.clone()),
            Method::broadcast_tx_sync,
            signed_tx,
        );

        // Send RPC request
        let response = match tokio::task::spawn_blocking(move || tx_send.send(wallet.chain_config.rpc_endpoint.clone()))
            .await // wait for the blocking task to finish
            {
                Ok(response) => response,
                Err(e) => {
                    println!("Failed to send RPC request: {}", e);
                    return Err(anyhow::anyhow!("Failed to send RPC request: {}", e));
                }
            };

        println!("response: {:?}", response);

        let result = match response {
            Ok(rpc_response) => from_rpc_response(rpc_response),
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to get tx result: {}", e));
            }
        };
        match result {
            Ok(result) => println!("result: {:?}", result),
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to get tx result: {}", e));
            }
        }
        Ok(())
    }

    // This test creates a seed and prints it.
    // RUST_LOG=debug cargo test --package nyks-wallet --lib --all-features -- test::tests::test_seed_signer --exact --show-output
    #[tokio::test]
    async fn test_seed_signer() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
        init_logger();
        global_setup().await;
        let wallet = Wallet::import_from_json("test.json")?;
        let private_key = wallet.private_key.clone();
        let twilight_address = wallet.twilightaddress.clone();
        let sign_mgs = "hello";
        let chain_id = "nyks";
        let seed = generate_seed(&private_key, &twilight_address, sign_mgs, chain_id);
        println!("{:?}", seed);
        Ok(())
    }

    // This test creates a ZkAccount from a seed and prints it.
    // RUST_LOG=debug cargo test --package nyks-wallet --lib --all-features -- test::tests::test_zkaccount_from_seed --exact --show-output
    #[tokio::test]
    #[serial]
    async fn test_zkaccount_from_seed() {
        dotenv::dotenv().ok();
        init_logger();
        global_setup().await;
        // Create a mock 64-byte signature/seed
        let wallet = Wallet::import_from_json("test.json").unwrap();
        let private_key = wallet.private_key.clone();
        let twilight_address = wallet.twilightaddress.clone();
        let chain_id = "nyks";

        let seed = match generate_seed(
            &private_key,
            &twilight_address,
            DERIVATION_MESSAGE,
            chain_id,
        ) {
            Ok(seed) => seed,
            Err(e) => panic!("Failed to generate seed: {}", e),
        };
        let seed_str = seed.get_signature();

        let balance = 40000;

        // Create ZkAccount from seed
        println!("{}", seed_str);
        let mut db = match ZkAccountDB::import_from_json("ZkAccounts.json") {
            Ok(db) => db,
            Err(e) => {
                println!("Failed to import from json: {}", e);
                ZkAccountDB::new()
            }
        };
        let index = db
            .generate_new_account(balance, &SecretString::new(seed_str))
            .unwrap();
        println!("{}", index);
        println!(
            "{:?}",
            db.get_all_accounts_as_json().map_err(|e| e.to_string())
        );
        match db.export_to_json("ZkAccounts.json") {
            Ok(_) => println!("Exported to ZkAccounts.json"),
            Err(e) => println!("Failed to export to json: {}", e),
        }
    }

    // This test creates a wallet from a keyring file and prints the wallet details.
    // RUST_LOG=debug cargo test --package nyks-wallet --lib --all-features -- test::tests::test_wallet_from_keyring_file --exact --show-output
    #[tokio::test]
    #[serial]
    async fn test_wallet_from_keyring_file() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
        init_logger();
        // global_setup().await;
        let wallet = Wallet::from_mnemonic_file("validator-self.mnemonic")?;
        let balance =
            check_balance(&wallet.twilightaddress, &wallet.chain_config.lcd_endpoint).await?;
        println!("balance: {:?}", balance);
        println!("wallet: {:?}", wallet);
        Ok(())
    }
    #[tokio::test]
    #[serial]
    async fn test_wallet_from_mnemonic_new() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
        init_logger();
        let wallet = Wallet::new(None)?;
        println!("wallet: {:?}", wallet);
        Ok(())
    }
}
