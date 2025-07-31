use curve25519_dalek::scalar::Scalar;
use twilight_client_sdk::{
    chain::get_transaction_coin_input_from_address_fast,
    programcontroller::ContractManager,
    quisquislib::RistrettoSecretKey,
    relayer_types::{OrderType, PositionType},
};

pub fn create_trader_order(
    sk: RistrettoSecretKey,
    rscalar: Scalar,
    value: u64,
    order_side: PositionType,
    order_type: OrderType,
    leverage: u64,
    entry_price: u64,
    position_value: u64,
    position_size: u64,
    contract_path: &str,
    address: String,
) -> Result<String, String> {
    // let sk = wallet.signing_key().map_err(|e| e.to_string())?;
    let programs = ContractManager::import_program(&contract_path);
    let input_coin = get_transaction_coin_input_from_address_fast(address)?;
    let order_tx_message = twilight_client_sdk::relayer::create_trader_order_zkos(
        input_coin,
        sk,
        rscalar,
        value,
        order_side.to_str(),
        order_type.to_str(),
        leverage as f64,
        value as f64,
        value as f64,
        "PENDING".to_string(),
        entry_price as f64,
        entry_price as f64,
        position_value,
        position_size,
        order_side.clone(),
        &programs,
        0u32,
    )
    .map_err(|e| e.to_string())?;
    let response = twilight_client_sdk::relayer_types::CreateTraderOrderZkos::submit_order(
        order_tx_message.clone(),
    )?;

    Ok(response.id_key.to_string())
}
