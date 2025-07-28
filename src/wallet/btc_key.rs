// use bitcoin::{
//     Address, CompressedPublicKey, Network, NetworkKind, PrivateKey, PublicKey,
//     secp256k1::{Secp256k1, SecretKey, rand::thread_rng},
// };
// // use rand::thread_rng;
// use bip32::{DerivationPath, XPrv};
// use bip39::{Language, Mnemonic};
// pub fn generate_btc_key_from_mnemonic(mnemonic: &str) -> (String, Address) {
//     let mnemonic = Mnemonic::parse_in(Language::English, mnemonic).unwrap();
//     let path: DerivationPath = "m/44'/118'/0'/0/0".parse().unwrap();
//     let seed = mnemonic.to_seed("");
//     let xprv = XPrv::derive_from_path(&seed, &path).unwrap();
//     let secp = Secp256k1::new();
//     let privkey = PrivateKey {
//         compressed: true,
//         network: NetworkKind::Main,
//         inner: SecretKey::from_slice(&xprv.private_key().to_bytes()).unwrap(),
//     };
//     let pubkey = PublicKey::from_private_key(&secp, &privkey);
//     let compressed_pubkey = CompressedPublicKey::from_slice(&pubkey.to_bytes()).unwrap();
//     let segwit_addr = Address::p2wpkh(&compressed_pubkey, Network::Bitcoin);
//     (privkey.to_wif(), segwit_addr)
// }

// pub fn generate_btc_key(private_key: &str) -> (String, Address) {
//     // 1. Create a random private key
//     let secp = Secp256k1::new();
//     // let secret_key = SecretKey::from_slice(private_key.as_bytes()).unwrap();
//     // let secret_key = SecretKey::new(&mut thread_rng());
//     let pk_bytes = hex::decode(private_key).unwrap();
//     let secret_key = SecretKey::from_slice(&pk_bytes).unwrap();
//     let privkey = PrivateKey {
//         compressed: true,
//         network: NetworkKind::Main,
//         inner: secret_key,
//     };

//     // 2. Derive the compressed public key
//     let pubkey = PublicKey::from_private_key(&secp, &privkey);
//     let compressed_pubkey = CompressedPublicKey::from_slice(&pubkey.to_bytes()).unwrap();
//     // 3. Build a P2WPKH (native-SegWit) address
//     let segwit_addr = Address::p2wpkh(&compressed_pubkey, Network::Bitcoin);

//     println!("WIF private key : {}", privkey.to_wif());
//     println!("Native SegWit address (bech32): {}", segwit_addr);
//     (privkey.to_wif(), segwit_addr)
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_generate_btc_key() {
//         let private_key = "80f346ad96bd811acac24f2b814286d16fcd7e4e01449b39dc7b9cd39b238e63";
//         let (wif, address) = generate_btc_key(private_key);

//         // // Check WIF format is correct
//         // assert!(wif.starts_with("c"));
//         // assert_eq!(wif.len(), 52);

//         // // Check address format is correct
//         // assert!(address.to_string().starts_with("tb1q"));
//         // assert_eq!(address.to_string().len(), 42);
//     }
// }

use bip39::{Language, Mnemonic};
use bitcoin::{
    Address, CompressedPublicKey, Network, NetworkKind, PrivateKey, PublicKey,
    bip32::{DerivationPath, ExtendedPrivKey},
    secp256k1::Secp256k1,
};
use std::str::FromStr;

/// Returns (WIF, bc1… address)
pub fn segwit_from_mnemonic(mnemonic: &str) -> anyhow::Result<(String, String)> {
    // 1. BIP‑39 seed
    let mnemonic = Mnemonic::parse_in(Language::English, mnemonic)?;
    let seed = mnemonic.to_seed(""); // empty pass‑phrase → deterministic

    // 2. Master XPrv on main‑net
    let master = ExtendedPrivKey::new_master(Network::Bitcoin, &seed)?;

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
}
