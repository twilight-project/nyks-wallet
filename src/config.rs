use serde::{Deserialize, Serialize};
lazy_static! {
    pub static ref FAUCET_BASE_URL: String =
        std::env::var("FAUCET_BASE_URL").unwrap_or("http://0.0.0.0:6969".to_string());
    pub static ref NYKS_LCD_BASE_URL: String =
        std::env::var("NYKS_LCD_BASE_URL").unwrap_or("http://0.0.0.0:1317".to_string());
    pub static ref NYKS_RPC_BASE_URL: String =
        std::env::var("NYKS_RPC_BASE_URL").unwrap_or("http://0.0.0.0:26657".to_string());
    pub static ref VALIDATOR_WALLET_PATH: String =
        std::env::var("VALIDATOR_WALLET_PATH").unwrap_or("validator.mnemonic".to_string());
    pub static ref RELAYER_PROGRAM_JSON_PATH: String = std::env::var("RELAYER_PROGRAM_JSON_PATH")
        .unwrap_or_else(|_| "./relayerprogram.json".to_string());
    pub static ref ZKOS_SERVER_URL: String =
        std::env::var("ZKOS_SERVER_URL").unwrap_or("http://0.0.0.0:3030".to_string());
    pub static ref RELAYER_API_RPC_SERVER_URL: String = std::env::var("RELAYER_API_RPC_SERVER_URL")
        .unwrap_or("http://0.0.0.0:8088/api".to_string());
    pub static ref CHAIN_ID: String = std::env::var("CHAIN_ID").unwrap_or("nyks".to_string());
}
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
impl EndpointConfig {
    pub fn default() -> Self {
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
        Self {
            validator_wallet_path: std::env::var("VALIDATOR_WALLET_PATH")
                .unwrap_or("validator.mnemonic".to_string()),
            relayer_program_json_path: std::env::var("RELAYER_PROGRAM_JSON_PATH")
                .unwrap_or("./relayerprogram.json".to_string()),
            zkos_server_endpoint: std::env::var("ZKOS_SERVER_URL")
                .unwrap_or("http://0.0.0.0:3030".to_string()),
            relayer_api_endpoint: std::env::var("RELAYER_API_RPC_SERVER_URL")
                .unwrap_or("http://0.0.0.0:8088/api".to_string()),
            nyks_lcd_endpoint: std::env::var("NYKS_LCD_BASE_URL")
                .unwrap_or("http://0.0.0.0:1317".to_string()),
            nyks_rpc_endpoint: std::env::var("NYKS_RPC_BASE_URL")
                .unwrap_or("http://0.0.0.0:26657".to_string()),
            faucet_endpoint: std::env::var("FAUCET_BASE_URL")
                .unwrap_or("http://0.0.0.0:6969".to_string()),
            chain_id: std::env::var("CHAIN_ID").unwrap_or("nyks".to_string()),
        }
    }
    pub fn update_validator_wallet_path(&mut self, path: String) {
        self.validator_wallet_path = path;
    }
    pub fn update_relayer_program_json_path(&mut self, path: String) {
        self.relayer_program_json_path = path;
    }
    pub fn update_zkos_server_endpoint(&mut self, endpoint: String) {
        self.zkos_server_endpoint = endpoint;
    }
    pub fn update_relayer_api_endpoint(&mut self, endpoint: String) {
        self.relayer_api_endpoint = endpoint;
    }
    pub fn update_nyks_lcd_endpoint(&mut self, endpoint: String) {
        self.nyks_lcd_endpoint = endpoint;
    }
    pub fn update_nyks_rpc_endpoint(&mut self, endpoint: String) {
        self.nyks_rpc_endpoint = endpoint;
    }
    pub fn update_faucet_endpoint(&mut self, endpoint: String) {
        self.faucet_endpoint = endpoint;
    }
    pub fn update_chain_id(&mut self, chain_id: String) {
        self.chain_id = chain_id;
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
    pub fn default() -> Self {
        Self {
            lcd_endpoint: NYKS_LCD_BASE_URL.to_string(),
            faucet_endpoint: FAUCET_BASE_URL.to_string(),
            rpc_endpoint: NYKS_RPC_BASE_URL.to_string(),
            chain_id: CHAIN_ID.to_string(),
        }
    }
    pub fn from_env() -> Self {
        Self {
            lcd_endpoint: std::env::var("NYKS_LCD_BASE_URL")
                .unwrap_or("http://0.0.0.0:1317".to_string()),
            faucet_endpoint: std::env::var("FAUCET_BASE_URL")
                .unwrap_or("http://0.0.0.0:6969".to_string()),
            rpc_endpoint: std::env::var("NYKS_RPC_BASE_URL")
                .unwrap_or("http://0.0.0.0:26657".to_string()),
            chain_id: std::env::var("CHAIN_ID").unwrap_or("nyks".to_string()),
        }
    }
    pub fn update_lcd_endpoint(&mut self, endpoint: String) {
        self.lcd_endpoint = endpoint;
    }
    pub fn update_faucet_endpoint(&mut self, endpoint: String) {
        self.faucet_endpoint = endpoint;
    }
    pub fn update_rpc_endpoint(&mut self, endpoint: String) {
        self.rpc_endpoint = endpoint;
    }
    pub fn update_chain_id(&mut self, chain_id: String) {
        self.chain_id = chain_id;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerEndPointConfig {
    pub relayer_api_endpoint: String,
    pub zkos_server_endpoint: String,
    pub relayer_program_json_path: String,
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
    pub fn default() -> Self {
        Self {
            relayer_api_endpoint: RELAYER_API_RPC_SERVER_URL.to_string(),
            zkos_server_endpoint: ZKOS_SERVER_URL.to_string(),
            relayer_program_json_path: RELAYER_PROGRAM_JSON_PATH.to_string(),
        }
    }
    pub fn from_env() -> Self {
        Self {
            relayer_api_endpoint: std::env::var("RELAYER_API_RPC_SERVER_URL")
                .unwrap_or("http://0.0.0.0:8088/api".to_string()),
            zkos_server_endpoint: std::env::var("ZKOS_SERVER_URL")
                .unwrap_or("http://0.0.0.0:3030".to_string()),
            relayer_program_json_path: std::env::var("RELAYER_PROGRAM_JSON_PATH")
                .unwrap_or("./relayerprogram.json".to_string()),
        }
    }
    pub fn update_relayer_api_endpoint(&mut self, endpoint: String) {
        self.relayer_api_endpoint = endpoint;
    }
    pub fn update_zkos_server_endpoint(&mut self, endpoint: String) {
        self.zkos_server_endpoint = endpoint;
    }
    pub fn update_relayer_program_json_path(&mut self, path: String) {
        self.relayer_program_json_path = path;
    }
}
