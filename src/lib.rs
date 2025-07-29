pub mod nyks_rpc;
pub mod wallet;
pub use wallet::*;
pub mod test;
pub mod zkos_accounts;

#[macro_use]
extern crate lazy_static;
// ----------------------------------------------------------------------------
// Generated protobuf module (prost-build)
// ----------------------------------------------------------------------------

pub mod nyks {
    pub mod module {
        pub mod bridge {
            include!(concat!(env!("OUT_DIR"), "/twilightproject.nyks.bridge.rs"));
        }
        pub mod zkos {
            include!(concat!(env!("OUT_DIR"), "/twilightproject.nyks.zkos.rs"));
        }
    }
}

pub use nyks::module::bridge::MsgRegisterBtcDepositAddress;
pub use nyks::module::zkos::MsgMintBurnTradingBtc;
pub use nyks::module::zkos::MsgTransferTx;

// -------------------------------------------------------------
// Optional validator-wallet feature
// -------------------------------------------------------------
#[cfg(feature = "validator-wallet")]
pub mod validator_wallet;

#[cfg(feature = "validator-wallet")]
pub use validator_wallet::*;
