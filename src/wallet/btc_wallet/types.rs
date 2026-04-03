use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtcReserve {
    pub reserve_id: u64,
    pub reserve_address: String,
    pub judge_address: String,
    pub btc_relay_capacity_value: u64,
    pub total_value: u64,
    pub private_pool_value: u64,
    pub public_value: u64,
    pub fee_pool: u64,
    pub unlock_height: u64,
    pub round_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtcDepositInfo {
    pub btc_deposit_address: String,
    pub twilight_address: String,
    pub is_confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtcDepositDetail {
    pub btc_deposit_address: String,
    pub btc_satoshi_amount: u64,
    pub twilight_staking_amount: u64,
    pub twilight_address: String,
    pub is_confirmed: bool,
    pub creation_block_height: i64,
}

/// On-chain BTC withdrawal request status from LCD.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtcWithdrawStatus {
    pub withdraw_identifier: u64,
    pub withdraw_address: String,
    pub withdraw_reserve_id: String,
    pub withdraw_amount: String,
    pub twilight_address: String,
    pub is_confirmed: bool,
    pub creation_twilight_block_height: String,
}
