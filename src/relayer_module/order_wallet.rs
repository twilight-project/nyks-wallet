use std::collections::HashMap;

use crate::{
    relayer_module::{
        self, fetch_utxo_details_with_retry, relayer_api::JsonRpcClient,
        relayer_order::create_trader_order,
    },
    wallet::Wallet,
    zkos_accounts::{
        encrypted_account::{DERIVATION_MESSAGE, KeyManager},
        zkaccount::ZkAccountDB,
    },
};
use log::{debug, error};
use relayer_module::utils::{TxResult, build_and_sign_msg_mint_burn_trading_btc, send_tx_to_chain};
use serde::{Deserialize, Serialize};
use twilight_client_sdk::{
    quisquislib::RistrettoSecretKey,
    relayer::query_trader_order_zkos,
    relayer_rpcclient::method::UtxoDetailResponse,
    relayer_types::{OrderType, PositionType, QueryTraderOrderZkos, TraderOrder},
    transfer::create_private_transfer_tx_single,
    zkvm::IOType,
};
pub type AccountIndex = u64;
pub type RequestId = String;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderWallet {
    pub wallet: Wallet,
    pub zk_accounts: ZkAccountDB,
    pub chain_id: String,
    seed: String,
    pub utxo_details: HashMap<AccountIndex, UtxoDetailResponse>,
    pub request_ids: HashMap<AccountIndex, RequestId>,
}

impl OrderWallet {
    pub fn new(wallet: Wallet, zk_accounts: ZkAccountDB, chain_id: &str) -> Result<Self, String> {
        let seed = wallet.get_zk_account_seed(chain_id, DERIVATION_MESSAGE)?;
        Ok(Self {
            wallet,
            zk_accounts,
            chain_id: chain_id.to_string(),
            seed,
            utxo_details: HashMap::new(),
            request_ids: HashMap::new(),
        })
    }
    pub fn get_zk_account_seed(&self, index: AccountIndex) -> RistrettoSecretKey {
        let key_manager = KeyManager::from_cosmos_signature(self.seed.as_bytes());
        key_manager.derive_child_key(index)
    }

    // Create a new zk_account and transfer sats from wallet to zk_account
    // Return the tx result (tx_hash, code) and the account index
    pub async fn funding_to_trading(&mut self, amount: u64) -> Result<(TxResult, u64), String> {
        let wallet_balance = self
            .wallet
            .update_balance()
            .await
            .map_err(|e| e.to_string())?;
        if wallet_balance.nyks > 0 && wallet_balance.sats >= amount {
            let account_index = self
                .zk_accounts
                .generate_new_account(amount, self.seed.clone())?;
            self.wallet
                .update_account_info()
                .await
                .map_err(|e| e.to_string())?;

            let wallet_on_chain_info = match self.wallet.account_info().await {
                Ok(account_info) => account_info,
                Err(e) => {
                    error!("Failed to get wallet account details on chain: {}", e);
                    return Err(e.to_string());
                }
            };
            let sequence = wallet_on_chain_info.account.sequence;
            let account_number = wallet_on_chain_info.account.account_number;
            let signed_tx = build_and_sign_msg_mint_burn_trading_btc(
                &self.wallet,
                &self.zk_accounts,
                account_index,
                sequence,
                account_number,
                amount,
            )?;
            let result = send_tx_to_chain(signed_tx.clone()).await?;
            if result.code != 0 {
                return Err(format!("Failed to send tx to chain: {}", result.tx_hash));
            } else {
                let account_address = self.zk_accounts.get_account_address(&account_index)?;
                let utxo_detail =
                    match fetch_utxo_details_with_retry(account_address, 20, 1000, IOType::Coin)
                        .await
                    {
                        Ok(utxo_detail) => utxo_detail,
                        Err(e) => {
                            error!("Failed to fetch utxo details after {} attempts: {}", 10, e);
                            return Err(e);
                        }
                    };
                self.utxo_details.insert(account_index, utxo_detail);
                self.zk_accounts.update_on_chain(&account_index, true)?;
            }
            Ok((result, account_index))
        } else {
            error!("Insufficient balance");
            Err(format!("Insufficient balance"))
        }
    }
    //  -> Result<(TxResult, u64), String>
    pub async fn trading_to_trading(
        &mut self,
        index: AccountIndex,
    ) -> Result<AccountIndex, String> {
        let sender_account = self.zk_accounts.get_account(&index)?;
        if sender_account.on_chain == false
            || sender_account.io_type != IOType::Coin
            || sender_account.balance == 0
        {
            error!("Account is not on chain or not a coin account");
            return Err(format!("Account is not on chain or not a coin account"));
        }
        let sender_account_address = sender_account.account.clone();
        let amount = sender_account.balance;
        let new_account_index = self
            .zk_accounts
            .generate_new_account(amount, self.seed.clone())?;
        let receiver_input_string = self
            .zk_accounts
            .get_account(&new_account_index)?
            .get_input_string()?;

        let utxo_detail =
            match fetch_utxo_details_with_retry(sender_account_address, 20, 1000, IOType::Coin)
                .await
            {
                Ok(utxo_detail) => utxo_detail,
                Err(e) => {
                    error!("Failed to fetch utxo details after {} attempts: {}", 10, e);
                    return Err(e);
                }
            };
        let input = utxo_detail.get_input()?;
        let tx_wallet = create_private_transfer_tx_single(
            self.get_zk_account_seed(index),
            input,
            receiver_input_string,
            amount,
            true,
            0,
            1u64,
        );
        let response = tokio::task::spawn_blocking(move || {
            twilight_client_sdk::chain::tx_commit_broadcast_transaction(
                tx_wallet.get_tx().ok_or("Failed to get tx")?,
            )
        })
        .await
        .map_err(|e| format!("Failed to send RPC request: {}", e))?;
        debug!("response: {:?}", response);
        let utxo_detail = match fetch_utxo_details_with_retry(
            self.zk_accounts.get_account_address(&new_account_index)?,
            20,
            1000,
            IOType::Coin,
        )
        .await
        {
            Ok(utxo_detail) => utxo_detail,
            Err(e) => {
                error!("Failed to fetch utxo details after {} attempts: {}", 10, e);
                return Err(e);
            }
        };

        self.utxo_details.insert(new_account_index, utxo_detail);
        self.utxo_details.remove(&index);
        self.zk_accounts.update_on_chain(&new_account_index, true)?;
        self.zk_accounts.update_on_chain(&index, false)?;

        Ok(new_account_index)
    }

    pub async fn open_trader_order(
        &mut self,
        index: AccountIndex,
        order_type: OrderType,
        order_side: PositionType,
        entry_price: u64,
        leverage: u64,
    ) -> Result<String, String> {
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let _utxo_detail =
            match fetch_utxo_details_with_retry(account_address.clone(), 60, 1000, IOType::Coin)
                .await
            {
                Ok(utxo_detail) => utxo_detail,
                Err(e) => {
                    error!("Failed to fetch utxo details after {} attempts: {}", 60, e);
                    return Err(e);
                }
            };
        let secret_key = self.get_zk_account_seed(index);
        let r_scalar = self.zk_accounts.get_account(&index)?.get_scalar()?;

        let initial_margin = self.zk_accounts.get_account(&index)?.balance;
        let position_value = initial_margin * leverage;
        let position_size = position_value * entry_price;

        let request_id = tokio::task::spawn_blocking(move || {
            let contract_path: &str = &std::env::var("RELAYER_PROGRAM_JSON_PATH")
                .unwrap_or_else(|_| "./relayerprogram.json".to_string());
            create_trader_order(
                secret_key,
                r_scalar,
                initial_margin,
                order_side,
                order_type,
                leverage,
                entry_price,
                position_value,
                position_size,
                contract_path,
                account_address,
            )
        })
        .await
        .map_err(|e| {
            error!("Failed to create order: {}", e);
            format!("Failed to create order: {}", e)
        })?;
        match request_id {
            Ok(request_id) => {
                self.request_ids.insert(index, request_id.clone());
                Ok(request_id)
            }
            Err(e) => {
                error!("Failed to create order: {}", e);
                Err(e)
            }
        }
    }

    // pub async fn close_trader_order(&mut self, index: AccountIndex) -> Result<String, String> {
    //     let account_address = self.zk_accounts.get_account_address(&index)?;
    //     let secret_key = self.get_zk_account_seed(index);
    //     let r_scalar = self.zk_accounts.get_account(&index)?.get_scalar()?;

    //     Ok(format!("close_trader_order"))
    // }

    pub async fn query_trader_order(&mut self, index: AccountIndex) -> Result<TraderOrder, String> {
        let account_address = self.zk_accounts.get_account_address(&index)?;
        let secret_key = self.get_zk_account_seed(index);
        // let request_id = self.request_ids.get(&index).unwrap();
        let query_order = query_trader_order_zkos(
            account_address.clone(),
            &secret_key,
            account_address.clone(),
            "PENDING".to_string(),
        );
        let query_order_zkos = QueryTraderOrderZkos::decode_from_hex_string(query_order)?;
        let relayer_connection = JsonRpcClient::new("https://relayer.twilight.rest/api").unwrap();
        let response = relayer_connection
            .trader_order_info(query_order_zkos)
            .await
            .map_err(|e| e.to_string())?;

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::get_test_tokens;
    use log::info;
    use std::sync::Once;
    use tokio::time::{Duration, sleep};
    use twilight_client_sdk::{
        relayer_rpcclient::txrequest::get_recent_price_from_relayer, relayer_types::PositionType,
    };
    static INIT: Once = Once::new();

    // This function initializes the logger for the tests.
    fn init_logger() {
        INIT.call_once(|| {
            // `is_test(true)` keeps the default filter at `trace`
            // and respects RUST_LOG if you set it.
            env_logger::builder().is_test(true).try_init().ok();
        });
    }
    async fn setup_wallet() -> Result<Wallet, String> {
        // info!("Creating new wallet with random BTC address");
        // let mut wallet = Wallet::create_new_with_random_btc_address()
        //     .await
        //     .map_err(|e| e.to_string())?;
        info!("importing wallet from json");
        let mut wallet = Wallet::import_from_json("test.json").map_err(|e| e.to_string())?;
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
    #[tokio::test]
    async fn test_create_order() -> Result<(), String> {
        dotenv::dotenv().ok();
        init_logger();
        let wallet = setup_wallet().await.unwrap();
        let zk_accounts = ZkAccountDB::new();

        let mut order_wallet = OrderWallet::new(wallet, zk_accounts, "nyks")?;
        let (tx_result, account_index) = order_wallet.funding_to_trading(6000).await?;
        if tx_result.code != 0 {
            return Err(format!("Failed to send tx to chain: {}", tx_result.tx_hash));
        }

        let btc_price = tokio::task::spawn_blocking(move || get_recent_price_from_relayer())
            .await
            .map_err(|e| format!("Failed to send RPC request: {}", e))?;
        let entry_price = btc_price?.result.price as u64;
        let result = order_wallet
            .open_trader_order(
                account_index,
                OrderType::MARKET,
                PositionType::LONG,
                entry_price,
                10,
            )
            .await;
        println!("result: {:?}", result);
        sleep(Duration::from_secs(15)).await;
        let response = order_wallet.query_trader_order(account_index).await?;
        println!("response: {:?}", response);

        Ok(())
    }

    #[tokio::test]
    async fn test_trading_to_trading() -> Result<(), String> {
        dotenv::dotenv().ok();
        init_logger();
        let wallet = setup_wallet().await.unwrap();
        let zk_accounts = ZkAccountDB::new();
        let mut order_wallet = OrderWallet::new(wallet, zk_accounts, "nyks")?;
        let (tx_result, sender_account_index) = order_wallet.funding_to_trading(6000).await?;
        if tx_result.code != 0 {
            return Err(format!("Failed to send tx to chain: {}", tx_result.tx_hash));
        }
        let receiver_account_index = order_wallet
            .trading_to_trading(sender_account_index)
            .await?;
        assert_ne!(sender_account_index, receiver_account_index);
        assert_eq!(
            order_wallet
                .zk_accounts
                .get_account(&sender_account_index)?
                .on_chain,
            false
        );
        assert_eq!(
            order_wallet
                .zk_accounts
                .get_account(&receiver_account_index)?
                .balance,
            6000
        );
        assert_eq!(
            order_wallet
                .zk_accounts
                .get_account(&receiver_account_index)?
                .on_chain,
            true
        );
        Ok(())
    }
}
