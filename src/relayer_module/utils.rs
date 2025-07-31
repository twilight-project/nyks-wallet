use twilight_client_sdk::{
    chain::get_utxo_details_by_address,
    zkvm::{self, IOType, Input},
};

pub fn get_transaction_coin_input_from_address_fast(address_hex: String) -> Result<Input, String> {
    let coin_utxo_result = get_utxo_details_by_address(address_hex, IOType::Coin);
    match coin_utxo_result {
        Ok(utxo_detail_response) => {
            let out_coin = match utxo_detail_response.output.as_out_coin() {
                Some(coin) => coin.clone(),
                None => return Err("Invalid Output:: Not a Coin Output")?,
            };
            let inp = Input::coin(zkvm::InputData::coin(
                utxo_detail_response.id.clone(),
                out_coin,
                0,
            ));
            Ok(inp)
        }
        Err(arg) => {
            Err(format!("GetUtxoDetailError in transaction_coin_input fn: {:?}", arg).into())
        }
    }
}
