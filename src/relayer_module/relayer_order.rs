use curve25519_dalek::scalar::Scalar;
use twilight_client_sdk::{
    programcontroller::ContractManager,
    quisquislib::RistrettoSecretKey,
    relayer_rpcclient::method::GetCreateTraderOrderResponse,
    relayer_types::{CreateTraderOrder, PositionType},
    zkvm::{Input, Output},
};

use crate::Wallet;

pub fn create_trader_order(
    wallet: &Wallet,
    input_coin: Input,
    sk: RistrettoSecretKey,
    rscalar: Scalar,
    value: u64,
    order_side: PositionType,
    leverage: u64,
    entry_price: u64,
    position_value: u64,
    position_size: u64,
    programs: &ContractManager,
    address: String,
    output_memo: Output,
) -> Result<String, String> {
    let input_coin = super::get_transaction_coin_input_from_address_fast(address)?;
    let order_tx_message = twilight_client_sdk::relayer::create_trader_order_zkos(
        input_coin,
        output_memo,
        sk,
        rscalar,
        value,
        address,
        order_side.to_str(),
        "MARKET".to_string(),
        leverage,
        value as f64,
        value as f64,
        "PENDING".to_string(),
        entry_price as f64,
        35000.0,
    )
    .map_err(|e| e.to_string())?;
    Ok("".to_string())
}
pub fn place_random_market_trader_order(
    sk: RistrettoSecretKey,
    accountdb: crate::models::AccountDB,
    entry_price: u64,
) -> Result<GetCreateTraderOrderResponse, String> {
    //fetch input account from the address
    let value = accountdb.balance as u64;
    let rscalar = twilight_client_sdk::util::hex_to_scalar(accountdb.scalar_str.unwrap()).unwrap();
    let coin_address = accountdb.pk_address;
    let input_coin = twilight_client_sdk::chain::get_transaction_coin_input_from_address_fast(
        coin_address.clone(),
    )?;
    let (leverage, order_side) = helper_random_values();
    let position_value = value * leverage as u64;
    let position_size = position_value * entry_price;
    let contract_path = std::env::var("RELAYER_PROGRAM_JSON_PATH")
        .unwrap_or_else(|_| "./relayerprogram.json".to_string());
    let programs =
        twilight_client_sdk::programcontroller::ContractManager::import_program(&contract_path);

    let order_tx_message = twilight_client_sdk::relayer::create_trader_order_zkos(
        input_coin,
        sk,
        rscalar,
        value,
        order_side.to_str(),
        "MARKET".to_string(),
        leverage,
        value as f64,
        value as f64,
        "PENDING".to_string(),
        entry_price as f64,
        35000.0,
        position_value,
        position_size,
        order_side.clone(),
        &programs,
        0u32,
    )
    .map_err(|e| e.to_string())?;
    // send to chain
    let response =
        crate::relayer_types::CreateTraderOrderZkos::submit_order(order_tx_message.clone())?;

    Ok(response)
}
