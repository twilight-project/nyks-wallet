// use crate::relayer_types::{LendOrder, OrderStatus, TraderOrder, TxHash};
use serde::{Deserialize, Serialize};
// use serde_this_or_that::as_f64;
// use sha2::{Digest, Sha256};
use std::hash::Hash;
// use uuid::Uuid;
// use zkvm::{IOType, Output, Utxo};
/// Serialized as the "method" field of JSON-RPC/HTTP requests.
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
pub enum Method {
    #[allow(non_camel_case_types)]
    broadcast_tx_sync,
    #[allow(non_camel_case_types)]
    abci_info,
    #[allow(non_camel_case_types)]
    abci_query,
    #[allow(non_camel_case_types)]
    block,
    #[allow(non_camel_case_types)]
    block_by_hash,
    #[allow(non_camel_case_types)]
    block_results,
    #[allow(non_camel_case_types)]
    block_search,
    #[allow(non_camel_case_types)]
    blockchain,
    #[allow(non_camel_case_types)]
    broadcast_evidence,
    #[allow(non_camel_case_types)]
    broadcast_tx_async,
    #[allow(non_camel_case_types)]
    broadcast_tx_commit,
    #[allow(non_camel_case_types)]
    check_tx,
    #[allow(non_camel_case_types)]
    commit,
    #[allow(non_camel_case_types)]
    consensus_params,
    #[allow(non_camel_case_types)]
    consensus_state,
    #[allow(non_camel_case_types)]
    dump_consensus_state,
    #[allow(non_camel_case_types)]
    genesis,
    #[allow(non_camel_case_types)]
    genesis_chunked,
    #[allow(non_camel_case_types)]
    health,
    #[allow(non_camel_case_types)]
    net_info,
    #[allow(non_camel_case_types)]
    num_unconfirmed_txs,
    #[allow(non_camel_case_types)]
    status,
    #[allow(non_camel_case_types)]
    subscribe,
    #[allow(non_camel_case_types)]
    tx,
    #[allow(non_camel_case_types)]
    tx_search,
    #[allow(non_camel_case_types)]
    unconfirmed_txs,
    #[allow(non_camel_case_types)]
    unsubscribe,
    #[allow(non_camel_case_types)]
    unsubscribe_all,
    #[allow(non_camel_case_types)]
    validators,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
pub enum MethodTypeURL {
    // ---- zkos module ----
    MsgMintBurnTradingBtc,
    MsgTransferTx,

    // ---- bridge module ----
    MsgConfirmBtcDeposit,
    MsgRegisterBtcDepositAddress,
    MsgRegisterReserveAddress,
    MsgBootstrapFragment,
    MsgProposeRefundHash,
    MsgWithdrawBtcRequest,
    MsgWithdrawTxSigned,
    MsgWithdrawTxFinal,
    MsgConfirmBtcWithdraw,
    MsgProposeSweepAddress,
    MsgUnsignedTxSweep,
    MsgUnsignedTxRefund,
    MsgSignRefund,
    MsgSignSweep,
    MsgBroadcastTxRefund,
    MsgBroadcastTxSweep,
    MsgSweepProposal,
}
use anyhow::anyhow;
use base64::{Engine as _, engine::general_purpose};
use cosmrs::crypto::{PublicKey, secp256k1::SigningKey};
use cosmrs::{
    tendermint::chain::Id as ChainId,
    tx::{Body, Fee, SignDoc, SignerInfo},
};
use std::str::FromStr;
impl MethodTypeURL {
    pub fn type_url<T>(&self, msg: T) -> cosmrs::Any
    where
        T: prost::Message,
    {
        match self {
            // ---- zkos module ----
            MethodTypeURL::MsgMintBurnTradingBtc => {
                let mut buf = Vec::new();
                msg.encode(&mut buf).expect("msg encoding failed");
                let any = cosmrs::Any {
                    type_url: "/twilightproject.nyks.zkos.MsgMintBurnTradingBtc".to_string(),
                    value: buf,
                };
                any
            }
            MethodTypeURL::MsgTransferTx => {
                let mut buf = Vec::new();
                msg.encode(&mut buf).expect("msg encoding failed");
                let any = cosmrs::Any {
                    type_url: "/twilightproject.nyks.zkos.MsgTransferTx".to_string(),
                    value: buf,
                };
                any
            }

            // ---- bridge module ----
            MethodTypeURL::MsgRegisterBtcDepositAddress => {
                let mut buf = Vec::new();
                msg.encode(&mut buf).expect("msg encoding failed");
                let any = cosmrs::Any {
                    type_url: "/twilightproject.nyks.bridge.MsgRegisterBtcDepositAddress"
                        .to_string(),
                    value: buf,
                };
                any
            }
            MethodTypeURL::MsgConfirmBtcDeposit => {
                let mut buf = Vec::new();
                msg.encode(&mut buf).expect("msg encoding failed");
                let any = cosmrs::Any {
                    type_url: "/twilightproject.nyks.bridge.MsgConfirmBtcDeposit".to_string(),
                    value: buf,
                };
                any
            }
            MethodTypeURL::MsgRegisterReserveAddress => {
                let mut buf = Vec::new();
                msg.encode(&mut buf).expect("msg encoding failed");
                let any = cosmrs::Any {
                    type_url: "/twilightproject.nyks.bridge.MsgRegisterReserveAddress".to_string(),
                    value: buf,
                };
                any
            }
            MethodTypeURL::MsgBootstrapFragment => {
                let mut buf = Vec::new();
                msg.encode(&mut buf).expect("msg encoding failed");
                let any = cosmrs::Any {
                    type_url: "/twilightproject.nyks.bridge.MsgBootstrapFragment".to_string(),
                    value: buf,
                };
                any
            }
            MethodTypeURL::MsgProposeRefundHash => {
                let mut buf = Vec::new();
                msg.encode(&mut buf).expect("msg encoding failed");
                let any = cosmrs::Any {
                    type_url: "/twilightproject.nyks.bridge.MsgProposeRefundHash".to_string(),
                    value: buf,
                };
                any
            }
            MethodTypeURL::MsgWithdrawBtcRequest => {
                let mut buf = Vec::new();
                msg.encode(&mut buf).expect("msg encoding failed");
                let any = cosmrs::Any {
                    type_url: "/twilightproject.nyks.bridge.MsgWithdrawBtcRequest".to_string(),
                    value: buf,
                };
                any
            }
            MethodTypeURL::MsgWithdrawTxSigned => {
                let mut buf = Vec::new();
                msg.encode(&mut buf).expect("msg encoding failed");
                let any = cosmrs::Any {
                    type_url: "/twilightproject.nyks.bridge.MsgWithdrawTxSigned".to_string(),
                    value: buf,
                };
                any
            }
            MethodTypeURL::MsgWithdrawTxFinal => {
                let mut buf = Vec::new();
                msg.encode(&mut buf).expect("msg encoding failed");
                let any = cosmrs::Any {
                    type_url: "/twilightproject.nyks.bridge.MsgWithdrawTxFinal".to_string(),
                    value: buf,
                };
                any
            }
            MethodTypeURL::MsgConfirmBtcWithdraw => {
                let mut buf = Vec::new();
                msg.encode(&mut buf).expect("msg encoding failed");
                let any = cosmrs::Any {
                    type_url: "/twilightproject.nyks.bridge.MsgConfirmBtcWithdraw".to_string(),
                    value: buf,
                };
                any
            }
            MethodTypeURL::MsgProposeSweepAddress => {
                let mut buf = Vec::new();
                msg.encode(&mut buf).expect("msg encoding failed");
                let any = cosmrs::Any {
                    type_url: "/twilightproject.nyks.bridge.MsgProposeSweepAddress".to_string(),
                    value: buf,
                };
                any
            }
            MethodTypeURL::MsgUnsignedTxSweep => {
                let mut buf = Vec::new();
                msg.encode(&mut buf).expect("msg encoding failed");
                let any = cosmrs::Any {
                    type_url: "/twilightproject.nyks.bridge.MsgUnsignedTxSweep".to_string(),
                    value: buf,
                };
                any
            }
            MethodTypeURL::MsgUnsignedTxRefund => {
                let mut buf = Vec::new();
                msg.encode(&mut buf).expect("msg encoding failed");
                let any = cosmrs::Any {
                    type_url: "/twilightproject.nyks.bridge.MsgUnsignedTxRefund".to_string(),
                    value: buf,
                };
                any
            }
            MethodTypeURL::MsgSignRefund => {
                let mut buf = Vec::new();
                msg.encode(&mut buf).expect("msg encoding failed");
                let any = cosmrs::Any {
                    type_url: "/twilightproject.nyks.bridge.MsgSignRefund".to_string(),
                    value: buf,
                };
                any
            }
            MethodTypeURL::MsgSignSweep => {
                let mut buf = Vec::new();
                msg.encode(&mut buf).expect("msg encoding failed");
                let any = cosmrs::Any {
                    type_url: "/twilightproject.nyks.bridge.MsgSignSweep".to_string(),
                    value: buf,
                };
                any
            }
            MethodTypeURL::MsgBroadcastTxRefund => {
                let mut buf = Vec::new();
                msg.encode(&mut buf).expect("msg encoding failed");
                let any = cosmrs::Any {
                    type_url: "/twilightproject.nyks.bridge.MsgBroadcastTxRefund".to_string(),
                    value: buf,
                };
                any
            }
            MethodTypeURL::MsgBroadcastTxSweep => {
                let mut buf = Vec::new();
                msg.encode(&mut buf).expect("msg encoding failed");
                let any = cosmrs::Any {
                    type_url: "/twilightproject.nyks.bridge.MsgBroadcastTxSweep".to_string(),
                    value: buf,
                };
                any
            }
            MethodTypeURL::MsgSweepProposal => {
                let mut buf = Vec::new();
                msg.encode(&mut buf).expect("msg encoding failed");
                let any = cosmrs::Any {
                    type_url: "/twilightproject.nyks.bridge.MsgSweepProposal".to_string(),
                    value: buf,
                };
                any
            }
        }
    }

    pub fn sign_msg<T>(
        &self,
        any: cosmrs::Any,
        pk: PublicKey,
        sequence: u64,
        account_number: u64,
        sk: SigningKey,
    ) -> Result<String, anyhow::Error> {
        let body = Body::new(vec![any], "", 0u16);

        let fee = Fee::from_amount_and_gas(
            cosmrs::Coin {
                denom: cosmrs::Denom::from_str("nyks").map_err(|e| anyhow!("{}", e))?,
                amount: 1_000u64.into(),
            },
            200_0000u64,
        );
        let auth_info = SignerInfo::single_direct(Some(pk.into()), sequence).auth_info(fee);
        let chain_id = ChainId::try_from("nyks").map_err(|e| anyhow!("{}", e))?;

        let sign_doc = SignDoc::new(&body, &auth_info, &chain_id, account_number)
            .map_err(|e| anyhow!("{}", e))?;

        let raw_tx = sign_doc.sign(&sk).map_err(|e| anyhow!("{}", e))?;
        let tx_bytes = raw_tx.to_bytes().map_err(|e| anyhow!("{}", e))?;
        let tx_base64 = general_purpose::STANDARD.encode(&tx_bytes);
        Ok(tx_base64)
    }
}
