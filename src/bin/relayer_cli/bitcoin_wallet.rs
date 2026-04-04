#[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
use nyks_wallet::relayer_module::order_wallet::OrderWallet;
use nyks_wallet::wallet::btc_wallet::validation::validate_btc_segwit_address;

use crate::commands::BitcoinWalletCmd;

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::helpers::resolve_order_wallet;

pub(crate) async fn handle_bitcoin_wallet(cmd: BitcoinWalletCmd) -> Result<(), String> {
    match cmd {
        BitcoinWalletCmd::Balance {
            wallet_id,
            password,
            btc_address,
            btc,
            mbtc,
        } => {
            let address = if let Some(addr) = btc_address {
                addr
            } else {
                #[cfg(any(feature = "sqlite", feature = "postgresql"))]
                let ow = resolve_order_wallet(wallet_id, password).await?;
                #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
                let ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

                if ow.wallet.btc_address.is_empty() {
                    return Err("Wallet has no BTC address configured".to_string());
                }
                ow.wallet.btc_address.clone()
            };

            validate_btc_segwit_address(&address)?;

            let network = if nyks_wallet::config::is_btc_mainnet() {
                "mainnet"
            } else {
                "testnet"
            };
            println!("Querying Bitcoin balance for: {}", address);
            println!("Network: {}", network);

            let bal = nyks_wallet::wallet::wallet::fetch_btc_balance(&address)
                .await
                .map_err(|e| e.to_string())?;

            if btc {
                let confirmed = bal.confirmed_sats as f64 / 100_000_000.0;
                let unconfirmed = bal.unconfirmed_sats as f64 / 100_000_000.0;
                let total = bal.total_sats as f64 / 100_000_000.0;
                println!("\nConfirmed:   {:.8} BTC", confirmed);
                println!("Unconfirmed: {:.8} BTC", unconfirmed);
                println!("Total:       {:.8} BTC", total);
            } else if mbtc {
                let confirmed = bal.confirmed_sats as f64 / 100_000.0;
                let unconfirmed = bal.unconfirmed_sats as f64 / 100_000.0;
                let total = bal.total_sats as f64 / 100_000.0;
                println!("\nConfirmed:   {:.5} mBTC", confirmed);
                println!("Unconfirmed: {:.5} mBTC", unconfirmed);
                println!("Total:       {:.5} mBTC", total);
            } else {
                println!("\nConfirmed:   {} sats", bal.confirmed_sats);
                println!("Unconfirmed: {} sats", bal.unconfirmed_sats);
                println!("Total:       {} sats", bal.total_sats);
            }

            Ok(())
        }

        BitcoinWalletCmd::Transfer {
            to,
            amount,
            amount_mbtc,
            amount_btc,
            fee_rate,
            wallet_id,
            password,
        } => {
            // Resolve amount: --amount > --amount-mbtc > --amount-btc
            let provided = [
                amount.is_some(),
                amount_mbtc.is_some(),
                amount_btc.is_some(),
            ]
            .iter()
            .filter(|&&v| v)
            .count();

            if provided == 0 {
                return Err(
                    "No amount specified. Provide one of:\n  \
                     --amount <sats>          Amount in satoshis\n  \
                     --amount-mbtc <mbtc>     Amount in milli-BTC (1 mBTC = 100,000 sats)\n  \
                     --amount-btc <btc>       Amount in BTC (1 BTC = 100,000,000 sats)"
                        .to_string(),
                );
            }
            if provided > 1 {
                eprintln!(
                    "Warning: Multiple amount flags provided. Using priority: --amount > --amount-mbtc > --amount-btc"
                );
            }

            let amount_sats = if let Some(sats) = amount {
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

            // Load wallet and extract BtcWallet
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let btc_wallet = ow.wallet.btc_wallet.as_ref().ok_or_else(|| {
                "BTC wallet not available. The wallet was created from a private key, \
                 not a mnemonic. Re-create or import the wallet using a mnemonic to \
                 enable BTC transfers.\n  \
                 Path: OrderWallet -> Wallet -> btc_wallet (None)"
                    .to_string()
            })?;

            // Validate destination is native SegWit
            validate_btc_segwit_address(&to)?;

            let network = if nyks_wallet::config::is_btc_mainnet() {
                "mainnet"
            } else {
                "testnet"
            };
            println!("Bitcoin Transfer");
            println!("  From:    {}", btc_wallet.address);
            println!("  To:      {}", to);
            println!("  Amount:  {} sats", amount_sats);
            println!("  Network: {}", network);
            if let Some(rate) = fee_rate {
                println!("  Fee rate: {} sat/vB (higher = faster confirmation)", rate);
            } else {
                println!("  Fee rate: auto (estimated from current mempool)");
            }
            println!();

            let from_addr = btc_wallet.address.clone();
            let to_addr = to.clone();

            let params = nyks_wallet::wallet::btc_wallet::send::SendBtcParams {
                btc_wallet,
                destination: to,
                amount_sats: amount_sats,
                fee_rate_sat_vb: fee_rate,
            };

            let result = nyks_wallet::wallet::btc_wallet::send::send_btc(params)
                .await
                .map_err(|e| e.to_string())?;

            println!("Transaction broadcast successfully!");
            println!("  TX ID: {}", result.txid);
            println!("  Fee:   {} sats", result.fee_sats);

            let explorer = if nyks_wallet::config::is_btc_mainnet() {
                format!("https://blockstream.info/tx/{}", result.txid)
            } else {
                format!("https://blockstream.info/testnet/tx/{}", result.txid)
            };
            println!("  Explorer: {}", explorer);

            // Save transfer to DB
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            if let Some(db) = ow.get_db_manager() {
                let record = nyks_wallet::database::models::NewDbBtcTransfer {
                    wallet_id: db.get_wallet_id().to_string(),
                    network_type: nyks_wallet::config::BTC_NETWORK_TYPE.clone(),
                    from_address: from_addr,
                    to_address: to_addr,
                    amount: amount_sats as i64,
                    fee: result.fee_sats as i64,
                    tx_id: Some(result.txid.clone()),
                    status: "broadcast".to_string(),
                    confirmations: 0,
                    created_at: chrono::Utc::now().naive_utc(),
                    updated_at: chrono::Utc::now().naive_utc(),
                };
                if let Err(e) = db.save_btc_transfer(record) {
                    eprintln!("Warning: Failed to save transfer to DB: {}", e);
                }
            }

            Ok(())
        }

        BitcoinWalletCmd::Receive {
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let network = if nyks_wallet::config::is_btc_mainnet() {
                "mainnet"
            } else {
                "testnet"
            };
            let address_type = if ow.wallet.btc_address.starts_with("bc1q")
                || ow.wallet.btc_address.starts_with("tb1q")
            {
                "Native SegWit (P2WPKH)"
            } else if ow.wallet.btc_address.starts_with("bc1p")
                || ow.wallet.btc_address.starts_with("tb1p")
            {
                "Taproot (P2TR)"
            } else {
                "Unknown"
            };

            println!("Bitcoin Receive Address");
            println!("{}", "-".repeat(50));
            println!("  Address:      {}", ow.wallet.btc_address);
            println!("  Network:      {}", network);
            println!("  Address type: {}", address_type);
            println!("  Registered:   {}", ow.wallet.btc_address_registered);

            if let Some(ref _btc_wallet) = ow.wallet.btc_wallet {
                println!("  BTC wallet:   available (keys loaded)");
                println!("  Derivation:   {}", nyks_wallet::wallet::btc_wallet::BTC_DERIVATION_PATH);
            } else {
                println!("  BTC wallet:   not available (created from private key)");
            }

            println!("\nSend BTC to this address to deposit into your wallet.");
            if !ow.wallet.btc_address_registered {
                println!("Note: Register this address on-chain first with `wallet register-btc`.");
            }

            Ok(())
        }

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        BitcoinWalletCmd::History {
            wallet_id,
            password,
            status,
            limit,
        } => {
            let ow = resolve_order_wallet(wallet_id, password).await?;
            let db = ow.get_db_manager().ok_or(
                "Database not available. Rebuild with --features sqlite".to_string(),
            )?;

            let transfers = if let Some(ref s) = status {
                db.load_btc_transfers_by_status(s)?
            } else {
                db.load_btc_transfers()?
            };

            if transfers.is_empty() {
                println!("No BTC transfers found.");
                if let Some(ref s) = status {
                    println!("  (filtered by status: {})", s);
                }
                return Ok(());
            }

            let display: Vec<_> = transfers.iter().take(limit).collect();

            println!(
                "{:<5} {:<12} {:<44} {:<44} {:<12} {:<8} {:<10} {:<6} {:<20}",
                "ID", "STATUS", "FROM", "TO", "AMOUNT", "FEE", "CONFIRMS", "NET", "DATE"
            );
            println!("{}", "-".repeat(160));

            for t in &display {
                println!(
                    "{:<5} {:<12} {:<44} {:<44} {:<12} {:<8} {:<10} {:<6} {:<20}",
                    t.id.unwrap_or(0),
                    t.status,
                    t.from_address,
                    t.to_address,
                    t.amount,
                    t.fee,
                    t.confirmations,
                    t.network_type,
                    t.created_at.format("%Y-%m-%d %H:%M"),
                );
            }

            let total_sent: i64 = transfers.iter().map(|t| t.amount).sum();
            let total_fees: i64 = transfers.iter().map(|t| t.fee).sum();
            let confirmed = transfers.iter().filter(|t| t.status == "confirmed").count();
            let pending = transfers.len() - confirmed;

            println!("{}", "-".repeat(160));
            println!(
                "Total: {} transfers ({} confirmed, {} pending) | {} sats sent | {} sats fees",
                transfers.len(),
                confirmed,
                pending,
                total_sent,
                total_fees,
            );

            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        BitcoinWalletCmd::History { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),
    }
}
