pub mod faucet;
pub mod test;
pub mod wallet;
// ----------------------------------------------------------------------------
// Generated protobuf module (prost-build)
// ----------------------------------------------------------------------------

pub mod nyks {
    pub mod module {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/twilightproject.nyks.bridge.rs"));
        }
    }
}

pub use nyks::module::v1::MsgRegisterBtcDepositAddress;
