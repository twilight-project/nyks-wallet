use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use cosmrs::crypto::secp256k1::SigningKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// ----- MsgSignData (ADR-036) -------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgSignDataValue {
    signer: String,
    /// base64-encoded arbitrary bytes
    data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgSignData {
    #[serde(rename = "type")]
    msg_type: String, // e.g. "sign/MsgSignData"
    value: MsgSignDataValue,
}

/// ----- StdSignDoc (legacy Amino) ---------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdFeeAmount {
    denom: String,
    amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdFee {
    gas: String,
    amount: Vec<StdFeeAmount>, // can be empty
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubKeyBundle {
    #[serde(rename = "type")]
    pub key_type: String,
    pub value: String,
}

impl PubKeyBundle {
    pub fn new(key_type: String, value: String) -> Self {
        Self { key_type, value }
    }
    pub fn get_value(&self) -> String {
        self.value.clone()
    }
    pub fn get_key_type(&self) -> String {
        self.key_type.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureBundle {
    pub address: String,
    pub key: PubKeyBundle,
    pub signature: String,
}
impl SignatureBundle {
    pub fn new(address: String, key: PubKeyBundle, signature: String) -> Self {
        Self {
            address,
            key,
            signature,
        }
    }
    pub fn get_signature(&self) -> String {
        self.signature.clone()
    }
    pub fn get_address(&self) -> String {
        self.address.clone()
    }
    pub fn get_key(&self) -> PubKeyBundle {
        self.key.clone()
    }
    pub fn get_signature_bytes(&self) -> Vec<u8> {
        match general_purpose::STANDARD.decode(self.signature.as_bytes()) {
            Ok(sig) => sig,
            Err(_) => return self.signature.as_bytes().to_vec(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdSignDoc {
    chain_id: String,
    account_number: String,
    sequence: String,
    fee: StdFee,
    msgs: Vec<MsgSignData>,
    memo: String,
}

/// Build the StdSignDoc for your "hello" example.
pub fn build_sign_doc(signer: &str, chain_id: &str, msg: &str) -> StdSignDoc {
    StdSignDoc {
        chain_id: chain_id.to_owned(),
        account_number: "0".into(),
        sequence: "0".into(),
        fee: StdFee {
            gas: "0".into(),
            amount: vec![], // no fee
        },
        msgs: vec![MsgSignData {
            msg_type: "sign/MsgSignData".into(),
            value: MsgSignDataValue {
                signer: signer.to_owned(),
                data: general_purpose::STANDARD.encode(msg.as_bytes()), // "aGVsbG8="
            },
        }],
        memo: "".into(),
    }
}

/// Canonical Amino JSON sign bytes = canonical JSON (sorted keys, no extra ws).
/// serde_json already produces deterministic key order when serializing structs.
pub fn sign_bytes(doc: &StdSignDoc) -> Vec<u8> {
    serde_json::to_vec(doc).expect("serialize sign doc")
}

/// Cosmos wallets sign the SHA-256 of the sign bytes with secp256k1.
/// Using cosmrs SigningKey -> k256 Signer.
pub fn sign_adr036(doc: &StdSignDoc, sk: &SigningKey) -> Result<Vec<u8>, String> {
    let bytes = sign_bytes(doc);
    let digest = Sha256::digest(&bytes);

    // k256 signature (DER) vs raw 64? Cosmos expects 64-byte r||s (not DER).
    // cosmrs SigningKey::sign_digest returns a 64-byte signature.
    let sig = match sk.sign(&digest) {
        Ok(sig) => sig,
        Err(e) => return Err(e.to_string()),
    };
    Ok(sig.to_vec()) // 64 bytes
}

/// Produce the output bundle you requested.
pub fn signature_bundle(address: &str, sk: &SigningKey, sig_bytes: &[u8]) -> SignatureBundle {
    // compressed secp256k1 pubkey (33 bytes)
    let pk = sk.public_key();
    let pk_bytes = pk.to_bytes(); // [u8;33]

    // json!({
    //   "address": address,
    //   "key": {
    //     "type": "tendermint/PubKeySecp256k1",
    //     "value": general_purpose::STANDARD.encode(pk_bytes),
    //   },
    //   "signature": general_purpose::STANDARD.encode(sig_bytes),
    // })
    SignatureBundle::new(
        address.to_string(),
        PubKeyBundle::new(
            "tendermint/PubKeySecp256k1".to_string(),
            general_purpose::STANDARD.encode(pk_bytes),
        ),
        general_purpose::STANDARD.encode(sig_bytes),
    )
}

pub fn generate_seed(
    private_key: &[u8],
    twilight_address: &str,
    sign_mgs: &str,
    chain_id: &str,
    // ) -> Result<MsgSignData, String> {
) -> Result<SignatureBundle, String> {
    let sk = match SigningKey::from_slice(&private_key) {
        Ok(sk) => sk,
        Err(e) => return Err(e.to_string()),
    };
    let doc = build_sign_doc(twilight_address, chain_id, sign_mgs);
    let sig_bytes = match sign_adr036(&doc, &sk) {
        Ok(sig_bytes) => sig_bytes,
        Err(e) => return Err(e),
    };
    Ok(signature_bundle(twilight_address, &sk, &sig_bytes))
}

#[cfg(test)]
mod tests {
    use crate::wallet::*;
    #[tokio::test]
    async fn test_seed_signer() -> anyhow::Result<()> {
        let wallet = Wallet::import_from_json("test.json")?;
        let private_key = wallet.private_key.clone();
        let twilight_address = wallet.twilightaddress.clone();
        let sign_mgs = "This signature is for deriving the master Twilight ZkOS Ristretto key. Version: 1. Do not share this signature.";
        let chain_id = "nyks";
        let seed = match generate_seed(&private_key, &twilight_address, sign_mgs, chain_id) {
            Ok(seed) => seed,
            Err(e) => return Err(anyhow::anyhow!(e)),
        };

        println!("{:?}", seed.signature);
        Ok(())
    }
}
