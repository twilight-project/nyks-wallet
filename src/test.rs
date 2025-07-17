#[allow(unused_imports, unused_variables, dead_code)]
#[cfg(test)]
mod tests {
    use crate::nyks_fn::MsgMintBurnTradingBtc;
    use crate::nyks_rpc::rpcclient::method::{Method, MethodTypeURL};
    use crate::nyks_rpc::rpcclient::txrequest::{RpcBody, RpcRequest, TxParams};
    use crate::seed_signer::{build_sign_doc, sign_adr036, signature_bundle};
    use crate::wallet::*;
    use crate::zkos_accounts::ZkAccountDB;
    use crate::zkos_accounts::encrypted_account::DERIVATION_MESSAGE;
    use cosmrs::crypto::secp256k1::SigningKey;
    use serial_test::serial;
    #[tokio::test]
    #[serial]
    async fn test_create_wallet() {
        dotenv::dotenv().ok();
        match create_and_export_randmon_wallet_account("test1").await {
            Ok(wallet) => println!("wallet: {:?}", wallet),
            Err(_) => println!(
                "error: {:?}",
                "wallet creation failed, wallet already exists"
            ),
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_wallet_import() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
        match create_and_export_randmon_wallet_account("test").await {
            Ok(wallet) => println!("wallet: {:?}", wallet),
            Err(e) => println!(
                "error: {:?}",
                "wallet creation failed, wallet already exists"
            ),
        }
        let wallet = Wallet::import_from_json("test.json")?;
        println!("wallet: {:?}", wallet);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_get_tokens() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
        match create_and_export_randmon_wallet_account("test").await {
            Ok(wallet) => println!("wallet: {:?}", wallet),
            Err(e) => println!(
                "error: {:?}",
                "wallet creation failed, wallet already exists"
            ),
        }
        let mut wallet = Wallet::import_from_json("test.json")?;
        get_test_tokens(&mut wallet).await?;
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_wallet_from_mnemonic() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
        let wallet =
            Wallet::from_mnemonic("test test test test test test test test test test test junk")?;
        println!("wallet: {:?}", wallet);
        Ok(())
    }
    #[tokio::test]
    #[serial]
    async fn test_wallet_from_private_key() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
        let wallet = Wallet::from_private_key(
            "e64e7928d4f6c06f01fefd31f760c51f59a16426e792761cd00529b76501c8a0",
        )?;
        println!("wallet: {:?}", wallet);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_fetch_account_details() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
        let wallet = Wallet::import_from_json("test.json")?;
        let account_details = fetch_account_details(&wallet.twilightaddress).await?;
        println!("Account details: {:?}", account_details);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_check_balance() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
        let wallet = Wallet::import_from_json("test.json")?;
        let balance = check_balance(&wallet.twilightaddress).await?;
        println!("Balance: {:?}", balance);
        Ok(())
    }

    #[tokio::test]
    async fn test_broadcast_tx_sync_mint_burn() -> anyhow::Result<()> {
        let wallet = Wallet::import_from_json("test.json")?;
        let sk = wallet.signing_key()?;
        let pk = wallet.public_key()?;
        let account_details = wallet.account_info().await?;
        let sequence = account_details.account.sequence.parse::<u64>()?;
        let account_number = account_details
            .account
            .account_number
            .parse::<u64>()
            .unwrap();
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
        // let response = tx_send.send("https://rpc.twilight.rest".to_string(), data);
        let url = "https://rpc.twilight.rest".to_string();
        let response = tokio::task::spawn_blocking(move || tx_send.send(url))
            .await // wait for the blocking task to finish
            .expect("blocking task panicked")?;

        println!("response: {:?}", response);
        Ok(())
    }

    #[tokio::test]
    async fn test_seed_signer() -> anyhow::Result<()> {
        let wallet = Wallet::import_from_json("test.json")?;
        let private_key = wallet.private_key.clone();
        let twilight_address = wallet.twilightaddress.clone();
        let sign_mgs = "hello";
        let chain_id = "nyks";
        let seed = generate_seed(&private_key, &twilight_address, sign_mgs, chain_id);
        println!("{:?}", seed);
        Ok(())
    }

    #[test]
    fn test_zkaccount_from_seed() {
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
        let index = db.generate_new_account(balance, seed_str).unwrap();
        println!("{}", index);
        println!("{}", db.get_all_accounts_as_json());
        match db.export_to_json("ZkAccounts.json") {
            Ok(_) => println!("Exported to ZkAccounts.json"),
            Err(e) => println!("Failed to export to json: {}", e),
        }
    }
}
