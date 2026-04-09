use curve25519_dalek::scalar::Scalar;
use twilight_client_sdk::{
    chain::get_transaction_coin_input_from_address_fast,
    programcontroller::ContractManager,
    quisquislib::RistrettoSecretKey,
    relayer::{
        cancel_trader_order_zkos, cancel_trader_order_zkos_sltp, create_lend_order_zkos,
        execute_order_zkos, execute_order_zkos_sltp,
    },
    relayer_types::{
        CancelTraderOrderZkos, CancelTraderOrderZkosSlTp, CreateLendOrderZkos,
        CreateTraderOrderClientZkos, ExecuteLendOrderZkos, ExecuteTraderOrderZkos,
        ExecuteTraderOrderZkosSlTp, OrderStatus, OrderType, PositionType, SlTpOrder,
        SlTpOrderCancel, TXType,
    },
    util::create_output_memo_for_lender,
    zkvm::Output,
};
use uuid::Uuid;

use crate::relayer_module::relayer_api::RelayerJsonRpcClient;

const DEFAULT_RELAYER_PROGRAM_JSON: &str = r#"{
  "program_index": {
    "LiquidateOrder": 5,
    "SettleTraderOrderNegativeMarginDifference": 6,
    "SettleTraderOrder": 2,
    "SettleLendOrder": 4,
    "CreateLendOrder": 3,
    "RelayerInitializer": 0,
    "CreateTraderOrder": 1
  },
  "program": [
    "060a0402000000060a0e0401000000060a0402000000060a0e1013",
    "060a0403000000060a0405000000060a0d0e13020202",
    "040300000002040300000002040a0000000603000000000a0b04070000000603000000000a04020000000c04020000000a0b04020000000a0c0404000000060a0b0c0302000000050d0307000000050d0407000000050403000000050b0c0406000000050d0407000000050d0403000000050c0e04010000000b0403000000060a0c0402000000060a0e101302",
    "0401000000060a0302000000060a0306000000060a0c0e0403000000060a0304000000060a0307000000060a0c0e100401000000050402000000060a0405000000060a0d0c0402000000060a0403000000060a0d0e1013",
    "050304000000060a0307000000060a0d0c0302000000060a0306000000060a0d0e0406000000060a0b0403000000060a0c0402000000060a0e100401000000060a0402000000060a0403000000060a0b0c0e101302",
    "0202020202060a0401000000060a0407000000060a0c0e130202020202",
    "040300000002040300000002040a0000000603000000000a0b04070000000603000000000a04020000000c04020000000a0b04020000000a0c0404000000060a0c0302000000050d0307000000050d0407000000050403000000050b0c0406000000050d0407000000050d0403000000050c0e04010000000b0403000000060a0c0402000000060a0e101302"
  ]
}"#;

fn load_programs(contract_path: &str) -> ContractManager {
    if std::path::Path::new(contract_path).exists() {
        ContractManager::import_program(contract_path)
    } else {
        serde_json::from_str(DEFAULT_RELAYER_PROGRAM_JSON)
            .expect("hardcoded relayer program JSON is valid")
    }
}

pub async fn create_trader_order(
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
    relayer_api_client: &RelayerJsonRpcClient,
) -> Result<String, String> {
    let programs = load_programs(&contract_path);
    let input_coin =
        tokio::task::spawn_blocking(move || get_transaction_coin_input_from_address_fast(address))
            .await
            .map_err(|e| e.to_string())?;
    let input_coin = input_coin.map_err(|e| e.to_string())?;
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
    let order_data = CreateTraderOrderClientZkos::decode_from_hex_string(order_tx_message.clone())?;
    let _verified = order_data.tx.verify()?;
    let response = relayer_api_client
        .submit_trade_order(order_data)
        .await
        .map_err(|e| e.to_string())?;
    Ok(response.id_key.to_string())
}

pub async fn close_trader_order_internal(
    output_memo: Output, // Provides the Prover Memo Output used to create the order. Input memo will be created by Exchange on behalf of the user
    secret_key: &RistrettoSecretKey,
    account_id: String,
    uuid: Uuid,
    order_type: OrderType,
    execution_price: f64,
    relayer_api_client: &RelayerJsonRpcClient,
) -> Result<String, String> {
    let request_msg = execute_order_zkos(
        output_memo,
        secret_key,
        account_id,
        uuid,
        order_type.to_str(),
        0.0,
        OrderStatus::FILLED.to_str(),
        execution_price,
        TXType::ORDERTX,
    );
    let response = relayer_api_client
        .settle_trade_order(ExecuteTraderOrderZkos::decode_from_hex_string(
            request_msg.clone(),
        )?)
        .await
        .map_err(|e| e.to_string())?;
    Ok(response.id_key.to_string())
}
pub async fn close_trader_order_sltp_internal(
    output_memo: Output, // Provides the Prover Memo Output used to create the order. Input memo will be created by Exchange on behalf of the user
    secret_key: &RistrettoSecretKey,
    account_id: String,
    uuid: Uuid,
    order_type: OrderType,
    execution_price: f64,
    stop_loss_price: Option<f64>,
    take_profit_price: Option<f64>,
    relayer_api_client: &RelayerJsonRpcClient,
) -> Result<String, String> {
    let request_msg = execute_order_zkos_sltp(
        output_memo,
        secret_key,
        account_id,
        uuid,
        order_type.to_str(),
        0.0,
        OrderStatus::FILLED.to_str(),
        execution_price,
        TXType::ORDERTX,
        Some(SlTpOrder::new(stop_loss_price, take_profit_price)),
    );
    let response = relayer_api_client
        .settle_trade_order_sltp(ExecuteTraderOrderZkosSlTp::decode_from_hex_string(
            request_msg.clone(),
        )?)
        .await
        .map_err(|e| e.to_string())?;
    // println!("request_msg: {}", request_msg);
    // Ok("".to_string())
    Ok(response.id_key.to_string())
}

pub async fn close_lend_order(
    output_memo: Output, // Provides the Prover Memo Output used to create the order. Input memo will be created by Exchange on behalf of the user
    secret_key: &RistrettoSecretKey,
    account_id: String,
    uuid: Uuid,
    order_type: OrderType,
    relayer_api_client: &RelayerJsonRpcClient,
) -> Result<String, String> {
    let request_msg = execute_order_zkos(
        output_memo,
        secret_key,
        account_id,
        uuid,
        order_type.to_str(),
        0.0,
        OrderStatus::FILLED.to_str(),
        0.0,
        TXType::LENDTX,
    );
    let response = relayer_api_client
        .settle_lend_order(ExecuteLendOrderZkos::decode_from_hex_string(
            request_msg.clone(),
        )?)
        .await
        .map_err(|e| e.to_string())?;
    Ok(response.id_key.to_string())
}

pub async fn create_lend_order(
    account_address: String,
    secret_key: RistrettoSecretKey,
    amount: u64,
    contract_path: &str,
    scalar_hex: String,
    relayer_api_client: &RelayerJsonRpcClient,
) -> Result<String, String> {
    let account_address_clone = account_address.clone();
    let input_coin = tokio::task::spawn_blocking(move || {
        get_transaction_coin_input_from_address_fast(account_address.clone())
    })
    .await
    .map_err(|e| e.to_string())?;
    let input_coin = input_coin.map_err(|e| e.to_string())?;
    let programs = load_programs(&contract_path);
    let script_address =
        programs.create_contract_address(twilight_client_sdk::address::Network::default())?;
    let output_memo_scalar = twilight_client_sdk::util::hex_to_scalar(scalar_hex.clone())
        .ok_or("Failed to convert scalar hex to scalar")?;
    let output_memo = create_output_memo_for_lender(
        script_address,
        account_address_clone.clone(),
        amount,
        0,
        output_memo_scalar,
        0,
    );
    let request_msg = create_lend_order_zkos(
        input_coin,
        output_memo,
        secret_key,
        scalar_hex,
        amount,
        account_address_clone,
        amount as f64,
        OrderType::LEND.to_str(),
        OrderStatus::PENDING.to_str(),
        amount as f64,
    );
    let response = relayer_api_client
        .submit_lend_order(CreateLendOrderZkos::decode_from_hex_string(request_msg?)?)
        .await
        .map_err(|e| e.to_string())?;
    Ok(response.id_key.to_string())
}

pub async fn cancel_trader_order(
    account_address: String,
    secret_key: &RistrettoSecretKey,
    account_id: String,
    uuid: Uuid,
    relayer_api_client: &RelayerJsonRpcClient,
) -> Result<String, String> {
    let request_msg = cancel_trader_order_zkos(
        account_address,
        secret_key,
        account_id,
        uuid,
        OrderType::LIMIT.to_str(),
        OrderStatus::CANCELLED.to_str(),
    );
    let response = relayer_api_client
        .cancel_trader_order(CancelTraderOrderZkos::decode_from_hex_string(
            request_msg.clone(),
        )?)
        .await
        .map_err(|e| e.to_string())?;
    Ok(response.id_key.to_string())
}

pub async fn cancel_trader_order_sltp(
    account_address: String,
    secret_key: &RistrettoSecretKey,
    account_id: String,
    uuid: Uuid,
    sltp_cancel: SlTpOrderCancel,
    relayer_api_client: &RelayerJsonRpcClient,
) -> Result<String, String> {
    let request_msg = cancel_trader_order_zkos_sltp(
        account_address,
        secret_key,
        account_id,
        uuid,
        OrderType::SLTP.to_str(),
        OrderStatus::CANCELLED.to_str(),
        sltp_cancel,
    );
    let response = relayer_api_client
        .cancel_trader_order_sltp(CancelTraderOrderZkosSlTp::decode_from_hex_string(
            request_msg.clone(),
        )?)
        .await
        .map_err(|e| e.to_string())?;
    Ok(response.id_key.to_string())
}
