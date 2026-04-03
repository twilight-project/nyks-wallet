use anyhow::anyhow;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtcBalance {
    pub confirmed_sats: u64,
    pub unconfirmed_sats: i64,
    pub total_sats: i64,
}

/// Fetch the on-chain Bitcoin balance for a given address via Blockstream Esplora API.
/// Falls back to mempool.space if Blockstream is unavailable.
/// to get testnet token on testnet 3  use url
/// https://coinfaucet.eu/en/btc-testnet/
/// for testnet4 token, use url https://coinfaucet.eu/en/btc-testnet4/
pub async fn fetch_btc_balance(btc_address: &str) -> anyhow::Result<BtcBalance> {
    let client = Client::new();
    let (primary, fallback) = if crate::config::is_btc_mainnet() {
        (
            "https://blockstream.info/api",
            "https://mempool.space/api",
        )
    } else {
        (
            "https://blockstream.info/testnet/api",
            "https://mempool.space/testnet4/api",
        )
    };

    let json = {
        let url = format!("{}/address/{}", primary, btc_address);
        let result = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await;
        match result {
            Ok(resp) if resp.status().is_success() => resp.json::<Value>().await?,
            _ => {
                // Fallback to mempool.space
                let url = format!("{}/address/{}", fallback, btc_address);
                let resp = client
                    .get(&url)
                    .timeout(std::time::Duration::from_secs(15))
                    .send()
                    .await?;
                if !resp.status().is_success() {
                    return Err(anyhow!(
                        "Both Blockstream and mempool.space failed (HTTP {})",
                        resp.status()
                    ));
                }
                resp.json::<Value>().await?
            }
        }
    };

    let funded = json
        .pointer("/chain_stats/funded_txo_sum")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let spent = json
        .pointer("/chain_stats/spent_txo_sum")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let confirmed_sats = funded.saturating_sub(spent);

    let mempool_funded = json
        .pointer("/mempool_stats/funded_txo_sum")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let mempool_spent = json
        .pointer("/mempool_stats/spent_txo_sum")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let unconfirmed_sats = mempool_funded as i64 - mempool_spent as i64;

    Ok(BtcBalance {
        confirmed_sats,
        unconfirmed_sats,
        total_sats: confirmed_sats as i64 + unconfirmed_sats,
    })
}

/// Fetch the current Bitcoin block height from the Twilight indexer.
pub async fn fetch_btc_block_height() -> anyhow::Result<u64> {
    let client = Client::new();
    let url = format!(
        "{}/api/bitcoin/info",
        crate::config::TWILIGHT_INDEXER_URL.as_str()
    );
    let response = client.get(&url).send().await?;
    let json: Value = response.json().await?;
    let height = json
        .get("blockHeight")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow!("Missing blockHeight in indexer response"))?;
    Ok(height)
}
//https://blockstream.info/api/blocks/tip/height
