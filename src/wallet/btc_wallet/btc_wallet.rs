use bitcoin::Network;
use serde::{Deserialize, Serialize};
use zeroize::ZeroizeOnDrop;

/// BIP-84 derivation path for native SegWit: m/84'/0'/0'/0/0
pub const BTC_DERIVATION_PATH: &str = "m/84'/0'/0'/0/0";

/// Minimal BTC key data — used to construct a `bdk_wallet::Wallet` on demand.
#[derive(Clone, Serialize, Deserialize, ZeroizeOnDrop)]
pub struct BtcWallet {
    /// WIF-encoded private key (needed to build BDK descriptor)
    wif: String,
    /// Native SegWit address (bc1q... / tb1q...)
    #[zeroize(skip)]
    pub address: String,
    /// Bitcoin network
    #[zeroize(skip)]
    pub network: BtcNetwork,
}

/// Serde-friendly Bitcoin network enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BtcNetwork {
    Mainnet,
    Testnet,
}

impl BtcNetwork {
    pub fn to_bitcoin_network(self) -> Network {
        match self {
            BtcNetwork::Mainnet => Network::Bitcoin,
            BtcNetwork::Testnet => Network::Testnet,
        }
    }

    pub fn from_config() -> Self {
        if crate::config::is_btc_mainnet() {
            BtcNetwork::Mainnet
        } else {
            BtcNetwork::Testnet
        }
    }
}

impl std::fmt::Display for BtcWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BtcWallet(address={}, network={:?})",
            self.address, self.network
        )
    }
}

impl std::fmt::Debug for BtcWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BtcWallet")
            .field("address", &self.address)
            .field("network", &self.network)
            .field("wif", &"[REDACTED]")
            .finish()
    }
}

impl BtcWallet {
    /// Create from a mnemonic. Derives BIP-84 native SegWit key.
    pub fn from_mnemonic(mnemonic: &str) -> anyhow::Result<Self> {
        let (wif, address) = super::keys::segwit_from_mnemonic(mnemonic)?;
        Ok(BtcWallet {
            wif,
            address,
            network: BtcNetwork::from_config(),
        })
    }

    /// Create from a WIF private key.
    pub fn from_wif(wif: &str) -> anyhow::Result<Self> {
        let (wif, address) = super::keys::segwit_from_private_key(wif)?;
        Ok(BtcWallet {
            wif,
            address,
            network: BtcNetwork::from_config(),
        })
    }

    /// Get the WIF private key (for building BDK descriptor).
    pub fn wif(&self) -> &str {
        &self.wif
    }

    /// Build a `bdk_wallet::Wallet` from this key data.
    /// The BDK wallet handles UTXO tracking, tx building, and signing.
    pub fn create_bdk_wallet(&self) -> anyhow::Result<bdk_wallet::Wallet> {
        use bdk_wallet::Wallet as BdkWallet;

        let network = self.network.to_bitcoin_network();
        let descriptor = format!("wpkh({})", self.wif);

        let wallet = BdkWallet::create_single(descriptor)
            .network(network)
            .create_wallet_no_persist()?;

        Ok(wallet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_mnemonic() {
        let mnemonic = "test test test test test test test test test test test junk";
        let btc = BtcWallet::from_mnemonic(mnemonic).unwrap();

        assert!(btc.address.starts_with("bc1q"));
        println!("Address: {}", btc.address);
    }

    #[test]
    fn test_from_wif() {
        let wif = "Ky3HTdELEKGJaHBXn3sstmxWbiJVNinKUnZoDanPpBR6czAPMMVg";
        let btc = BtcWallet::from_wif(wif).unwrap();

        assert!(btc.address.starts_with("bc1q"));
    }

    #[test]
    fn test_consistent_with_legacy_keys() {
        let mnemonic = "test test test test test test test test test test test junk";
        let btc = BtcWallet::from_mnemonic(mnemonic).unwrap();
        let (_wif, addr) = super::super::keys::segwit_from_mnemonic(mnemonic).unwrap();

        assert_eq!(btc.address, addr);
        assert_eq!(btc.wif(), _wif);
    }

    #[test]
    fn test_create_bdk_wallet() {
        let mnemonic = "test test test test test test test test test test test junk";
        let btc = BtcWallet::from_mnemonic(mnemonic).unwrap();
        let bdk = btc.create_bdk_wallet().unwrap();

        // BDK wallet should have the same address
        let bdk_addr = bdk.peek_address(bdk_wallet::KeychainKind::External, 0);
        assert_eq!(btc.address, bdk_addr.address.to_string());
    }
}
