use nyks_wallet::relayer_module::order_wallet::OrderWallet;

use crate::commands::ZkaccountCmd;
use crate::helpers::get_or_resolve_wallet;

pub(crate) async fn handle_zkaccount(
    cmd: ZkaccountCmd,
    repl_wallet: Option<&mut OrderWallet>,
) -> Result<(), String> {
    match cmd {
        ZkaccountCmd::Fund {
            amount,
            amount_mbtc,
            amount_btc,
            wallet_id,
            password,
        } => {
            let provided = [
                amount.is_some(),
                amount_mbtc.is_some(),
                amount_btc.is_some(),
            ]
            .iter()
            .filter(|&&v| v)
            .count();

            if provided == 0 {
                return Err("No amount specified. Provide one of:\n  \
                     --amount <sats>        Amount in satoshis\n  \
                     --amount-mbtc <mbtc>   Amount in milli-BTC (1 mBTC = 100,000 sats)\n  \
                     --amount-btc <btc>     Amount in BTC (1 BTC = 100,000,000 sats)"
                    .to_string());
            }
            if provided > 1 {
                eprintln!(
                    "Warning: Multiple amount flags provided. Using priority: --amount > --amount-mbtc > --amount-btc"
                );
            }

            let amount_sats: u64 = if let Some(sats) = amount {
                sats
            } else if let Some(mbtc) = amount_mbtc {
                (mbtc * 100_000.0).round() as u64
            } else if let Some(btc) = amount_btc {
                (btc * 100_000_000.0).round() as u64
            } else {
                unreachable!()
            };

            if amount_sats == 0 {
                return Err("Amount must be greater than 0".to_string());
            }

            let mut ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

            println!("Funding {amount_sats} sats to new ZkOS trading account...");
            let (tx_result, account_index) = ow.funding_to_trading(amount_sats).await?;
            println!("Funding successful");
            println!("  TX hash: {}", tx_result.tx_hash);
            println!("  TX code: {}", tx_result.code);
            println!("  Account index: {account_index}");
            Ok(())
        }

        ZkaccountCmd::Withdraw {
            account_index,
            wallet_id,
            password,
        } => {
            let mut ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

            println!("Withdrawing from ZkOS account {account_index} back to on-chain wallet...");
            ow.trading_to_funding(account_index).await?;
            println!("Withdrawal successful");
            Ok(())
        }

        ZkaccountCmd::Transfer {
            account_index,
            wallet_id,
            password,
        } => {
            let mut ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

            println!("Transferring from ZkOS account {account_index} to new account...");
            let new_index = ow.trading_to_trading(account_index).await?;
            println!("Transfer successful");
            println!("  New account index: {new_index}");
            Ok(())
        }

        ZkaccountCmd::Split {
            account_index,
            balances,
            balances_mbtc,
            balances_btc,
            wallet_id,
            password,
        } => {
            let provided = [
                balances.is_some(),
                balances_mbtc.is_some(),
                balances_btc.is_some(),
            ]
            .iter()
            .filter(|&&v| v)
            .count();

            if provided == 0 {
                return Err(
                    "No balances specified. Provide one of:\n  \
                     --balances <sats>          Comma-separated balances in satoshis\n  \
                     --balances-mbtc <mbtc>     Comma-separated balances in milli-BTC (1 mBTC = 100,000 sats)\n  \
                     --balances-btc <btc>       Comma-separated balances in BTC (1 BTC = 100,000,000 sats)"
                        .to_string(),
                );
            }
            if provided > 1 {
                eprintln!(
                    "Warning: Multiple balance flags provided. Using priority: --balances > --balances-mbtc > --balances-btc"
                );
            }

            let balance_vec: Vec<u64> = if let Some(ref b) = balances {
                b.split(',')
                    .map(|s| {
                        s.trim()
                            .parse::<u64>()
                            .map_err(|e| format!("Invalid balance '{}': {}", s.trim(), e))
                    })
                    .collect::<Result<Vec<u64>, String>>()?
            } else if let Some(ref b) = balances_mbtc {
                b.split(',')
                    .map(|s| {
                        s.trim()
                            .parse::<f64>()
                            .map(|v| (v * 100_000.0).round() as u64)
                            .map_err(|e| format!("Invalid mBTC balance '{}': {}", s.trim(), e))
                    })
                    .collect::<Result<Vec<u64>, String>>()?
            } else if let Some(ref b) = balances_btc {
                b.split(',')
                    .map(|s| {
                        s.trim()
                            .parse::<f64>()
                            .map(|v| (v * 100_000_000.0).round() as u64)
                            .map_err(|e| format!("Invalid BTC balance '{}': {}", s.trim(), e))
                    })
                    .collect::<Result<Vec<u64>, String>>()?
            } else {
                unreachable!()
            };

            if balance_vec.is_empty() {
                return Err("At least one balance is required".into());
            }
            if balance_vec.iter().any(|&b| b == 0) {
                return Err("All balances must be greater than 0".to_string());
            }

            let mut ow = get_or_resolve_wallet(repl_wallet, wallet_id, password).await?;

            let total: u64 = balance_vec.iter().sum();
            println!(
                "Splitting ZkOS account {} into {} accounts (total: {} sats)...",
                account_index,
                balance_vec.len(),
                total
            );
            let results = ow
                .trading_to_trading_multiple_accounts(account_index, balance_vec)
                .await?;
            println!("Split successful");
            for (idx, bal) in &results {
                println!("  Account {}: {} sats", idx, bal);
            }
            Ok(())
        }
    }
}
