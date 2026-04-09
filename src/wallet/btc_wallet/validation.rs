use bitcoin::{Address, Network};
use std::str::FromStr;

/// Validate that a BTC address is a native SegWit address for the configured network.
/// Rejects Taproot (bc1p/tb1p) and other address types.
pub fn validate_btc_segwit_address(addr: &str) -> Result<(), String> {
    let network = if crate::config::is_btc_mainnet() {
        Network::Bitcoin
    } else {
        Network::Testnet
    };
    let parsed = Address::from_str(addr)
        .map_err(|e| format!("Invalid BTC address: {}", e))?
        .require_network(network)
        .map_err(|e| format!("Address network mismatch: {}", e))?;
    let addr_str = parsed.to_string();
    let (segwit_prefix, taproot_prefix) = if crate::config::is_btc_mainnet() {
        ("bc1q", "bc1p")
    } else {
        ("tb1q", "tb1p")
    };
    if !addr_str.starts_with(segwit_prefix) {
        if addr_str.starts_with(taproot_prefix) {
            return Err(
                "Taproot addresses are not supported. Use a native SegWit address."
                    .to_string(),
            );
        }
        return Err(format!(
            "Address must be a native SegWit address ({}...)",
            segwit_prefix
        ));
    }
    Ok(())
}
