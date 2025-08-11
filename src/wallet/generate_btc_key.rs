use bip39::{Language, Mnemonic};
use bitcoin::{
    Address, CompressedPublicKey, Network, NetworkKind, PrivateKey, PublicKey,
    bip32::{DerivationPath, Xpriv},
    secp256k1::Secp256k1,
};
use std::str::FromStr;

/// Returns (WIF, bc1… address)
pub fn segwit_from_mnemonic(mnemonic: &str) -> anyhow::Result<(String, String)> {
    // 1. BIP‑39 seed
    let mnemonic = Mnemonic::parse_in(Language::English, mnemonic)?;
    let seed = mnemonic.to_seed(""); // empty pass‑phrase → deterministic

    // 2. Master XPrv on main‑net
    let master = Xpriv::new_master(Network::Bitcoin, &seed)?;

    // 3. BIP‑84 path m/84'/0'/0'/0/0  (purpose 84', coin‑type 0', account 0', external 0, index 0)
    let path = DerivationPath::from_str("m/84'/0'/0'/0/0")?;
    let secp = Secp256k1::signing_only();
    let child = master.derive_priv(&secp, &path)?;

    // 4. Keys
    let privkey = PrivateKey {
        compressed: true,
        network: NetworkKind::Main,
        inner: child.private_key,
    };
    let pubkey: PublicKey = privkey.public_key(&secp);
    let compressed_pubkey = CompressedPublicKey::from_slice(&pubkey.to_bytes()).unwrap();
    // 5. Native SegWit (bech32 bc1…) address
    let addr = Address::p2wpkh(&compressed_pubkey, Network::Bitcoin);

    Ok((privkey.to_wif(), addr.to_string()))
}

pub fn segwit_from_private_key(private_key: &str) -> anyhow::Result<(String, String)> {
    let privkey = PrivateKey::from_str(private_key)?;
    let secp = Secp256k1::signing_only();
    let pubkey: PublicKey = privkey.public_key(&secp);
    let compressed_pubkey = CompressedPublicKey::from_slice(&pubkey.to_bytes()).unwrap();
    let addr = Address::p2wpkh(&compressed_pubkey, Network::Bitcoin);
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
        // // Check WIF format is correct
        // assert!(wif.starts_with("c"));
        // assert_eq!(wif.len(), 52);

        // // Check address format is correct
        // assert!(address.to_string().starts_with("tb1q"));
        // assert_eq!(address.to_string().len(), 42);
    }
    #[test]
    fn test_segwit_from_private_key() {
        let private_key = "Ky3HTdELEKGJaHBXn3sstmxWbiJVNinKUnZoDanPpBR6czAPMMVg";
        let (wif, address) = segwit_from_private_key(private_key).unwrap();
        println!("WIF: {}", wif);
        println!("Address: {}", address);
    }
}
