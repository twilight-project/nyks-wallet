//! Relayer JSON-RPC API client for interacting with the Twilight relayer service.
//!
//! This module provides a comprehensive HTTP client for all relayer endpoints including:
//! - Market data (prices, candles, order books)
//! - Trading operations (submit, settle, cancel orders)
//! - Historical data (prices, funding rates, fees)
//! - Order management and querying
//! - Pool and liquidity information
//!
//! The client handles automatic serialization/deserialization of ZkOS transaction types
//! and provides a clean async interface for all relayer operations.

use super::relayer_types::{
    BtcUsdPrice, Candle, Candles, FeeHistory, FundingRate, HistoricalFeeArgs,
    HistoricalFundingArgs, HistoricalPriceArgs, LendOrder, LendPoolInfo, OrderBook, PositionSize,
    RecentOrders, RequestResponse, TraderOrder, TransactionHashArgs, TxHash,
};
use chrono::{DateTime, Utc};
use jsonrpsee::core::client::ClientT;
use jsonrpsee::core::traits::ToRpcParams;
use serde_json::value::RawValue;

use jsonrpsee::core::client::Error as RpcError;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee::rpc_params;
use serde::{Deserialize, Serialize};
use twilight_client_sdk::relayer_types::{
    CancelTraderOrderZkos, CreateLendOrderZkos, CreateTraderOrderClientZkos, ExecuteLendOrderZkos,
    ExecuteTraderOrderZkos, QueryLendOrderZkos, QueryTraderOrderZkos,
};

/// Wrapper for hex-encoded binary data sent to relayer endpoints.
#[derive(Debug, Serialize, Deserialize)]
pub struct HexEncodedData {
    pub data: String,
}

/// JSON-RPC HTTP client for the Twilight relayer API.
///
/// Provides async methods for all relayer endpoints including market data,
/// trading operations, historical queries, and pool information.
///
/// # Example
///
/// ```no_run
/// use nyks_wallet::relayer_module::relayer_api::RelayerJsonRpcClient;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = RelayerJsonRpcClient::new("http://0.0.0.0:8088/api")?;
///     
///     // Get current BTC price
///     let price = client.btc_usd_price().await?;
///     println!("BTC/USD: ${}", price.price);
///     
///     // Get order book
///     let order_book = client.open_limit_orders().await?;
///     println!("Order book: {:?}", order_book);
///     
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct RelayerJsonRpcClient {
    client: HttpClient,
}

impl RelayerJsonRpcClient {
    /// Create a new relayer client with the specified endpoint URL.
    ///
    /// # Arguments
    /// * `url` - The base URL of the relayer API (e.g., "http://0.0.0.0:8088/api")
    pub fn new(url: &str) -> Result<Self, RpcError> {
        let client = HttpClientBuilder::default()
            .request_timeout(std::time::Duration::from_secs(30))
            .build(url)?;
        Ok(Self { client })
    }

    // -------------------------
    // Market Data APIs
    // -------------------------

    /// Get the current BTC/USD price from the relayer.
    pub async fn btc_usd_price(&self) -> Result<BtcUsdPrice, RpcError> {
        self.client.request("btc_usd_price", rpc_params![]).await
    }

    /// Get historical BTC/USD price data for a given time range.
    pub async fn historical_price(
        &self,
        params: HistoricalPriceArgs,
    ) -> Result<Vec<BtcUsdPrice>, RpcError> {
        self.client
            .request("historical_price", AsRpcParams(params))
            .await
    }

    /// Get candlestick/OHLCV data for price charting.
    pub async fn candle_data(&self, params: Candles) -> Result<Vec<Candle>, RpcError> {
        self.client
            .request("candle_data", AsRpcParams(params))
            .await
    }

    pub async fn historical_funding_rate(
        &self,
        params: HistoricalFundingArgs,
    ) -> Result<Vec<FundingRate>, RpcError> {
        self.client
            .request("historical_funding_rate", AsRpcParams(params))
            .await
    }

    pub async fn get_funding_rate(&self) -> Result<FundingRate, RpcError> {
        self.client.request("get_funding_rate", rpc_params![]).await
    }

    pub async fn historical_fee_rate(
        &self,
        params: HistoricalFeeArgs,
    ) -> Result<Vec<FeeHistory>, RpcError> {
        self.client
            .request("historical_fee_rate", AsRpcParams(params))
            .await
    }

    pub async fn get_fee_rate(&self) -> Result<FeeHistory, RpcError> {
        self.client.request("get_fee_rate", rpc_params![]).await
    }

    pub async fn open_limit_orders(&self) -> Result<OrderBook, RpcError> {
        self.client
            .request("open_limit_orders", rpc_params![])
            .await
    }

    pub async fn recent_trade_orders(&self) -> Result<RecentOrders, RpcError> {
        self.client
            .request("recent_trade_orders", rpc_params![])
            .await
    }

    pub async fn position_size(&self) -> Result<PositionSize, RpcError> {
        self.client.request("position_size", rpc_params![]).await
    }

    pub async fn transaction_hashes(
        &self,
        params: TransactionHashArgs,
    ) -> Result<Vec<TxHash>, RpcError> {
        self.client
            .request("transaction_hashes", AsRpcParams(params))
            .await
    }

    /// Get the current server time in UTC.
    pub async fn server_time(&self) -> Result<DateTime<Utc>, RpcError> {
        self.client.request("server_time", rpc_params![]).await
    }

    // -------------------------
    // Order Query APIs
    // -------------------------

    /// Query trader order information using ZkOS parameters.
    pub async fn trader_order_info(
        &self,
        tx: QueryTraderOrderZkos,
    ) -> Result<TraderOrder, RpcError> {
        let data = bincode::serialize(&tx).unwrap();
        let params = HexEncodedData {
            data: hex::encode(data),
        };
        self.client
            .request("trader_order_info", AsRpcParams(params))
            .await
    }

    pub async fn lend_order_info(&self, tx: QueryLendOrderZkos) -> Result<LendOrder, RpcError> {
        let data = bincode::serialize(&tx).unwrap();
        let params = HexEncodedData {
            data: hex::encode(data),
        };
        self.client
            .request("lend_order_info", AsRpcParams(params))
            .await
    }

    pub async fn historical_trader_order_info(
        &self,
        tx: QueryTraderOrderZkos,
    ) -> Result<Vec<TraderOrder>, RpcError> {
        let data = bincode::serialize(&tx).unwrap();
        let data = hex::encode(data);
        let params = HexEncodedData { data };
        self.client
            .request("historical_trader_order_info", AsRpcParams(params))
            .await
    }

    pub async fn historical_lend_order_info(
        &self,
        tx: QueryLendOrderZkos,
    ) -> Result<Vec<LendOrder>, RpcError> {
        let data = bincode::serialize(&tx).unwrap();
        let params = HexEncodedData {
            data: hex::encode(data),
        };
        self.client
            .request("historical_lend_order_info", AsRpcParams(params))
            .await
    }

    // -------------------------
    // Order Submission APIs
    // -------------------------

    /// Submit a trader order to the relayer for execution.
    pub async fn submit_trade_order(
        &self,
        tx: CreateTraderOrderClientZkos,
    ) -> Result<RequestResponse, RpcError> {
        let params = HexEncodedData {
            data: tx.encode_as_hex_string().map_err(|e| RpcError::Custom(e))?,
        };
        self.client
            .request("submit_trade_order", AsRpcParams(params))
            .await
    }

    pub async fn submit_lend_order(
        &self,
        tx: CreateLendOrderZkos,
    ) -> Result<RequestResponse, RpcError> {
        let params = HexEncodedData {
            data: tx.encode_as_hex_string(),
        };
        self.client
            .request("submit_lend_order", AsRpcParams(params))
            .await
    }

    pub async fn settle_trade_order(
        &self,
        tx: ExecuteTraderOrderZkos,
    ) -> Result<RequestResponse, RpcError> {
        let params = HexEncodedData {
            data: tx.encode_as_hex_string(),
        };
        self.client
            .request("settle_trade_order", AsRpcParams(params))
            .await
    }

    pub async fn settle_lend_order(
        &self,
        tx: ExecuteLendOrderZkos,
    ) -> Result<RequestResponse, RpcError> {
        let params = HexEncodedData {
            data: tx.encode_as_hex_string(),
        };
        self.client
            .request("settle_lend_order", AsRpcParams(params))
            .await
    }

    pub async fn cancel_trader_order(
        &self,
        tx: CancelTraderOrderZkos,
    ) -> Result<RequestResponse, RpcError> {
        let params = HexEncodedData {
            data: tx.encode_as_hex_string(),
        };
        self.client
            .request("cancel_trader_order", AsRpcParams(params))
            .await
    }

    pub async fn pool_share_value(&self) -> Result<f64, RpcError> {
        self.client.request("pool_share_value", rpc_params![]).await
    }

    pub async fn lend_pool_info(&self) -> Result<LendPoolInfo, RpcError> {
        self.client.request("lend_pool_info", rpc_params![]).await
    }
}

pub struct AsRpcParams<T>(pub T);

impl<T: Serialize> ToRpcParams for AsRpcParams<T> {
    fn to_rpc_params(self) -> Result<Option<Box<RawValue>>, serde_json::Error> {
        // 1. Serialize the inner value to a JSON string…
        let s = serde_json::to_string(&self.0).map_err(serde_json::Error::from)?;

        // 2. …wrap it in `RawValue` so jsonrpsee can send it verbatim.
        RawValue::from_string(s)
            .map(Some)
            .map_err(serde_json::Error::from)
    }
}

#[cfg(test)]
mod tests {
    use crate::relayer_module::relayer_types::OrderStatus;

    use super::*;
    use crate::relayer_module::relayer_types::{Interval, TransactionHashArgs};
    #[tokio::test]
    async fn test_transaction_hashes_by_request_id() {
        dotenv::dotenv().ok();
        let relayer_url = std::env::var("RELAYER_API_RPC_SERVER_URL")
            .unwrap_or("http://0.0.0.0:8088/api".to_string());
        let relayer = RelayerJsonRpcClient::new(&relayer_url).unwrap();

        let request_id = TransactionHashArgs::RequestId {
            id: "REQID9804F25BCD722E00D408E3034B6E4EC144992DA7871A1F4DCEC86F0CBBCE81D1".to_string(),
            status: Some(OrderStatus::FILLED),
        };

        match relayer.transaction_hashes(request_id).await {
            Ok(response) => println!("Transaction hashes response: {:?}", response),
            Err(e) => {
                println!("Error getting transaction hashes: {:?}", e);
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn test_position_size() {
        dotenv::dotenv().ok();
        let relayer_url = std::env::var("RELAYER_API_RPC_SERVER_URL")
            .unwrap_or("http://0.0.0.0:8088/api".to_string());
        let relayer = RelayerJsonRpcClient::new(&relayer_url).unwrap();

        match relayer.position_size().await {
            Ok(response) => println!("Position size response: {:?}", response),
            Err(e) => {
                println!("Error getting position size: {:?}", e);
                assert!(false);
            }
        }
    }
    #[tokio::test]
    async fn test_pool_share_value() {
        dotenv::dotenv().ok();
        let relayer_url = std::env::var("RELAYER_API_RPC_SERVER_URL")
            .unwrap_or("http://0.0.0.0:8088/api".to_string());
        let relayer = RelayerJsonRpcClient::new(&relayer_url).unwrap();

        match relayer.pool_share_value().await {
            Ok(value) => println!("Pool share value: {}", value),
            Err(e) => {
                println!("Error getting pool share value: {:?}", e);
                assert!(false);
            }
        }
    }
    #[tokio::test]
    async fn test_lend_pool_info() {
        dotenv::dotenv().ok();
        let relayer_url = std::env::var("RELAYER_API_RPC_SERVER_URL")
            .unwrap_or("http://0.0.0.0:8088/api".to_string());
        let relayer = RelayerJsonRpcClient::new(&relayer_url).unwrap();

        match relayer.lend_pool_info().await {
            Ok(info) => println!("Lend pool info: {:?}", info),
            Err(e) => {
                println!("Error getting lend pool info: {:?}", e);
                assert!(false);
            }
        }
    }
    #[tokio::test]
    async fn test_historical_funding() {
        dotenv::dotenv().ok();
        let relayer_url = std::env::var("RELAYER_API_RPC_SERVER_URL")
            .unwrap_or("http://0.0.0.0:8088/api".to_string());
        let relayer = RelayerJsonRpcClient::new(&relayer_url).unwrap();
        let args = HistoricalFundingArgs {
            from: chrono::Utc::now() - chrono::Duration::hours(24),
            to: chrono::Utc::now(),
            limit: 100,
            offset: 0,
        };

        match relayer.historical_funding_rate(args).await {
            Ok(funding) => println!("Historical funding: {:?}", funding),
            Err(e) => {
                println!("Error getting historical funding: {:?}", e);
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn test_historical_fees() {
        dotenv::dotenv().ok();
        let relayer_url = std::env::var("RELAYER_API_RPC_SERVER_URL")
            .unwrap_or("http://0.0.0.0:8088/api".to_string());
        let relayer = RelayerJsonRpcClient::new(&relayer_url).unwrap();
        let args = HistoricalFeeArgs {
            from: chrono::Utc::now() - chrono::Duration::days(365),
            to: chrono::Utc::now(),
            limit: 100,
            offset: 0,
        };

        match relayer.historical_fee_rate(args).await {
            Ok(fees) => println!("Historical fees: {:?}", fees),
            Err(e) => {
                println!("Error getting historical fees: {:?}", e);
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn test_historical_prices() {
        dotenv::dotenv().ok();
        let relayer_url = std::env::var("RELAYER_API_RPC_SERVER_URL")
            .unwrap_or("http://0.0.0.0:8088/api".to_string());
        let relayer = RelayerJsonRpcClient::new(&relayer_url).unwrap();
        let args = HistoricalPriceArgs {
            from: chrono::Utc::now() - chrono::Duration::hours(24),
            to: chrono::Utc::now(),
            limit: 100,
            offset: 0,
        };
        println!("args: {:?}", serde_json::to_string(&args).unwrap());

        match relayer.historical_price(args).await {
            Ok(prices) => println!("Historical prices: {:?}", prices),
            Err(e) => {
                println!("Error getting historical prices: {:?}", e);
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn test_btc_usd_price() {
        dotenv::dotenv().ok();
        let relayer_url = std::env::var("RELAYER_API_RPC_SERVER_URL")
            .unwrap_or("http://0.0.0.0:8088/api".to_string());
        let relayer = RelayerJsonRpcClient::new(&relayer_url).unwrap();
        match relayer.btc_usd_price().await {
            Ok(price) => println!("BTC/USD price: {:?}", price),
            Err(e) => {
                println!("Error getting BTC/USD price: {:?}", e);
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn test_open_limit_orders() {
        dotenv::dotenv().ok();
        let relayer_url = std::env::var("RELAYER_API_RPC_SERVER_URL")
            .unwrap_or("http://0.0.0.0:8088/api".to_string());
        let relayer = RelayerJsonRpcClient::new(&relayer_url).unwrap();
        match relayer.open_limit_orders().await {
            Ok(order_book) => println!("Open limit orders: {:?}", order_book),
            Err(e) => {
                println!("Error getting open limit orders: {:?}", e);
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn test_recent_trade_orders() {
        dotenv::dotenv().ok();
        let relayer_url = std::env::var("RELAYER_API_RPC_SERVER_URL")
            .unwrap_or("http://0.0.0.0:8088/api".to_string());
        let relayer = RelayerJsonRpcClient::new(&relayer_url).unwrap();
        match relayer.recent_trade_orders().await {
            Ok(orders) => println!("Recent trade orders: {:?}", orders),
            Err(e) => {
                println!("Error getting recent trade orders: {:?}", e);
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn test_candle_data() {
        dotenv::dotenv().ok();
        let relayer_url = std::env::var("RELAYER_API_RPC_SERVER_URL")
            .unwrap_or("http://0.0.0.0:8088/api".to_string());
        let relayer = RelayerJsonRpcClient::new(&relayer_url).unwrap();
        let args = Candles {
            interval: Interval::ONE_MINUTE,
            since: chrono::Utc::now() - chrono::Duration::hours(1),
            limit: 100,
            offset: 0,
        };
        match relayer.candle_data(args).await {
            Ok(candles) => println!("Candle data: {:?}", candles),
            Err(e) => {
                println!("Error getting candle data: {:?}", e);
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn test_get_funding_rate() {
        dotenv::dotenv().ok();
        let relayer_url = std::env::var("RELAYER_API_RPC_SERVER_URL")
            .unwrap_or("http://0.0.0.0:8088/api".to_string());
        let relayer = RelayerJsonRpcClient::new(&relayer_url).unwrap();
        match relayer.get_funding_rate().await {
            Ok(rate) => println!("Funding rate: {:?}", rate),
            Err(e) => {
                println!("Error getting funding rate: {:?}", e);
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn test_get_fee_rate() {
        dotenv::dotenv().ok();
        let relayer_url = std::env::var("RELAYER_API_RPC_SERVER_URL")
            .unwrap_or("http://0.0.0.0:8088/api".to_string());
        let relayer = RelayerJsonRpcClient::new(&relayer_url).unwrap();
        match relayer.get_fee_rate().await {
            Ok(fee) => println!("Fee rate: {:?}", fee),
            Err(e) => {
                println!("Error getting fee rate: {:?}", e);
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn test_historical_trader_order_info() {
        dotenv::dotenv().ok();
        let relayer_url = std::env::var("RELAYER_API_RPC_SERVER_URL")
            .unwrap_or("http://0.0.0.0:8088/api".to_string());
        let relayer = RelayerJsonRpcClient::new(&relayer_url).unwrap();
        let query_str = "8a00000000000000306339363431313165653936636436663332656333363734353065656130636464663632343439393766623339366265323464623335363063653831616133333435313633393733333265316135386131646162623135346134346539663631616163386436663264323366386338386464623435306332623065626663373831633239366638343761050000008a000000000000003063393634313131656539366364366633326563333637343530656561306364646636323434393937666233393662653234646233353630636538316161333334353136333937333332653161353861316461626231353461343465396636316161633864366632643233663863383864646234353063326230656266633738316332393666383437614000000000000000bcdd784e30b11b3d80b039e951e2de9ce68bc9e12254f588872a4cb05621916267f41835ff570ed81f53e4553f055b6b3e691d75910b4e5ae0d9d5556f5b0e08".to_string();
        let query = QueryTraderOrderZkos::decode_from_hex_string(query_str).unwrap();
        match relayer.historical_trader_order_info(query).await {
            Ok(orders) => println!("Historical trader orders: {:?}", orders),
            Err(e) => {
                println!("Error getting historical trader orders: {:?}", e);
                assert!(false);
            }
        }
    }
    #[tokio::test]
    #[ignore]
    async fn test_trader_order_info() {
        dotenv::dotenv().ok();
        let relayer_url = std::env::var("RELAYER_API_RPC_SERVER_URL")
            .unwrap_or("http://0.0.0.0:8088/api".to_string());
        let relayer = RelayerJsonRpcClient::new(&relayer_url).unwrap();
        let query_str = "8a00000000000000306339363431313165653936636436663332656333363734353065656130636464663632343439393766623339366265323464623335363063653831616133333435313633393733333265316135386131646162623135346134346539663631616163386436663264323366386338386464623435306332623065626663373831633239366638343761050000008a000000000000003063393634313131656539366364366633326563333637343530656561306364646636323434393937666233393662653234646233353630636538316161333334353136333937333332653161353861316461626231353461343465396636316161633864366632643233663863383864646234353063326230656266633738316332393666383437614000000000000000bcdd784e30b11b3d80b039e951e2de9ce68bc9e12254f588872a4cb05621916267f41835ff570ed81f53e4553f055b6b3e691d75910b4e5ae0d9d5556f5b0e08".to_string();
        let query = QueryTraderOrderZkos::decode_from_hex_string(query_str).unwrap();
        match relayer.trader_order_info(query).await {
            Ok(order) => println!("Trader order: {:?}", order),
            Err(e) => {
                println!("Error getting trader order: {:?}", e);
                assert!(false);
            }
        }
    }
    #[tokio::test]
    #[ignore]
    async fn test_lend_order_info() {
        dotenv::dotenv().ok();
        let relayer_url = std::env::var("RELAYER_API_RPC_SERVER_URL")
            .unwrap_or("http://0.0.0.0:8088/api".to_string());
        let relayer = RelayerJsonRpcClient::new(&relayer_url).unwrap();
        let query_str = "8a00000000000000306366383562333461346538383936643232333035623462333831636335663237383535373631616361356361323761376237636366663561663536623330343238336334303266656363363634346561323638663031643661643466356631663736663066383631386261323533353963623462353064623530653638363733313031653037653934050000008a000000000000003063663835623334613465383839366432323330356234623338316363356632373835353736316163613563613237613762376363666635616635366233303432383363343032666563633636343465613236386630316436616434663566316637366630663836313862613235333539636234623530646235306536383637333130316530376539344000000000000000622ec112f3087ef15f6265c4d600c53098ebb27310f3296f1dbcfa3c165a7b4ce780d5cf9fc7dd2ef89fe127ef16188fe5811b6e4a8f1e0c90c92bb96be7a302".to_string();
        let query = QueryLendOrderZkos::decode_from_hex_string(query_str).unwrap();
        match relayer.lend_order_info(query).await {
            Ok(order) => println!("Lend order: {:?}", order),
            Err(e) => {
                println!("Error getting lend order: {:?}", e);
                assert!(false);
            }
        }
    }
    #[tokio::test]
    async fn test_server_time() {
        dotenv::dotenv().ok();
        let relayer_url = std::env::var("RELAYER_API_RPC_SERVER_URL")
            .unwrap_or("http://0.0.0.0:8088/api".to_string());
        let relayer = RelayerJsonRpcClient::new(&relayer_url).unwrap();
        match relayer.server_time().await {
            Ok(time) => println!("Server time: {:?}", time),
            Err(e) => {
                println!("Error getting server time: {:?}", e);
                assert!(false);
            }
        }
    }
}
