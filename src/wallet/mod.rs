pub mod wallet;
pub use wallet::*;
pub mod nyks_fn;
pub use nyks_fn::*;
pub mod faucet;
pub use faucet::*;
pub mod seed_signer;
pub use seed_signer::*;
pub mod btc_wallet;

// Backward-compat: old import path `crate::wallet::generate_btc_key::*` still works
pub mod generate_btc_key {
    pub use super::btc_wallet::keys::*;
}
