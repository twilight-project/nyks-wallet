use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// Network type: "testnet" or "mainnet". Controls BIP-44 coin type (1 vs 118)
/// and default endpoint URLs.
pub static NETWORK_TYPE: LazyLock<String> =
    LazyLock::new(|| std::env::var("NETWORK_TYPE").unwrap_or("mainnet".to_string()));

fn is_mainnet() -> bool {
    *NETWORK_TYPE == "mainnet"
}

pub static FAUCET_BASE_URL: LazyLock<String> = LazyLock::new(|| {
    let default = if is_mainnet() {
        "".to_string()
    } else {
        "https://faucet-rpc.twilight.rest".to_string()
    };
    std::env::var("FAUCET_BASE_URL").unwrap_or(default)
});
pub static NYKS_LCD_BASE_URL: LazyLock<String> = LazyLock::new(|| {
    let default = if is_mainnet() {
        "https://lcd.twilight.org".to_string()
    } else {
        "https://lcd.twilight.rest".to_string()
    };
    std::env::var("NYKS_LCD_BASE_URL").unwrap_or(default)
});
pub static NYKS_RPC_BASE_URL: LazyLock<String> = LazyLock::new(|| {
    let default = if is_mainnet() {
        "https://rpc.twilight.org".to_string()
    } else {
        "https://rpc.twilight.rest".to_string()
    };
    std::env::var("NYKS_RPC_BASE_URL").unwrap_or(default)
});
pub static VALIDATOR_WALLET_PATH: LazyLock<String> =
    LazyLock::new(|| std::env::var("VALIDATOR_WALLET_PATH").unwrap_or("validator.mnemonic".to_string()));
pub static RELAYER_PROGRAM_JSON_PATH: LazyLock<String> =
    LazyLock::new(|| std::env::var("RELAYER_PROGRAM_JSON_PATH").unwrap_or_else(|_| "./relayerprogram.json".to_string()));
pub static ZKOS_SERVER_URL: LazyLock<String> = LazyLock::new(|| {
    let default = if is_mainnet() {
        "https://zkserver.twilight.org".to_string()
    } else {
        "https://nykschain.twilight.rest/zkos".to_string()
    };
    std::env::var("ZKOS_SERVER_URL").unwrap_or(default)
});
pub static RELAYER_API_RPC_SERVER_URL: LazyLock<String> = LazyLock::new(|| {
    let default = if is_mainnet() {
        "https://api.ephemeral.fi/api".to_string()
    } else {
        "https://relayer.twilight.rest/api".to_string()
    };
    std::env::var("RELAYER_API_RPC_SERVER_URL").unwrap_or(default)
});
pub static CHAIN_ID: LazyLock<String> =
    LazyLock::new(|| std::env::var("CHAIN_ID").unwrap_or("nyks".to_string()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfig {
    pub validator_wallet_path: String,
    pub relayer_program_json_path: String,
    pub zkos_server_endpoint: String,
    pub relayer_api_endpoint: String,
    pub nyks_lcd_endpoint: String,
    pub nyks_rpc_endpoint: String,
    pub faucet_endpoint: String,
    pub chain_id: String,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            validator_wallet_path: VALIDATOR_WALLET_PATH.to_string(),
            relayer_program_json_path: RELAYER_PROGRAM_JSON_PATH.to_string(),
            zkos_server_endpoint: ZKOS_SERVER_URL.to_string(),
            relayer_api_endpoint: RELAYER_API_RPC_SERVER_URL.to_string(),
            nyks_lcd_endpoint: NYKS_LCD_BASE_URL.to_string(),
            nyks_rpc_endpoint: NYKS_RPC_BASE_URL.to_string(),
            faucet_endpoint: FAUCET_BASE_URL.to_string(),
            chain_id: CHAIN_ID.to_string(),
        }
    }
}

impl EndpointConfig {
    pub fn new(
        validator_wallet_path: String,
        relayer_program_json_path: String,
        zkos_server_endpoint: String,
        relayer_api_endpoint: String,
        nyks_lcd_endpoint: String,
        nyks_rpc_endpoint: String,
        faucet_endpoint: String,
        chain_id: String,
    ) -> Self {
        Self {
            validator_wallet_path,
            relayer_program_json_path,
            zkos_server_endpoint,
            relayer_api_endpoint,
            nyks_lcd_endpoint,
            nyks_rpc_endpoint,
            faucet_endpoint,
            chain_id,
        }
    }

    pub fn from_env() -> Self {
        Self::default()
    }

    pub fn to_wallet_endpoint_config(&self) -> WalletEndPointConfig {
        WalletEndPointConfig::new(
            self.nyks_lcd_endpoint.clone(),
            self.faucet_endpoint.clone(),
            self.nyks_rpc_endpoint.clone(),
            self.chain_id.clone(),
        )
    }
    pub fn to_relayer_endpoint_config(&self) -> RelayerEndPointConfig {
        RelayerEndPointConfig::new(
            self.relayer_api_endpoint.clone(),
            self.zkos_server_endpoint.clone(),
            self.relayer_program_json_path.clone(),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletEndPointConfig {
    pub lcd_endpoint: String,
    pub faucet_endpoint: String,
    pub rpc_endpoint: String,
    pub chain_id: String,
}

impl Default for WalletEndPointConfig {
    fn default() -> Self {
        Self {
            lcd_endpoint: NYKS_LCD_BASE_URL.to_string(),
            faucet_endpoint: FAUCET_BASE_URL.to_string(),
            rpc_endpoint: NYKS_RPC_BASE_URL.to_string(),
            chain_id: CHAIN_ID.to_string(),
        }
    }
}

impl WalletEndPointConfig {
    pub fn new(
        lcd_endpoint: String,
        faucet_endpoint: String,
        rpc_endpoint: String,
        chain_id: String,
    ) -> Self {
        Self {
            lcd_endpoint,
            faucet_endpoint,
            rpc_endpoint,
            chain_id,
        }
    }

    pub fn from_env() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerEndPointConfig {
    pub relayer_api_endpoint: String,
    pub zkos_server_endpoint: String,
    pub relayer_program_json_path: String,
}

impl Default for RelayerEndPointConfig {
    fn default() -> Self {
        Self {
            relayer_api_endpoint: RELAYER_API_RPC_SERVER_URL.to_string(),
            zkos_server_endpoint: ZKOS_SERVER_URL.to_string(),
            relayer_program_json_path: RELAYER_PROGRAM_JSON_PATH.to_string(),
        }
    }
}

impl RelayerEndPointConfig {
    pub fn new(
        relayer_api_endpoint: String,
        zkos_server_endpoint: String,
        relayer_program_json_path: String,
    ) -> Self {
        Self {
            relayer_api_endpoint,
            zkos_server_endpoint,
            relayer_program_json_path,
        }
    }

    pub fn from_env() -> Self {
        Self::default()
    }
}
