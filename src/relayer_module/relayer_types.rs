#![allow(non_camel_case_types)]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, de};
pub use twilight_client_sdk::relayer_types::*;
pub use twilight_client_sdk::zkvm::IOType;
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
        #[serde(skip_serializing_if = "Option::is_none")]
        limit: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        offset: Option<i64>,
    },
    AccountId {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<OrderStatus>,
        #[serde(skip_serializing_if = "Option::is_none")]
        limit: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        offset: Option<i64>,
    },
    RequestId {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<OrderStatus>,
        #[serde(skip_serializing_if = "Option::is_none")]
        limit: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        offset: Option<i64>,
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
    pub id: i64,
    pub sequence: i64,
    pub nonce: i64,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub total_pool_share: f64,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub total_locked_value: f64,
    pub pending_orders: i64,
    pub aggregate_log_sequence: i64,
    #[serde(default)]
    pub last_snapshot_id: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RequestResponse {
    pub msg: String,
    pub id_key: String,
}

// --- New types for additional relayer endpoints ---

/// Open interest data (long/short exposure).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OpenInterest {
    #[serde(deserialize_with = "from_str_to_f64")]
    pub long_exposure: f64,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub short_exposure: f64,
    #[serde(default)]
    pub last_order_timestamp: Option<String>,
}

/// Risk parameters returned inside `MarketStats`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RiskParams {
    pub max_oi_mult: f64,
    pub max_net_mult: f64,
    pub max_position_pct: f64,
    pub min_position_btc: f64,
    pub max_leverage: f64,
    pub mm_ratio: f64,
}

/// Comprehensive market risk statistics from `get_market_stats`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MarketStats {
    pub pool_equity_btc: f64,
    pub total_long_btc: f64,
    pub total_short_btc: f64,
    pub total_pending_long_btc: f64,
    pub total_pending_short_btc: f64,
    pub open_interest_btc: f64,
    pub net_exposure_btc: f64,
    pub long_pct: f64,
    pub short_pct: f64,
    pub utilization: f64,
    pub max_long_btc: f64,
    pub max_short_btc: f64,
    pub status: String,
    pub status_reason: Option<String>,
    pub params: RiskParams,
}

/// Parameters for the `apy_chart` endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct ApyChartArgs {
    pub range: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lookback: Option<String>,
}

/// Single data point in an APY chart.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ApyChartPoint {
    pub bucket_ts: String,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub apy: f64,
}

/// Parameters for `account_summary_by_twilight_address`.
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountSummaryArgs {
    pub t_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(with = "option_rfc3339_date")]
    pub from: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(with = "option_rfc3339_date")]
    pub to: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(with = "option_rfc3339_date")]
    pub since: Option<DateTime<Utc>>,
}

/// Response from `account_summary_by_twilight_address`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AccountSummary {
    pub from: String,
    pub to: String,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub settled_positionsize: f64,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub filled_positionsize: f64,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub liquidated_positionsize: f64,
    pub settled_count: i64,
    pub filled_count: i64,
    pub liquidated_count: i64,
}

/// Parameters for `all_account_summaries`.
#[derive(Debug, Serialize, Deserialize)]
pub struct AllAccountSummariesArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(with = "option_rfc3339_date")]
    pub from: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(with = "option_rfc3339_date")]
    pub to: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(with = "option_rfc3339_date")]
    pub since: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

/// Single account entry within `AllAccountSummariesResponse`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AccountSummaryEntry {
    pub twilight_address: String,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub settled_positionsize: f64,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub filled_positionsize: f64,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub liquidated_positionsize: f64,
    pub settled_count: i64,
    pub filled_count: i64,
    pub liquidated_count: i64,
}

/// Response from `all_account_summaries`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AllAccountSummariesResponse {
    pub from: String,
    pub to: String,
    pub limit: i64,
    pub offset: i64,
    pub summaries: Vec<AccountSummaryEntry>,
}

/// Price trigger details shared by settle_limit, take_profit, and stop_loss
/// in the `TraderOrderV1` response.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OrderTrigger {
    pub uuid: Uuid,
    pub position_type: PositionType,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub price: f64,
    #[serde(default)]
    pub created_time: Option<String>,
}

/// Backwards-compatible alias.
pub type SettleLimit = OrderTrigger;

/// Enhanced trader order info (v1) with settle_limit, take_profit, stop_loss,
/// and funding_applied.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TraderOrderV1 {
    #[serde(flatten)]
    pub order: TraderOrder,
    #[serde(default)]
    pub settle_limit: Option<OrderTrigger>,
    #[serde(default)]
    pub take_profit: Option<OrderTrigger>,
    #[serde(default)]
    pub stop_loss: Option<OrderTrigger>,
    #[serde(default)]
    #[serde(deserialize_with = "option_from_str_to_f64")]
    pub funding_applied: Option<f64>,
}

/// A single funding history entry from `order_funding_history`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FundingHistoryEntry {
    pub time: String,
    pub position_side: PositionType,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub payment: f64,
    #[serde(deserialize_with = "from_str_to_f64")]
    pub funding_rate: f64,
    pub order_id: Uuid,
}

/// Unrealised profit details for a lend position (returned by `lend_order_info_v1`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct UnrealisedProfit {
    pub u_pnl: f64,
    pub apr: f64,
    pub timestamp: String,
}

/// Enhanced lend order info (v1) with unrealised profit data.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LendOrderV1 {
    #[serde(flatten)]
    pub order: LendOrder,
    #[serde(default)]
    pub unrealised_profit: Option<UnrealisedProfit>,
}

/// Deserialize an optional string-or-number to `Option<f64>`.
pub fn option_from_str_to_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum MaybeF64 {
        Str(String),
        Num(f64),
        Null,
    }
    match Option::<MaybeF64>::deserialize(deserializer)? {
        Some(MaybeF64::Str(s)) => s.parse::<f64>().map(Some).map_err(de::Error::custom),
        Some(MaybeF64::Num(n)) => Ok(Some(n)),
        Some(MaybeF64::Null) | None => Ok(None),
    }
}

/// Optional RFC3339 date (de)serializer for `Option<DateTime<Utc>>`.
mod option_rfc3339_date {
    use chrono::{DateTime, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(d) => serializer.serialize_str(&d.to_rfc3339()),
            None => serializer.serialize_none(),
        }
    }

    #[allow(dead_code)]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<String> = Option::deserialize(deserializer)?;
        match opt {
            Some(s) => chrono::DateTime::parse_from_rfc3339(&s)
                .map_err(serde::de::Error::custom)
                .map(|dt| Some(dt.with_timezone(&Utc))),
            None => Ok(None),
        }
    }
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
