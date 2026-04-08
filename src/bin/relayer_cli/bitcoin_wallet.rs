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
                return Err("No amount specified. Provide one of:\n  \
                     --amount <sats>          Amount in satoshis\n  \
                     --amount-mbtc <mbtc>     Amount in milli-BTC (1 mBTC = 100,000 sats)\n  \
                     --amount-btc <btc>       Amount in BTC (1 BTC = 100,000,000 sats)"
                    .to_string());
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

            // Build text info lines
            let mut info_lines = vec![
                "Bitcoin Receive Address".to_string(),
                "-".repeat(50),
                format!("  Address:      {}", ow.wallet.btc_address),
                format!("  Network:      {}", network),
                format!("  Address type: {}", address_type),
                format!("  Registered:   {}", ow.wallet.btc_address_registered),
            ];

            if let Some(ref _btc_wallet) = ow.wallet.btc_wallet {
                info_lines.push("  BTC wallet:   available (keys loaded)".to_string());
                info_lines.push(format!(
                    "  Derivation:   {}",
                    nyks_wallet::wallet::btc_wallet::BTC_DERIVATION_PATH
                ));
            } else {
                info_lines
                    .push("  BTC wallet:   not available (created from private key)".to_string());
            }

            // Render info + QR code (side-by-side if terminal is wide enough)
            crate::helpers::print_with_qr(&info_lines, &ow.wallet.btc_address);

            println!("\nSend BTC to this address to deposit into your wallet.");
            if !ow.wallet.btc_address_registered {
                println!("Note: Register this address on-chain first with `wallet register-btc`.");
            }

            Ok(())
        }

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        BitcoinWalletCmd::UpdateBitcoinWallet {
            wallet_id,
            password,
            mnemonic,
        } => {
            let mnemonic = match mnemonic {
                Some(m) => m,
                None => rpassword::prompt_password("Enter mnemonic phrase: ")
                    .map_err(|e| format!("Failed to read mnemonic: {}", e))?,
            };

            let mnemonic = mnemonic.trim().to_string();
            if mnemonic.is_empty() {
                return Err("Mnemonic cannot be empty".to_string());
            }

            // Derive new BTC wallet from the mnemonic
            let btc_wallet = nyks_wallet::wallet::btc_wallet::BtcWallet::from_mnemonic(&mnemonic)
                .map_err(|e| format!("Invalid mnemonic: {}", e))?;

            let new_address = btc_wallet.address.clone();

            // Load the existing wallet from DB
            let mut ow = resolve_order_wallet(wallet_id, password).await?;

            let old_address = ow.wallet.btc_address.clone();
            let twilight_address = ow.wallet.twilightaddress.clone();

            // Check if current BTC address is already registered on-chain for this twilight address
            if ow.wallet.btc_address_registered {
                return Err(format!(
                    "Cannot update BTC wallet: the current BTC address ({}) is already \
                     registered and linked to your twilight address ({}).\n\
                     A twilight address can only be linked to one BTC address.\n\
                     You can either:\n  \
                     - Send the balance to the linked BTC address\n  \
                     - Import a new wallet using `wallet import` with the desired mnemonic",
                    old_address, twilight_address
                ));
            }

            // Check if the new BTC address is already linked to another twilight address
            match ow
                .wallet
                .fetch_registered_btc_by_address(&new_address)
                .await
            {
                Ok(Some(info)) => {
                    if info.twilight_address != twilight_address {
                        return Err(format!(
                            "Cannot update BTC wallet: the new BTC address ({}) is already \
                             linked to a different twilight address ({}).\n\
                             A BTC address can only be linked to one twilight address.",
                            new_address, info.twilight_address
                        ));
                    } else {
                        ow.wallet.btc_address_registered = true;
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    eprintln!("Warning: Could not check BTC address registration ({e}). Proceeding anyway...");
                }
            }

            // Update wallet fields
            ow.wallet.btc_address = new_address.clone();
            ow.wallet.btc_wallet = Some(btc_wallet);

            // Persist to DB
            let db = ow
                .get_db_manager()
                .ok_or("Database not available. Rebuild with --features sqlite".to_string())?;
            let wallet_password = ow
                .get_wallet_password()
                .ok_or("No wallet password available for re-encryption".to_string())?;
            db.save_encrypted_wallet(&ow.wallet, wallet_password)
                .map_err(|e| format!("Failed to save updated wallet: {}", e))?;

            let network = if nyks_wallet::config::is_btc_mainnet() {
                "mainnet"
            } else {
                "testnet"
            };

            println!("Bitcoin wallet updated successfully!");
            println!("  Network:     {}", network);
            println!("  Old address: {}", old_address);
            println!("  New address: {}", new_address);

            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        BitcoinWalletCmd::UpdateBitcoinWallet { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        BitcoinWalletCmd::History {
            wallet_id,
            password,
            status,
            limit,
        } => {
            let ow = resolve_order_wallet(wallet_id, password).await?;
            let db = ow
                .get_db_manager()
                .ok_or("Database not available. Rebuild with --features sqlite".to_string())?;

            // Update confirmation status for pending/broadcast transfers
            {
                let pending = db.load_btc_transfers()?;
                let to_check: Vec<_> = pending
                    .iter()
                    .filter(|t| t.status == "broadcast" || t.status == "pending")
                    .filter(|t| t.tx_id.is_some())
                    .collect();

                if !to_check.is_empty() {
                    println!(
                        "Checking confirmation status for {} pending transfer(s)...",
                        to_check.len()
                    );
                    for t in &to_check {
                        let txid = t.tx_id.as_deref().unwrap();
                        match nyks_wallet::wallet::btc_wallet::balance::fetch_tx_confirmations(
                            txid,
                        )
                        .await
                        {
                            Ok((true, confs)) => {
                                if let Some(id) = t.id {
                                    let _ = db.update_btc_transfer_status(
                                        id,
                                        "confirmed",
                                        confs as i32,
                                    );
                                }
                            }
                            Ok((false, _)) => {
                                // Still unconfirmed — leave as is
                            }
                            Err(_) => {
                                // Network error — skip silently
                            }
                        }
                    }
                    println!();
                }
            }

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
