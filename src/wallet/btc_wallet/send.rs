use anyhow::anyhow;
use bdk_esplora::esplora_client;
use bdk_esplora::EsploraAsyncExt;
use bdk_wallet::bitcoin::Amount;
use bdk_wallet::SignOptions;
use bitcoin::Address;
use std::str::FromStr;

use super::btc_wallet::BtcWallet;

/// Parameters for a BTC send operation.
pub struct SendBtcParams<'a> {
    /// BtcWallet holding the key material
    pub btc_wallet: &'a BtcWallet,
    /// Destination bc1q address
    pub destination: String,
    /// Amount in satoshis to send
    pub amount_sats: u64,
    /// Fee rate in sat/vB (if None, estimates for ~6 block target)
    pub fee_rate_sat_vb: Option<f32>,
}

/// Result of a successful BTC send.
pub struct SendBtcResult {
    /// The broadcast transaction ID
    pub txid: String,
    /// Total fee paid in satoshis
    pub fee_sats: u64,
}

/// Build, sign, and broadcast a Bitcoin transaction via Esplora.
///
/// 1. Creates a BDK wallet from the BtcWallet key data
/// 2. Syncs UTXOs via Esplora (Blockstream, with mempool.space fallback)
/// 3. Builds a transaction to `destination` for `amount_sats`
/// 4. Signs and broadcasts
#[allow(deprecated)] // SignOptions moves to bitcoin::psbt in BDK 3.x
pub async fn send_btc(params: SendBtcParams<'_>) -> anyhow::Result<SendBtcResult> {
    let mut wallet = params.btc_wallet.create_bdk_wallet()?;
    let network = params.btc_wallet.network;

    // Pick Esplora endpoint
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

    // Try primary, fall back on failure
    let client = match esplora_client::Builder::new(primary).build_async() {
        Ok(c) => c,
        Err(_) => esplora_client::Builder::new(fallback)
            .build_async()
            .map_err(|e| anyhow!("Failed to create Esplora client: {e}"))?,
    };

    // Sync wallet UTXOs
    let request = wallet.start_full_scan().build();
    let update = client
        .full_scan(request, 5, 5)
        .await
        .map_err(|e| anyhow!("Failed to sync wallet UTXOs: {e}"))?;
    wallet.apply_update(update)?;

    // Parse destination address
    let dest_addr = Address::from_str(&params.destination)
        .map_err(|e| anyhow!("Invalid destination address: {e}"))?
        .require_network(network.to_bitcoin_network())
        .map_err(|e| anyhow!("Address network mismatch: {e}"))?;

    // Snapshot balance before building tx (for error reporting)
    let available_sats = wallet.balance().total().to_sat();

    // Build transaction — BDK handles coin selection and checks for
    // insufficient funds (including fees) via InsufficientFunds error
    let mut tx_builder = wallet.build_tx();
    tx_builder.add_recipient(dest_addr.script_pubkey(), Amount::from_sat(params.amount_sats));

    if let Some(rate) = params.fee_rate_sat_vb {
        let rate_u64 = (rate.ceil()) as u64; // round up to avoid stuck txs
        tx_builder.fee_rate(bdk_wallet::bitcoin::FeeRate::from_sat_per_vb(rate_u64).unwrap());
    }

    let mut psbt = tx_builder.finish().map_err(|e| {
        anyhow!(
            "{e}\n  Available: {available_sats} sats, requested: {} sats (+ fees)",
            params.amount_sats
        )
    })?;

    // Sign
    let finalized = wallet.sign(&mut psbt, SignOptions::default())?;
    if !finalized {
        return Err(anyhow!("Failed to finalize transaction signing"));
    }

    let tx = psbt.extract_tx()?;
    let txid = tx.compute_txid();
    let fee_sats = wallet.calculate_fee(&tx).map(|f| f.to_sat()).unwrap_or(0);

    // Broadcast
    client
        .broadcast(&tx)
        .await
        .map_err(|e| anyhow!("Broadcast failed: {e}"))?;

    Ok(SendBtcResult {
        txid: txid.to_string(),
        fee_sats,
    })
}
