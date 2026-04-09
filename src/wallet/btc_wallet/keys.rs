use bip39::{Language, Mnemonic};
use bitcoin::{
    Address, CompressedPublicKey, Network, NetworkKind, PrivateKey, PublicKey,
    bip32::{DerivationPath, Xpriv},
    secp256k1::Secp256k1,
};
use std::str::FromStr;

fn btc_network() -> (Network, NetworkKind) {
    if crate::config::is_btc_mainnet() {
        (Network::Bitcoin, NetworkKind::Main)
    } else {
        (Network::Testnet, NetworkKind::Test)
    }
}

/// Returns (WIF, bc1q/tb1q address)
pub fn segwit_from_mnemonic(mnemonic: &str) -> anyhow::Result<(String, String)> {
    let mnemonic = Mnemonic::parse_in(Language::English, mnemonic)?;
    let seed = mnemonic.to_seed("");

    let (network, network_kind) = btc_network();

    let master = Xpriv::new_master(network, &seed)?;
    let path = DerivationPath::from_str("m/84'/0'/0'/0/0")?;
    let secp = Secp256k1::signing_only();
    let child = master.derive_priv(&secp, &path)?;

    let privkey = PrivateKey {
        compressed: true,
        network: network_kind,
        inner: child.private_key,
    };
    let pubkey: PublicKey = privkey.public_key(&secp);
    let compressed_pubkey = CompressedPublicKey::from_slice(&pubkey.to_bytes()).unwrap();
    let addr = Address::p2wpkh(&compressed_pubkey, network);

    Ok((privkey.to_wif(), addr.to_string()))
}

/// Generate a random valid BTC segwit address using a fresh mnemonic.
/// Returns (WIF, bc1q/tb1q address).
pub fn generate_random_btc_address() -> anyhow::Result<(String, String)> {
    let mnemonic = Mnemonic::generate_in(Language::English, 24)?;
    segwit_from_mnemonic(&mnemonic.to_string())
}

pub fn segwit_from_private_key(private_key: &str) -> anyhow::Result<(String, String)> {
    let privkey = PrivateKey::from_str(private_key)?;
    let secp = Secp256k1::signing_only();
    let pubkey: PublicKey = privkey.public_key(&secp);
    let compressed_pubkey = CompressedPublicKey::from_slice(&pubkey.to_bytes()).unwrap();
    let (network, _) = btc_network();
    let addr = Address::p2wpkh(&compressed_pubkey, network);
    Ok((privkey.to_wif(), addr.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segwit_from_mnemonic() {
        let mnemonic = "fragile suffer other retire often wrong ribbon alcohol wine dutch wet cancel physical dignity awkward trophy atom twist cover seminar voice only describe slide";
        let (wif, address) = segwit_from_mnemonic(mnemonic).unwrap();
        println!("WIF: {}", wif);
        println!("Address: {}", address);
    }

    #[test]
    fn test_segwit_from_private_key() {
        let private_key = "Ky3HTdELEKGJaHBXn3sstmxWbiJVNinKUnZoDanPpBR6czAPMMVg";
        let (wif, address) = segwit_from_private_key(private_key).unwrap();
        println!("WIF: {}", wif);
        println!("Address: {}", address);
    }
}
