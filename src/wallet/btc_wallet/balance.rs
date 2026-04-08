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
    let (primary, fallback) = crate::config::esplora_endpoints();

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

/// Fetch the confirmation count for a Bitcoin transaction via Esplora.
/// Returns `(confirmed, confirmations)` — confirmed is true if the tx is in a block.
pub async fn fetch_tx_confirmations(txid: &str) -> anyhow::Result<(bool, u32)> {
    let client = Client::new();
    let (primary, fallback) = crate::config::esplora_endpoints();

    let json = {
        let url = format!("{}/tx/{}", primary, txid);
        let result = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await;
        match result {
            Ok(resp) if resp.status().is_success() => resp.json::<Value>().await?,
            _ => {
                let url = format!("{}/tx/{}", fallback, txid);
                let resp = client
                    .get(&url)
                    .timeout(std::time::Duration::from_secs(15))
                    .send()
                    .await?;
                if !resp.status().is_success() {
                    return Err(anyhow!("Could not fetch tx {txid} from Esplora"));
                }
                resp.json::<Value>().await?
            }
        }
    };

    let confirmed = json
        .pointer("/status/confirmed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !confirmed {
        return Ok((false, 0));
    }

    let block_height = json
        .pointer("/status/block_height")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    if block_height == 0 {
        return Ok((true, 1)); // confirmed but can't determine exact count
    }

    // Get current tip height to calculate confirmations
    let tip_url = format!("{}/blocks/tip/height", primary);
    let tip_height = match client
        .get(&tip_url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            resp.text().await?.trim().parse::<u64>().unwrap_or(0)
        }
        _ => {
            let tip_url = format!("{}/blocks/tip/height", fallback);
            match client
                .get(&tip_url)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    resp.text().await?.trim().parse::<u64>().unwrap_or(0)
                }
                _ => 0,
            }
        }
    };

    let confirmations = if tip_height >= block_height {
        (tip_height - block_height + 1) as u32
    } else {
        1
    };

    Ok((true, confirmations))
}
