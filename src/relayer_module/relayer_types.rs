#![allow(non_camel_case_types)]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, de};
pub use twilight_client_sdk::relayer_types::{
    LendOrder, OrderStatus, OrderType, PositionType, TXType, TraderOrder, TxHash,
};
use uuid::Uuid;
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalPriceArgs {
    #[serde(with = "rfc3339_date")]
    pub from: DateTime<Utc>,
    #[serde(with = "rfc3339_date")]
    pub to: DateTime<Utc>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Candles {
    pub interval: Interval,
    #[serde(with = "rfc3339_date")]
    pub since: DateTime<Utc>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Copy, Eq, Hash, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum Interval {
    #[serde(alias = "1 minute")]
    ONE_MINUTE,
    #[serde(alias = "5 minutes")]
    FIVE_MINUTE,
    #[serde(alias = "15 minutes")]
    FIFTEEN_MINUTE,
    #[serde(alias = "30 minutes")]
    THIRTY_MINUTE,
    #[serde(alias = "1 hour")]
    ONE_HOUR,
    #[serde(alias = "4 hours")]
    FOUR_HOUR,
    #[serde(alias = "8 hours")]
    EIGHT_HOUR,
    #[serde(alias = "12 hours")]
    TWELVE_HOUR,
    #[serde(alias = "1 day")]
    ONE_DAY,
    #[serde(alias = "1 day change")]
    ONE_DAY_CHANGE,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoricalFundingArgs {
    #[serde(with = "rfc3339_date")]
    pub from: DateTime<Utc>,
    #[serde(with = "rfc3339_date")]
    pub to: DateTime<Utc>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoricalFeeArgs {
    #[serde(with = "rfc3339_date")]
    pub from: DateTime<Utc>,
    #[serde(with = "rfc3339_date")]
    pub to: DateTime<Utc>,
    pub limit: i64,
    pub offset: i64,
}
// use jsonrpsee_core::Error;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub enum TransactionHashArgs {
    TxId {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<OrderStatus>,
    },
    AccountId {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<OrderStatus>,
    },
    RequestId {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<OrderStatus>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BtcUsdPrice {
    pub id: i64,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub price: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FundingRate {
    pub id: i64,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub rate: f64,
    #[serde(deserialize_with = "from_str_to_f64")]
    #[serde(rename = "price")]
    pub btc_price: f64,
    #[serde(with = "rfc3339_date")]
    pub timestamp: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Candle {
    pub resolution: Interval,
    #[serde(with = "rfc3339_date")]
    #[serde(rename = "start")]
    pub started_at: DateTime<Utc>,
    #[serde(with = "rfc3339_date")]
    #[serde(rename = "end")]
    pub end: DateTime<Utc>,
    #[serde(with = "rfc3339_date")]
    pub updated_at: DateTime<Utc>,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub low: f64,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub high: f64,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub open: f64,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub close: f64,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub btc_volume: f64,
    pub trades: i32,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub usd_volume: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FeeHistory {
    #[serde(deserialize_with = "from_str_to_f64")]
    pub order_filled_on_market: f64,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub order_filled_on_limit: f64,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub order_settled_on_market: f64,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub order_settled_on_limit: f64,
    #[serde(with = "rfc3339_date")]
    pub timestamp: DateTime<Utc>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderBook {
    pub bid: Vec<Bid>,
    pub ask: Vec<Ask>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bid {
    pub positionsize: f64,
    pub price: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ask {
    pub positionsize: f64,
    pub price: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(transparent)]
pub struct RecentOrders {
    pub orders: Vec<CloseTrade>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Side {
    BUY,
    SELL,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CloseTrade {
    pub order_id: Uuid,
    pub side: PositionType,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub positionsize: f64,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub price: f64,
    #[serde(with = "rfc3339_date")]
    pub timestamp: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PositionSize {
    #[serde(rename = "total_short")]
    #[serde(deserialize_with = "from_str_to_f64")]
    pub total_short_position_size: f64,
    #[serde(rename = "total_long")]
    #[serde(deserialize_with = "from_str_to_f64")]
    pub total_long_position_size: f64,
    #[serde(rename = "total")]
    #[serde(deserialize_with = "from_str_to_f64")]
    pub total_position_size: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LendPoolInfo {
    id: i64,
    sequence: i64,
    nonce: i64,
    #[serde(deserialize_with = "from_str_to_f64")]
    total_pool_share: f64,
    #[serde(deserialize_with = "from_str_to_f64")]
    total_locked_value: f64,
    pending_orders: i64,
    aggregate_log_sequence: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RequestResponse {
    pub msg: String,
    pub id_key: String,
}

// Custom (de)serializer to enforce RFC3339 formatted date strings when talking to the relayer
mod rfc3339_date {
    use chrono::{DateTime, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&date.to_rfc3339())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        chrono::DateTime::parse_from_rfc3339(&s)
            .map_err(serde::de::Error::custom)
            .map(|dt| dt.with_timezone(&Utc))
    }
}

pub fn from_str_to_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    // Accept either "113655" (string) *or* 113655 (number)
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = f64;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "a string or number that can be parsed as f64")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            v.parse::<f64>().map_err(E::custom)
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            v.parse::<f64>().map_err(E::custom)
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(v as f64)
        }

        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(v)
        }
    }

    deserializer.deserialize_any(Visitor)
}
