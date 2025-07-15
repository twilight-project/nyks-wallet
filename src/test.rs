#[cfg(test)]
mod tests {
    use super::*;
    use crate::faucet::*;
    use crate::wallet::*;
    use serial_test::serial;
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    #[serial]
    async fn test_create_wallet() {
        dotenv::dotenv().ok();
        match create_and_export_randmon_wallet_account("test1").await {
            Ok(wallet) => println!("wallet: {:?}", wallet),
            Err(e) => println!(
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
        let mut wallet = Wallet::import_from_json("test.json")?;
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
        let mut wallet = Wallet::import_from_json("test.json")?;
        let account_details = fetch_account_details(&wallet.twilightaddress).await?;
        println!("Account details: {:?}", account_details);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_check_balance() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
        let mut wallet = Wallet::import_from_json("test.json")?;
        let balance = check_balance(&wallet.twilightaddress).await?;
        println!("Balance: {:?}", balance);
        Ok(())
    }
}
