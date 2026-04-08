use nyks_wallet::relayer_module::order_wallet::OrderWallet;
use nyks_wallet::wallet::btc_wallet::validation::validate_btc_segwit_address;
use nyks_wallet::wallet::btc_wallet::BtcReserve;
use secrecy::{ExposeSecret, SecretString};

use crate::commands::WalletCmd;
use crate::helpers::{
    load_order_wallet_from_db, resolve_password, resolve_wallet_id, session_clear, session_load,
    session_save,
};

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::helpers::resolve_order_wallet;

/// How many blocks remain before a reserve's unlock window closes.
fn reserve_blocks_left(reserve: &BtcReserve, btc_height: u64) -> u64 {
    if btc_height > 0 && reserve.unlock_height + 144 > btc_height {
        reserve.unlock_height + 144 - btc_height
    } else {
        0
    }
}

/// Human-readable status label for a reserve given its remaining blocks.
/// Returns `None` for CRITICAL/EXPIRED reserves (blocks_left <= 4).
fn reserve_status_label(blocks_left: u64) -> Option<&'static str> {
    if blocks_left > 72 {
        Some("ACTIVE")
    } else if blocks_left > 4 {
        Some("WARNING")
    } else {
        None // CRITICAL or EXPIRED
    }
}

/// Pick the best reserve: non-critical, non-expired, with the latest expiry.
fn find_best_reserve(reserves: &[BtcReserve], btc_height: u64) -> Option<(BtcReserve, u64)> {
    reserves
        .iter()
        .filter_map(|r| {
            let bl = reserve_blocks_left(r, btc_height);
            if bl > 4 { Some((r.clone(), bl)) } else { None }
        })
        .max_by_key(|(_, bl)| *bl)
}

/// Save a BTC deposit record to the database (best-effort, logs warnings on failure).
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
fn save_deposit_record(
    ow: &OrderWallet,
    btc_addr: &str,
    tw_addr: &str,
    reserve_address: Option<String>,
    reserve_id: Option<u64>,
    amount: u64,
    staking_amount: u64,
    registration_tx_hash: Option<String>,
    btc_tx_hash: Option<String>,
    status: &str,
) {
    if let Some(db_manager) = ow.get_db_manager() {
        let now = chrono::Utc::now().naive_utc();
        let deposit_entry = nyks_wallet::database::models::NewDbBtcDeposit {
            wallet_id: db_manager.get_wallet_id().to_string(),
            network_type: nyks_wallet::config::NETWORK_TYPE.to_string(),
            btc_address: btc_addr.to_string(),
            twilight_address: tw_addr.to_string(),
            reserve_address,
            reserve_id: reserve_id.map(|id| id as i64),
            amount: amount as i64,
            staking_amount: staking_amount as i64,
            registration_tx_hash,
            btc_tx_hash,
            status: status.to_string(),
            created_at: now,
            updated_at: now,
        };
        if let Err(e) = db_manager.save_btc_deposit(deposit_entry) {
            eprintln!("Warning: Could not save deposit to database: {e}");
        }
    }
}

pub(crate) async fn handle_wallet(cmd: WalletCmd) -> Result<(), String> {
    match cmd {
        WalletCmd::Create {
            wallet_id,
            password,
            btc_address,
        } => {
            if let Some(ref addr) = btc_address {
                validate_btc_segwit_address(addr)?;
            }
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;
            if let Some(addr) = btc_address {
                ow.wallet.btc_address = addr;
                ow.wallet.btc_address_registered = false;
            }
            println!("Wallet created successfully");
            println!("  Address: {}", ow.wallet.twilightaddress);
            println!("  BTC address: {}", ow.wallet.btc_address);

            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            {
                let pwd = resolve_password(password).map(|p| SecretString::new(p.into()));
                ow.with_db(pwd, wallet_id.clone())?;
                println!(
                    "  Wallet ID: {}",
                    wallet_id.unwrap_or_else(|| ow.wallet.twilightaddress.clone())
                );
            }
            Ok(())
        }

        WalletCmd::Import {
            mnemonic,
            wallet_id,
            password,
            btc_address,
        } => {
            if let Some(ref addr) = btc_address {
                validate_btc_segwit_address(addr)?;
            }
            let mnemonic = match mnemonic {
                Some(m) => m.trim().to_string(),
                None => {
                    let m = rpassword::prompt_password("Mnemonic phrase: ")
                        .map_err(|e| e.to_string())?;
                    if m.trim().is_empty() {
                        return Err("mnemonic must not be empty".to_string());
                    }
                    m.trim().to_string()
                }
            };
            let mut ow =
                OrderWallet::import_from_mnemonic(&mnemonic, None).map_err(|e| e.to_string())?;

            // Check BTC address registration status on-chain
            if let Some(addr) = btc_address {
                // User provided a custom BTC address — check if it's already linked elsewhere
                match ow.wallet.fetch_registered_btc_by_address(&addr).await {
                    Ok(Some(info)) => {
                        if info.twilight_address != ow.wallet.twilightaddress {
                            return Err(format!(
                                "BTC address {} is already registered to a different twilight address: {}.\n\
                                 You cannot use this BTC address with your wallet ({}).",
                                addr, info.twilight_address, ow.wallet.twilightaddress
                            ));
                        }
                        // Registered to this wallet — set flag
                        ow.wallet.btc_address = addr;
                        ow.wallet.btc_address_registered = true;
                    }
                    Ok(None) => {
                        // Not registered yet — just set the address
                        ow.wallet.btc_address = addr;
                        ow.wallet.btc_address_registered = false;
                    }
                    Err(e) => {
                        eprintln!("Warning: Could not check BTC registration status: {e}");
                        ow.wallet.btc_address = addr;
                        ow.wallet.btc_address_registered = false;
                    }
                }
            } else {
                // No custom BTC address — check if the wallet's default btc_address is registered
                match ow
                    .wallet
                    .fetch_registered_btc_by_address(&ow.wallet.btc_address)
                    .await
                {
                    Ok(Some(info)) => {
                        if info.twilight_address == ow.wallet.twilightaddress {
                            ow.wallet.btc_address_registered = true;
                        } else {
                            return Err(format!(
                                "BTC address {} is already registered to a different twilight address: {}.\n\
                                 You cannot use this BTC address with your wallet ({}). pass a different BTC address with wallet create or import",
                                 &ow.wallet.btc_address, info.twilight_address, ow.wallet.twilightaddress
                            ));
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        eprintln!("Warning: Could not check BTC registration status: {e}");
                    }
                }
            }

            println!("Wallet imported successfully");
            println!("  Address: {}", ow.wallet.twilightaddress);
            println!("  BTC address: {}", ow.wallet.btc_address);
            println!(
                "  BTC registered: {}",
                if ow.wallet.btc_address_registered {
                    "yes"
                } else {
                    "no"
                }
            );

            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            {
                let pwd = resolve_password(password).map(|p| SecretString::new(p.into()));
                ow.with_db(pwd, wallet_id.clone())?;
                println!(
                    "  Wallet ID: {}",
                    wallet_id.unwrap_or_else(|| ow.wallet.twilightaddress.clone())
                );
            }

            if !ow.wallet.btc_address_registered {
                println!();
                println!("Note: If the BTC address above is not the one you use, update it with:");
                println!("  relayer-cli wallet update-btc-address --btc-address <your-bc1q-address> --wallet-id <your_wallet_id>");
            }
            println!();
            println!("Tip: To avoid typing --wallet-id and --password on every command, run:");
            println!("  relayer-cli wallet unlock");
            Ok(())
        }

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::Load {
            wallet_id,
            password,
            db_url,
        } => {
            let ow = load_order_wallet_from_db(&wallet_id, password, db_url)?;
            println!("Wallet loaded from database");
            println!("  Wallet ID: {}", wallet_id);
            println!("  Address: {}", ow.wallet.twilightaddress);
            println!("  BTC address: {}", ow.wallet.btc_address);
            println!("  Chain ID: {}", ow.chain_id);
            println!("  ZkOS accounts: {}", ow.zk_accounts.accounts.len());
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::Load { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        WalletCmd::Balance {
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let balance = ow
                .wallet
                .update_balance()
                .await
                .map_err(|e| e.to_string())?;
            println!("Wallet Balance");
            println!("  Address:  {}", ow.wallet.twilightaddress);
            println!("  NYKS:     {}", balance.nyks);
            println!("  SATS:     {}", balance.sats);
            Ok(())
        }

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::List { db_url } => {
            let wallets = OrderWallet::get_wallet_list_from_db(db_url)?;
            if wallets.is_empty() {
                println!("No wallets found in database");
            } else {
                println!("{:<40} {:<20}", "WALLET ID", "CREATED AT");
                println!("{}", "-".repeat(60));
                for w in &wallets {
                    println!("{:<40} {:<20}", w.wallet_id, w.created_at);
                }
                println!("\nTotal: {} wallet(s)", wallets.len());
            }
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::List { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        WalletCmd::Export {
            output,
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            ow.wallet
                .export_to_json(&output)
                .map_err(|e| e.to_string())?;
            println!("Wallet exported to {output}");
            Ok(())
        }

        WalletCmd::Accounts {
            wallet_id,
            password,
            on_chain_only,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            let mut accounts = ow.zk_accounts.get_all_accounts();
            accounts.sort_by_key(|a| a.index);
            if on_chain_only {
                accounts.retain(|a| a.on_chain);
            }
            if accounts.is_empty() {
                println!("No ZkOS accounts found");
            } else {
                println!(
                    "{:<8} {:<12} {:<10} {:<10} {:<10} {:<44}",
                    "INDEX", "BALANCE", "ON-CHAIN", "IO-TYPE", "TX-TYPE", "ACCOUNT"
                );
                println!("{}", "-".repeat(100));
                for acc in accounts {
                    let tx_type_str = acc
                        .tx_type
                        .as_ref()
                        .map(|t| format!("{:?}", t))
                        .unwrap_or_else(|| "-".to_string());
                    println!(
                        "{:<8} {:<12} {:<10} {:<10} {:<10} {:<44}",
                        acc.index,
                        acc.balance,
                        acc.on_chain,
                        format!("{:?}", acc.io_type),
                        tx_type_str,
                        &acc.account[..std::cmp::min(44, acc.account.len())],
                    );
                }
            }
            Ok(())
        }

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::Info {
            wallet_id,
            password,
        } => {
            let ow = resolve_order_wallet(wallet_id, password).await?;
            println!("Wallet Info");
            println!("  Address:         {}", ow.wallet.twilightaddress);
            println!("  BTC address:     {}", ow.wallet.btc_address);
            println!("  BTC registered:  {}", ow.wallet.btc_address_registered);
            println!("  Chain ID:        {}", ow.chain_id);
            println!("  ZkOS accounts:   {}", ow.zk_accounts.accounts.len());
            println!("  Next nonce:      {}", ow.nonce_manager.peek_next());
            println!("  Account number:  {}", ow.nonce_manager.account_number());
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::Info { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::Backup {
            output,
            wallet_id,
            password,
        } => {
            let ow = load_order_wallet_from_db(&wallet_id, password, None)?;
            let db_manager = ow
                .get_db_manager()
                .ok_or("Database not enabled on this wallet")?;
            db_manager.export_backup_to_file(&output)?;
            println!("Backup exported to {output}");
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::Backup { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::Restore {
            input,
            wallet_id,
            password,
            force,
        } => {
            let ow = load_order_wallet_from_db(&wallet_id, password, None)?;
            let db_manager = ow
                .get_db_manager()
                .ok_or("Database not enabled on this wallet")?;
            db_manager.import_backup_from_file(&input, force)?;
            println!("Backup restored from {input}");
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::Restore { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        WalletCmd::SyncNonce {
            wallet_id,
            password,
        } => {
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            let ow = resolve_order_wallet(wallet_id, password).await?;
            #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
            let ow = OrderWallet::new(None).map_err(|e| e.to_string())?;

            ow.sync_nonce().await?;
            println!("Nonce synced from chain");
            println!("  Next sequence: {}", ow.nonce_manager.peek_next());
            println!("  Account number: {}", ow.nonce_manager.account_number());
            println!(
                "  Released (pending reuse): {}",
                ow.nonce_manager.released_count()
            );
            Ok(())
        }

        WalletCmd::Unlock {
            wallet_id,
            password,
            force,
        } => {
            // If a session is already active, error unless --force.
            if session_load().is_some() && !force {
                eprintln!(
                    "A session is already cached. Run `wallet lock` first or use `wallet unlock --force`."
                );
                return Err("session already active".to_string());
            }

            // Resolve wallet_id: flag -> env -> interactive prompt
            let wid = if let Some(id) = wallet_id {
                id
            } else if let Ok(id) = std::env::var("NYKS_WALLET_ID") {
                println!("Using wallet from NYKS_WALLET_ID: {}", id);
                id
            } else {
                // List available wallets before prompting
                #[cfg(any(feature = "sqlite", feature = "postgresql"))]
                {
                    match OrderWallet::get_wallet_list_from_db(None) {
                        Ok(wallets) if !wallets.is_empty() => {
                            println!("Available wallets:");
                            println!("{:<40} {:<20}", "WALLET ID", "CREATED AT");
                            println!("{}", "-".repeat(60));
                            for w in &wallets {
                                println!("{:<40} {:<20}", w.wallet_id, w.created_at);
                            }
                            println!();
                        }
                        _ => {
                            println!("No wallets found in database.\n");
                        }
                    }
                }
                let mut input = String::new();
                eprint!("Wallet ID: ");
                std::io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| e.to_string())?;
                let input = input.trim().to_string();
                if input.is_empty() {
                    return Err("wallet_id must not be empty".to_string());
                }
                input
            };

            let password = if let Some(p) = password {
                p
            } else if let Ok(p) = std::env::var("NYKS_WALLET_PASSPHRASE") {
                println!("Using password from NYKS_WALLET_PASSPHRASE env var.");
                p
            } else {
                rpassword::prompt_password("Wallet password: ").map_err(|e| e.to_string())?
            };
            if password.is_empty() {
                return Err("password must not be empty".to_string());
            }

            // Verify the wallet_id + password combination before caching
            #[cfg(any(feature = "sqlite", feature = "postgresql"))]
            {
                load_order_wallet_from_db(&wid, Some(password.clone()), None)
                    .map_err(|e| format!("Failed to unlock wallet '{}': {}", wid, e))?;
            }

            session_save(&wid, &password)?;
            println!("Session cached for wallet '{}' in this terminal.", wid);
            println!("Run `wallet lock` to clear it, or just close the terminal.");
            Ok(())
        }

        WalletCmd::Lock => {
            session_clear();
            println!("Session password cleared.");
            Ok(())
        }

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::ChangePassword { wallet_id } => {
            let wid = resolve_wallet_id(wallet_id)
                .ok_or("wallet_id is required (pass --wallet-id, set NYKS_WALLET_ID env var, or run `wallet unlock`)")?;

            // Always prompt via TTY — ignore session cache and env var
            let old_password =
                rpassword::prompt_password("Current password: ").map_err(|e| e.to_string())?;
            if old_password.is_empty() {
                return Err("password must not be empty".to_string());
            }

            // Load wallet with old password to verify it's correct
            let ow = load_order_wallet_from_db(&wid, Some(old_password), None)?;

            let new_password =
                rpassword::prompt_password("New password: ").map_err(|e| e.to_string())?;
            if new_password.is_empty() {
                return Err("new password must not be empty".to_string());
            }
            let confirm_password =
                rpassword::prompt_password("Confirm new password: ").map_err(|e| e.to_string())?;
            if new_password != confirm_password {
                return Err("passwords do not match".to_string());
            }

            let db_manager = ow
                .get_db_manager()
                .ok_or("database manager not available")?;
            let new_secret = SecretString::new(new_password.into());
            db_manager.save_encrypted_wallet(&ow.wallet, &new_secret)?;

            // Update session cache if one exists
            if session_load().is_some() {
                session_save(&wid, new_secret.expose_secret())?;
            }

            println!("Password changed successfully for wallet '{}'.", wid);
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::ChangePassword { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::UpdateBtcAddress {
            btc_address,
            wallet_id,
            password,
        } => {
            validate_btc_segwit_address(&btc_address)?;

            let mut ow = resolve_order_wallet(wallet_id, password).await?;

            // Block update if already registered on-chain — each twilight address
            // can only be linked to a single BTC address.
            if ow.wallet.btc_address_registered {
                return Err(format!(
                    "Cannot update BTC address: your current address ({}) is already registered on-chain.\n\
                     Each twilight address can only be linked to a single BTC address.",
                    ow.wallet.btc_address
                ));
            }

            let old_address = ow.wallet.btc_address.clone();
            let twilight_address = ow.wallet.twilightaddress.clone();

            // Check if the new BTC address is already linked to another twilight address
            match ow
                .wallet
                .fetch_registered_btc_by_address(&btc_address)
                .await
            {
                Ok(Some(info)) => {
                    if info.twilight_address != twilight_address {
                        return Err(format!(
                            "Cannot update BTC address: the new BTC address ({}) is already \
                             linked to a different twilight address ({}).\n\
                             A BTC address can only be linked to one twilight address.",
                            btc_address, info.twilight_address
                        ));
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    eprintln!("Warning: Could not check BTC address registration ({e}). Proceeding anyway...");
                }
            }

            ow.wallet.btc_address = btc_address.clone();
            // Invalidate the old BTC wallet since the address has changed
            ow.wallet.btc_wallet = None;

            let db_manager = ow
                .get_db_manager()
                .ok_or("database manager not available")?;
            let wallet_password = ow
                .get_wallet_password()
                .ok_or("wallet password not available — cannot persist changes")?;
            db_manager.save_encrypted_wallet(&ow.wallet, wallet_password)?;

            println!("BTC address updated for wallet.");
            println!("  Old: {}", old_address);
            println!("  New: {}", btc_address);
            println!("  Note: Register on-chain with `wallet register-btc` before depositing.");
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::UpdateBtcAddress { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::Send {
            to,
            amount,
            denom,
            wallet_id,
            password,
        } => {
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            let from_addr = ow.wallet.twilightaddress.clone();

            println!("Sending {amount} {denom}");
            println!("  From: {from_addr}");
            println!("  To:   {to}");

            match ow.wallet.send_tokens(&to, amount, &denom).await {
                Ok(tx_hash) => {
                    println!("Transaction successful");
                    println!("  TX Hash: {tx_hash}");
                    Ok(())
                }
                Err(e) => Err(format!("Send failed: {e}")),
            }
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::Send { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::RegisterBtc {
            amount,
            staking_amount,
            wallet_id,
            password,
        } => {
            if nyks_wallet::config::NETWORK_TYPE.as_str() != "mainnet" {
                return Err("register-btc is only available on mainnet. Use `wallet faucet` for testnet tokens.".to_string());
            }
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            let btc_addr = ow.wallet.btc_address.clone();
            let tw_addr = ow.wallet.twilightaddress.clone();

            // 1. Check if BTC address is already registered on-chain
            println!("Checking if BTC address is already registered...");
            match ow.wallet.fetch_registered_btc_by_address(&btc_addr).await {
                Ok(Some(info)) => {
                    if info.twilight_address == tw_addr {
                        println!("BTC address {btc_addr} is already registered to your wallet ({tw_addr}).");
                        println!("Use `wallet deposit-btc --reserve-address <addr>` to record a deposit.");
                        return Ok(());
                    } else {
                        return Err(format!(
                            "BTC address {btc_addr} is already registered to another twilight address: {}\n\n\
                             If this is not your BTC address, you can:\n  \
                             - Update the BTC address:  `wallet update-btc-address --btc-address <new_address>`\n  \
                             - Update the BTC wallet:   `bitcoin-wallet update-bitcoin-wallet --mnemonic <phrase>`",
                            info.twilight_address
                        ));
                    }
                }
                Ok(None) => {
                    println!("BTC address not yet registered. Proceeding...\n");
                }
                Err(e) => {
                    use log::debug;

                    eprintln!("Warning: Could not check registration status");
                    debug!("Error: {e}");
                    println!("Proceeding with registration anyway...\n");
                }
            }

            // 2. Check reserves status — reject if all are CRITICAL or EXPIRED
            println!("Checking BTC reserve status...");
            let reserves = ow
                .wallet
                .fetch_btc_reserves()
                .await
                .map_err(|e| format!("Failed to fetch reserves: {e}"))?;

            if reserves.is_empty() {
                return Err(
                    "No BTC reserves found on chain. Cannot register — try again later."
                        .to_string(),
                );
            }

            let btc_height = nyks_wallet::wallet::wallet::fetch_btc_block_height()
                .await
                .unwrap_or(0);

            let best_reserve = find_best_reserve(&reserves, btc_height);

            if btc_height > 0 && best_reserve.is_none() {
                let any_expired = reserves
                    .iter()
                    .any(|r| reserve_blocks_left(r, btc_height) == 0);
                if any_expired {
                    let max_unlock = reserves.iter().map(|r| r.unlock_height).max().unwrap_or(0);
                    let new_reserve_at = max_unlock + 148;
                    if new_reserve_at > btc_height {
                        let blocks_until = new_reserve_at - btc_height;
                        return Err(format!(
                            "All reserves are expired or critical. A new reserve address will be \
                             available in ~{blocks_until} BTC blocks (~{} min). Try again later.",
                            blocks_until * 10
                        ));
                    }
                } else {
                    return Err(
                        "All reserves are in CRITICAL status (less than 4 blocks remaining). \
                         Wait for the next reserve rotation before registering."
                            .to_string(),
                    );
                }
            }

            // 3. Estimate fee (real estimate when btc_wallet available, 0 otherwise)
            let estimated_fee: u64 =
                if let (Some(btc_w), Some((target, _))) =
                    (&ow.wallet.btc_wallet, &best_reserve)
                {
                    println!("Estimating transaction fee...");
                    match nyks_wallet::wallet::btc_wallet::send::estimate_btc_fee(
                        &nyks_wallet::wallet::btc_wallet::send::SendBtcParams {
                            btc_wallet: btc_w,
                            destination: target.reserve_address.clone(),
                            amount_sats: amount,
                            fee_rate_sat_vb: None,
                        },
                    )
                    .await
                    {
                        Ok(fee) => {
                            println!("  Estimated fee: {fee} sats");
                            fee
                        }
                        Err(e) => {
                            eprintln!(
                                "  Warning: Could not estimate fee ({e}), using 2000 sat buffer"
                            );
                            2_000
                        }
                    }
                } else {
                    0
                };

            // 4. Check BTC balance covers amount + fee
            let required_sats = amount + estimated_fee;
            println!("Checking BTC balance for {btc_addr}...");
            let btc_balance = nyks_wallet::wallet::wallet::fetch_btc_balance(&btc_addr)
                .await
                .map_err(|e| format!("Failed to fetch BTC balance: {e}"))?;

            if btc_balance.confirmed_sats < required_sats {
                let fee_note = if estimated_fee > 0 {
                    format!(
                        "\n  (Amount: {amount} sats + estimated fee: {estimated_fee} sats = {required_sats} sats needed)"
                    )
                } else {
                    "\n\n  Note: This amount excludes BTC transaction fees. \
                     Ensure you maintain additional balance to cover network fees."
                        .to_string()
                };
                return Err(format!(
                    "Insufficient BTC balance. You need at least {required_sats} sats but your address \
                     has only {} confirmed sats.{fee_note}\n\n\
                     Please send at least {} more sats to your BTC address:\n  {btc_addr}\n\n\
                     If this is not your BTC address, you can:\n  \
                     - Update the BTC address:  `wallet update-btc-address --btc-address <new_address>`\n  \
                     - Update the BTC wallet:   `bitcoin-wallet update-bitcoin-wallet --mnemonic <phrase>`\n\n\
                     Then try again.",
                    btc_balance.confirmed_sats,
                    required_sats - btc_balance.confirmed_sats,
                ));
            }
            println!(
                "  Balance OK: {} confirmed sats (need {required_sats} sats{})\n",
                btc_balance.confirmed_sats,
                if estimated_fee > 0 {
                    " incl. estimated fee"
                } else {
                    ""
                }
            );

            // 5. Register on-chain
            println!("Registering BTC deposit address on-chain");
            println!("  Twilight address: {tw_addr}");
            println!("  BTC address:      {btc_addr}");
            println!("  Deposit amount:   {amount} sats");
            println!("  Staking amount:   {staking_amount}");

            let tx_hash = ow
                .wallet
                .register_btc_deposit(amount, staking_amount)
                .await
                .map_err(|e| format!("Registration failed: {e}"))?;

            println!("\nRegistration submitted successfully");
            println!("  TX Hash: {tx_hash}");

            // Persist the updated btc_address_registered flag
            if let Some(db_manager) = ow.get_db_manager() {
                if let Some(wallet_password) = ow.get_wallet_password() {
                    let _ = db_manager.save_encrypted_wallet(&ow.wallet, wallet_password);
                }
            }

            // 6. Pay to reserve — auto if btc_wallet available, manual otherwise
            if let (Some(btc_wallet), Some((target_reserve, blocks_left))) =
                (&ow.wallet.btc_wallet, &best_reserve)
            {
                let status = reserve_status_label(*blocks_left).unwrap_or("UNKNOWN");
                println!("\nBTC wallet detected. Auto-sending to best reserve...");
                println!(
                    "  Reserve:      {} (ID: {})",
                    target_reserve.reserve_address, target_reserve.reserve_id
                );
                println!(
                    "  Status:       {status} (~{blocks_left} blocks / ~{} min remaining)",
                    blocks_left * 10
                );
                println!("  Amount:       {amount} sats");

                match nyks_wallet::wallet::btc_wallet::send::send_btc(
                    nyks_wallet::wallet::btc_wallet::send::SendBtcParams {
                        btc_wallet,
                        destination: target_reserve.reserve_address.clone(),
                        amount_sats: amount,
                        fee_rate_sat_vb: None,
                    },
                )
                .await
                {
                    Ok(result) => {
                        println!("\nBTC payment sent successfully!");
                        println!("  TX ID: {}", result.txid);
                        println!("  Fee:   {} sats", result.fee_sats);
                        save_deposit_record(
                            &ow,
                            &btc_addr,
                            &tw_addr,
                            Some(target_reserve.reserve_address.clone()),
                            Some(target_reserve.reserve_id),
                            amount,
                            staking_amount,
                            Some(tx_hash.clone()),
                            Some(result.txid.clone()),
                            "sent",
                        );
                        // Save BTC transfer record for bitcoin-wallet history
                        if let Some(db) = ow.get_db_manager() {
                            let record = nyks_wallet::database::models::NewDbBtcTransfer {
                                wallet_id: db.get_wallet_id().to_string(),
                                network_type: nyks_wallet::config::BTC_NETWORK_TYPE.clone(),
                                from_address: btc_addr.clone(),
                                to_address: target_reserve.reserve_address.clone(),
                                amount: amount as i64,
                                fee: result.fee_sats as i64,
                                tx_id: Some(result.txid.clone()),
                                status: "broadcast".to_string(),
                                confirmations: 0,
                                created_at: chrono::Utc::now().naive_utc(),
                                updated_at: chrono::Utc::now().naive_utc(),
                            };
                            if let Err(e) = db.save_btc_transfer(record) {
                                eprintln!("Warning: Failed to save transfer to DB: {e}");
                            }
                        }
                        println!("\nCheck status with: wallet deposit-status");
                    }
                    Err(e) => {
                        eprintln!("\nFailed to auto-send BTC: {e}");
                        eprintln!("You can send manually instead:");
                        eprintln!(
                            "  Send {amount} sats from {btc_addr} to {}",
                            target_reserve.reserve_address
                        );
                        eprintln!(
                            "  Or run: wallet deposit-btc --reserve-address {}",
                            target_reserve.reserve_address
                        );
                        save_deposit_record(
                            &ow,
                            &btc_addr,
                            &tw_addr,
                            None,
                            None,
                            amount,
                            staking_amount,
                            Some(tx_hash.clone()),
                            None,
                            "registered",
                        );
                    }
                }
            } else {
                // No BTC wallet — manual flow
                println!("\nNote: BTC wallet not loaded — manual payment required.");
                println!("  To enable auto-pay, load a mnemonic:");
                println!("  bitcoin-wallet update-bitcoin-wallet --mnemonic <phrase>\n");

                // Collect active reserves for display
                let active_reserves: Vec<_> = reserves
                    .iter()
                    .filter(|r| reserve_blocks_left(r, btc_height) > 4)
                    .collect();

                // If exactly one active reserve, record it in the deposit
                let (rec_addr, rec_id) = if active_reserves.len() == 1 {
                    let r = active_reserves[0];
                    (
                        Some(r.reserve_address.clone()),
                        Some(r.reserve_id),
                    )
                } else {
                    (None, None)
                };

                save_deposit_record(
                    &ow,
                    &btc_addr,
                    &tw_addr,
                    rec_addr,
                    rec_id,
                    amount,
                    staking_amount,
                    Some(tx_hash.clone()),
                    None,
                    "registered",
                );

                println!("\nActive reserve addresses to send {amount} sats to:");
                println!(
                    "\n{:<6} {:<50} {:<15} {:<10}",
                    "ID", "RESERVE ADDRESS", "TOTAL VALUE", "STATUS"
                );
                println!("{}", "-".repeat(85));
                for r in &active_reserves {
                    let bl = reserve_blocks_left(r, btc_height);
                    let status = reserve_status_label(bl).unwrap_or("UNKNOWN");
                    println!(
                        "{:<6} {:<50} {:<15} {:<10}",
                        r.reserve_id, r.reserve_address, r.total_value, status
                    );
                }

                if active_reserves.len() == 1 {
                    let r = active_reserves[0];
                    println!("\nOnly one active reserve — send {amount} sats to:");
                    println!("  Address:    {}", r.reserve_address);
                    println!("  Reserve ID: {}", r.reserve_id);
                    println!(
                        "\n  From: {btc_addr}  (your registered BTC address)"
                    );
                    println!("  Check status with: wallet deposit-status");
                } else {
                    println!("\nNext steps:");
                    println!("  1. Pick an ACTIVE reserve address above");
                    println!("  2. Run: wallet deposit-btc --reserve-address <reserve_addr>");
                    println!("  3. Send {amount} sats from your registered BTC address ({btc_addr}) to the reserve");
                    println!("  4. Check status with: wallet deposit-status");
                }
            }
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::RegisterBtc { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::Reserves {
            wallet_id,
            password,
        } => {
            let ow = resolve_order_wallet(wallet_id, password).await?;
            match ow.wallet.fetch_btc_reserves().await {
                Ok(reserves) => {
                    if reserves.is_empty() {
                        println!("No BTC reserves found on chain.");
                    } else {
                        // Fetch current BTC block height for status calculation
                        let btc_height = nyks_wallet::wallet::wallet::fetch_btc_block_height()
                            .await
                            .unwrap_or(0);
                        if btc_height > 0 {
                            println!("Current BTC block height: {btc_height}\n");
                        }

                        println!(
                            "{:<6} {:<50} {:<15} {:<14} {:<10}",
                            "ID", "RESERVE ADDRESS", "TOTAL VALUE", "BLOCKS LEFT", "STATUS"
                        );
                        println!("{}", "-".repeat(98));
                        for r in &reserves {
                            let next_unlock = r.unlock_height + 144;
                            let (blocks_left, status) = if btc_height > 0 {
                                if next_unlock <= btc_height {
                                    ("expired".to_string(), "EXPIRED")
                                } else {
                                    let remaining = next_unlock - btc_height;
                                    let st = if remaining <= 4 {
                                        "CRITICAL"
                                    } else if remaining <= 72 {
                                        "WARNING"
                                    } else {
                                        "ACTIVE"
                                    };
                                    (remaining.to_string(), st)
                                }
                            } else {
                                (format!("unlock:{}", r.unlock_height), "UNKNOWN")
                            };
                            println!(
                                "{:<6} {:<50} {:<15} {:<14} {:<10}",
                                r.reserve_id, r.reserve_address, r.total_value, blocks_left, status
                            );
                        }
                        println!("\nTotal: {} reserve(s)", reserves.len());

                        // Check if any reserves are expired and show new-address ETA
                        if btc_height > 0 {
                            let any_expired =
                                reserves.iter().any(|r| r.unlock_height + 144 <= btc_height);
                            if any_expired {
                                // New reserve address becomes available at unlock_height + 148 (4 blocks after expiry)
                                let max_unlock =
                                    reserves.iter().map(|r| r.unlock_height).max().unwrap_or(0);
                                let new_reserve_at = max_unlock + 148;
                                if new_reserve_at > btc_height {
                                    let blocks_until = new_reserve_at - btc_height;
                                    println!("\nNote: Expired reserves are sweeping. A new reserve address will be");
                                    println!("available in ~{blocks_until} BTC blocks (~{} min) at block {new_reserve_at}.",
                                        blocks_until * 10);
                                } else {
                                    println!("\nNote: New reserve address should already be available. Re-run this command to refresh.");
                                }
                            }
                        }

                        println!("\nSTATUS KEY:");
                        println!("  ACTIVE   - Safe to send BTC");
                        println!("  WARNING  - Less than ~12h remaining, send only if your BTC tx will confirm quickly");
                        println!("  CRITICAL - Less than 4 blocks remaining, do NOT send");
                        println!("  EXPIRED  - Reserve is sweeping, do NOT send (new address available ~4 blocks after expiry)");
                        println!("\nReserve addresses rotate every ~144 BTC blocks (~24 hours).");
                        println!("The reserve must still be ACTIVE when your BTC transaction confirms on Bitcoin.");

                        // Show QR code for the best reserve address
                        if let Some((best, bl)) = find_best_reserve(&reserves, btc_height) {
                            let status = reserve_status_label(bl).unwrap_or("UNKNOWN");
                            let info_lines = vec![
                                String::new(),
                                "Recommended reserve (latest expiry):".to_string(),
                                format!("  Address: {}", best.reserve_address),
                                format!("  ID:      {}", best.reserve_id),
                                format!(
                                    "  Status:  {status} (~{bl} blocks / ~{} min remaining)",
                                    bl * 10
                                ),
                            ];
                            crate::helpers::print_with_qr(&info_lines, &best.reserve_address);
                        }
                    }
                    Ok(())
                }
                Err(e) => Err(format!("Failed to fetch reserves: {e}")),
            }
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::Reserves { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::DepositStatus {
            wallet_id,
            password,
        } => {
            if nyks_wallet::config::NETWORK_TYPE.as_str() != "mainnet" {
                return Err(
                    "deposit-status is only available on mainnet. Testnet uses faucet for tokens."
                        .to_string(),
                );
            }
            let ow = resolve_order_wallet(wallet_id, password).await?;
            let tw_addr = ow.wallet.twilightaddress.clone();

            println!("Checking deposit & withdrawal status for {tw_addr}...\n");

            // ---- Section 1: On-chain / Indexer data (confirmed transactions) ----
            match ow.wallet.fetch_account_from_indexer().await {
                Ok(info) => {
                    println!("Account: {}", info.address);
                    println!("  Transactions: {}", info.tx_count);
                    if !info.first_seen.is_empty() {
                        println!(
                            "  First seen:   {}",
                            &info.first_seen[..std::cmp::min(19, info.first_seen.len())]
                        );
                    }
                    if !info.last_seen.is_empty() {
                        println!(
                            "  Last seen:    {}",
                            &info.last_seen[..std::cmp::min(19, info.last_seen.len())]
                        );
                    }
                    println!();

                    if !info.balances.is_empty() {
                        println!("Balances:");
                        for b in &info.balances {
                            println!("  {}: {}", b.denom, b.amount);
                        }
                        println!();
                    }

                    // Confirmed Deposits (from indexer)
                    if !info.deposits.is_empty() {
                        println!("Confirmed Deposits ({}):", info.deposits.len());
                        println!(
                            "  {:<6} {:<12} {:<12} {:<10} {:<8} {:<22}",
                            "ID", "AMOUNT", "BTC HEIGHT", "CONFIRMED", "VOTES", "DATE"
                        );
                        println!("  {}", "-".repeat(72));
                        for d in &info.deposits {
                            let date = if d.created_at.len() >= 19 {
                                &d.created_at[..19]
                            } else {
                                &d.created_at
                            };
                            println!(
                                "  {:<6} {:<12} {:<12} {:<10} {:<8} {:<22}",
                                d.id,
                                d.deposit_amount,
                                d.btc_height,
                                if d.confirmed { "YES" } else { "NO" },
                                d.votes,
                                date
                            );
                        }

                        let confirmed = info.deposits.iter().filter(|d| d.confirmed).count();
                        let pending_on_chain = info.deposits.len() - confirmed;
                        let total_deposited: u64 = info
                            .deposits
                            .iter()
                            .filter(|d| d.confirmed)
                            .filter_map(|d| d.deposit_amount.parse::<u64>().ok())
                            .sum();
                        println!("\n  Total confirmed deposits: {total_deposited} sats ({confirmed} confirmed, {pending_on_chain} pending on-chain)");
                    }

                    // Confirmed Withdrawals (from indexer)
                    if !info.withdrawals.is_empty() {
                        println!("\nConfirmed Withdrawals ({}):", info.withdrawals.len());
                        println!(
                            "  {:<6} {:<50} {:<12} {:<10} {:<22}",
                            "ID", "BTC ADDRESS", "AMOUNT", "CONFIRMED", "DATE"
                        );
                        println!("  {}", "-".repeat(102));
                        for w in &info.withdrawals {
                            let date = if w.created_at.len() >= 19 {
                                &w.created_at[..19]
                            } else {
                                &w.created_at
                            };
                            println!(
                                "  {:<6} {:<50} {:<12} {:<10} {:<22}",
                                w.withdraw_identifier,
                                w.withdraw_address,
                                w.withdraw_amount,
                                if w.is_confirmed { "YES" } else { "NO" },
                                date
                            );
                        }

                        let w_confirmed =
                            info.withdrawals.iter().filter(|w| w.is_confirmed).count();
                        let w_pending = info.withdrawals.len() - w_confirmed;
                        let total_withdrawn: u64 = info
                            .withdrawals
                            .iter()
                            .filter(|w| w.is_confirmed)
                            .filter_map(|w| w.withdraw_amount.parse::<u64>().ok())
                            .sum();
                        println!("\n  Total confirmed withdrawals: {total_withdrawn} sats ({w_confirmed} confirmed, {w_pending} pending)");
                    }

                    // Update local DB: mark deposits as confirmed if they appear on the indexer.
                    // Match by btc_tx_hash (Bitcoin TX ID) when available for exact matching.
                    // Fall back to amount-based matching for older records without a tx hash.
                    if let Some(db_manager) = ow.get_db_manager() {
                        let local_deposits = db_manager.load_btc_deposits().unwrap_or_default();

                        // Build a set of confirmed BTC tx hashes from the indexer
                        let confirmed_btc_hashes: std::collections::HashSet<String> = info
                            .deposits
                            .iter()
                            .filter(|d| d.confirmed && !d.btc_hash.is_empty())
                            .map(|d| d.btc_hash.to_lowercase())
                            .collect();

                        for dep in &local_deposits {
                            if dep.status == "confirmed" {
                                continue;
                            }
                            let should_confirm =
                                if let Some(ref local_hash) = dep.btc_tx_hash {
                                    // Exact match by Bitcoin TX hash
                                    confirmed_btc_hashes.contains(&local_hash.to_lowercase())
                                } else {
                                    false
                                };
                            if should_confirm {
                                if let Some(id) = dep.id {
                                    let _ =
                                        db_manager.update_btc_deposit_status(id, "confirmed");
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Could not fetch from indexer: {e}");
                    println!("Showing local database records only.\n");
                }
            }

            // ---- Section 2: Local DB pending deposits (not yet on indexer) ----
            if let Some(db_manager) = ow.get_db_manager() {
                let local_deposits = db_manager.load_btc_deposits().unwrap_or_default();
                let pending_deposits: Vec<_> = local_deposits
                    .iter()
                    .filter(|d| d.status != "confirmed")
                    .collect();

                if !pending_deposits.is_empty() {
                    println!(
                        "\nPending Deposits — local (not yet confirmed on-chain) ({}):",
                        pending_deposits.len()
                    );
                    println!(
                        "  {:<4} {:<50} {:<12} {:<50} {:<10} {:<20}",
                        "ID", "BTC ADDRESS", "AMOUNT", "RESERVE ADDRESS", "STATUS", "DATE"
                    );
                    println!("  {}", "-".repeat(148));
                    for d in &pending_deposits {
                        let date = d.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
                        let reserve = d.reserve_address.as_deref().unwrap_or("-");
                        let status_display = match d.status.as_str() {
                            "registered" => "REGISTERED",
                            "pending" => "PENDING",
                            "sent" => "SENT",
                            other => other,
                        };
                        println!(
                            "  {:<4} {:<50} {:<12} {:<50} {:<10} {:<20}",
                            d.id.unwrap_or(0),
                            d.btc_address,
                            d.amount,
                            reserve,
                            status_display,
                            date
                        );
                    }
                    let total_pending: i64 = pending_deposits.iter().map(|d| d.amount).sum();
                    println!(
                        "\n  Total pending: {total_pending} sats ({} deposit(s))",
                        pending_deposits.len()
                    );
                    println!("\n  Pending deposits require:");
                    println!("    1. BTC sent to an active reserve address (run: wallet reserves)");
                    println!("    2. BTC transaction confirmed on Bitcoin (~10 min)");
                    println!("    3. Validator detection and confirmation (can take 1+ hours)");
                } else if local_deposits.is_empty() {
                    println!("\nNo deposit records in local database.");
                    println!("Register with: wallet register-btc --amount <sats>");
                }

                // Local DB pending withdrawals
                let local_withdrawals = db_manager.load_btc_withdrawals().unwrap_or_default();
                let pending_withdrawals: Vec<_> = local_withdrawals
                    .iter()
                    .filter(|w| w.status != "confirmed")
                    .collect();

                if !pending_withdrawals.is_empty() {
                    println!(
                        "\nPending Withdrawals — local (not yet confirmed on-chain) ({}):",
                        pending_withdrawals.len()
                    );
                    println!(
                        "  {:<4} {:<50} {:<8} {:<12} {:<10} {:<20}",
                        "ID", "BTC ADDRESS", "RESERVE", "AMOUNT", "STATUS", "DATE"
                    );
                    println!("  {}", "-".repeat(106));
                    for w in &pending_withdrawals {
                        let date = w.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
                        println!(
                            "  {:<4} {:<50} {:<8} {:<12} {:<10} {:<20}",
                            w.id.unwrap_or(0),
                            w.withdraw_address,
                            w.reserve_id,
                            w.amount,
                            "PENDING",
                            date
                        );
                    }
                    let total_pending_wd: i64 = pending_withdrawals.iter().map(|w| w.amount).sum();
                    println!(
                        "\n  Total pending withdrawals: {total_pending_wd} sats ({} request(s))",
                        pending_withdrawals.len()
                    );
                    println!(
                        "  Run `wallet withdraw-status` to check and update confirmation status."
                    );
                }
            }

            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::DepositStatus { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::WithdrawBtc {
            reserve_id,
            amount,
            wallet_id,
            password,
        } => {
            if nyks_wallet::config::NETWORK_TYPE.as_str() != "mainnet" {
                return Err("withdraw-btc is only available on mainnet.".to_string());
            }

            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            let tw_addr = ow.wallet.twilightaddress.clone();
            let btc_addr = ow.wallet.btc_address.clone();

            // Withdrawals go to the wallet's registered BTC address
            if !ow.wallet.btc_address_registered {
                return Err(format!(
                    "BTC address {} is not registered on-chain. Register first with:\n  \
                     wallet register-btc --amount <sats>",
                    btc_addr
                ));
            }

            println!("Submitting BTC withdrawal request");
            println!("  From:       {tw_addr}");
            println!("  To (BTC):   {btc_addr}");
            println!("  Reserve ID: {reserve_id}");
            println!("  Amount:     {amount} sats");

            match ow.wallet.withdraw_btc(&btc_addr, reserve_id, amount).await {
                Ok(tx_hash) => {
                    println!("\nWithdrawal request submitted successfully");
                    println!("  TX Hash: {tx_hash}");

                    // Save withdrawal record to database
                    if let Some(db_manager) = ow.get_db_manager() {
                        let now = chrono::Utc::now().naive_utc();
                        let withdrawal_entry = nyks_wallet::database::models::NewDbBtcWithdrawal {
                            wallet_id: db_manager.get_wallet_id().to_string(),
                            network_type: nyks_wallet::config::NETWORK_TYPE.to_string(),
                            withdraw_address: btc_addr.clone(),
                            twilight_address: tw_addr.clone(),
                            reserve_id: reserve_id as i64,
                            amount: amount as i64,
                            tx_hash: Some(tx_hash.clone()),
                            status: "submitted".to_string(),
                            created_at: now,
                            updated_at: now,
                        };
                        if let Err(e) = db_manager.save_btc_withdrawal(withdrawal_entry) {
                            eprintln!("Warning: Could not save withdrawal to database: {e}");
                        }
                    }

                    println!("\nThe withdrawal will be processed by validators.");
                    println!("Check status with: wallet withdraw-status");
                    Ok(())
                }
                Err(e) => Err(format!("Withdrawal failed: {e}")),
            }
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::WithdrawBtc { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::WithdrawStatus {
            wallet_id,
            password,
        } => {
            if nyks_wallet::config::NETWORK_TYPE.as_str() != "mainnet" {
                return Err("withdraw-status is only available on mainnet.".to_string());
            }
            let ow = resolve_order_wallet(wallet_id, password).await?;

            let db_manager = ow.get_db_manager().ok_or("No database manager available")?;

            // Load all withdrawals (not just pending) for display
            let all_withdrawals = db_manager
                .load_btc_withdrawals()
                .map_err(|e| format!("Failed to load withdrawals: {e}"))?;

            if all_withdrawals.is_empty() {
                println!("No BTC withdrawal requests found in database.");
                println!("Submit one with: wallet withdraw-btc --to <btc_addr> --reserve-id <id> --amount <sats>");
                return Ok(());
            }

            // Check pending withdrawals against on-chain status
            let pending: Vec<_> = all_withdrawals
                .iter()
                .filter(|w| w.status == "submitted")
                .collect();

            if !pending.is_empty() {
                println!(
                    "Checking {} pending withdrawal(s) against on-chain status...\n",
                    pending.len()
                );
                let mut updated_count = 0;
                for w in &pending {
                    let amount: u64 = w.amount as u64;
                    match ow
                        .wallet
                        .fetch_withdrawal_status(w.reserve_id as u64, &w.withdraw_address, amount)
                        .await
                    {
                        Ok(Some(status)) => {
                            if status.is_confirmed {
                                // Update DB to confirmed
                                if let Some(id) = w.id {
                                    let _ =
                                        db_manager.update_btc_withdrawal_status(id, "confirmed");
                                    updated_count += 1;
                                    println!(
                                        "  Updated withdrawal #{} ({} sats to {}) -> CONFIRMED",
                                        status.withdraw_identifier, w.amount, w.withdraw_address
                                    );
                                }
                            }
                        }
                        Ok(None) => {
                            // Not found on chain yet — might still be processing
                        }
                        Err(e) => {
                            eprintln!(
                                "  Warning: Could not check withdrawal {} sats to {}: {e}",
                                w.amount, w.withdraw_address
                            );
                        }
                    }
                }
                if updated_count > 0 {
                    println!("\n{updated_count} withdrawal(s) confirmed on-chain.\n");
                } else {
                    println!("No new confirmations found.\n");
                }
            }

            // Reload and display all withdrawals (with updated statuses)
            let withdrawals = db_manager
                .load_btc_withdrawals()
                .map_err(|e| format!("Failed to reload withdrawals: {e}"))?;

            println!("BTC Withdrawals ({}):", withdrawals.len());
            println!(
                "  {:<4} {:<50} {:<8} {:<12} {:<10} {:<20}",
                "ID", "BTC ADDRESS", "RESERVE", "AMOUNT", "STATUS", "DATE"
            );
            println!("  {}", "-".repeat(106));
            for w in &withdrawals {
                let date = w.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
                let status_display = match w.status.as_str() {
                    "confirmed" => "CONFIRMED",
                    "submitted" => "PENDING",
                    other => other,
                };
                println!(
                    "  {:<4} {:<50} {:<8} {:<12} {:<10} {:<20}",
                    w.id.unwrap_or(0),
                    w.withdraw_address,
                    w.reserve_id,
                    w.amount,
                    status_display,
                    date
                );
            }

            let confirmed_count = withdrawals
                .iter()
                .filter(|w| w.status == "confirmed")
                .count();
            let pending_count = withdrawals
                .iter()
                .filter(|w| w.status == "submitted")
                .count();
            let total_confirmed: i64 = withdrawals
                .iter()
                .filter(|w| w.status == "confirmed")
                .map(|w| w.amount)
                .sum();
            println!(
                "\n  Total: {} confirmed ({total_confirmed} sats), {} pending",
                confirmed_count, pending_count
            );
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::WithdrawStatus { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::DepositBtc {
            amount,
            amount_mbtc,
            amount_btc,
            reserve_address,
            wallet_id,
            password,
        } => {
            if nyks_wallet::config::NETWORK_TYPE.as_str() != "mainnet" {
                return Err(
                    "deposit-btc is only available on mainnet. Use `wallet faucet` for testnet tokens."
                        .to_string(),
                );
            }

            // Resolve amount (at least one is required)
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

            let ow = resolve_order_wallet(wallet_id, password).await?;
            let btc_addr = ow.wallet.btc_address.clone();
            let tw_addr = ow.wallet.twilightaddress.clone();

            // 1. Check if BTC address is registered on-chain
            println!("Checking BTC registration status...");
            match ow.wallet.fetch_registered_btc_by_address(&btc_addr).await {
                Ok(Some(info)) => {
                    if info.twilight_address != tw_addr {
                        return Err(format!(
                            "BTC address {btc_addr} is registered to a different twilight address: {}. \
                             You cannot deposit from this address.",
                            info.twilight_address
                        ));
                    }
                    println!("BTC address is registered:");
                    println!("  BTC address:      {}", info.btc_deposit_address);
                    println!("  Twilight address: {}", info.twilight_address);
                }
                Ok(None) => {
                    return Err(format!(
                        "BTC address {btc_addr} is not registered on-chain.\n\
                         Register first with: wallet register-btc --amount <sats>"
                    ));
                }
                Err(e) => {
                    eprintln!("Warning: Could not check registration status ({e}). Proceeding anyway...");
                }
            }
            println!("  Deposit amount:   {amount_sats} sats");

            // 2. Show reserve addresses (or the one user chose)
            let reserves = ow
                .wallet
                .fetch_btc_reserves()
                .await
                .map_err(|e| format!("Failed to fetch reserves: {e}"))?;

            let btc_height = nyks_wallet::wallet::wallet::fetch_btc_block_height()
                .await
                .unwrap_or(0);

            // 3. Resolve target reserve
            let target_reserve = if let Some(ref chosen) = reserve_address {
                // User explicitly chose a reserve — validate it
                let found = reserves
                    .iter()
                    .find(|r| r.reserve_address == *chosen)
                    .ok_or_else(|| {
                        format!("Reserve address {chosen} not found. Run `wallet reserves` to see available reserves.")
                    })?;
                let bl = reserve_blocks_left(found, btc_height);
                if bl <= 4 {
                    return Err(format!(
                        "Reserve {} is CRITICAL or EXPIRED. Pick a different reserve with more time remaining.",
                        chosen
                    ));
                }
                (found.clone(), bl)
            } else if let Some(best) = find_best_reserve(&reserves, btc_height) {
                best
            } else {
                return Err(
                    "No active reserves available. Wait for reserve rotation.".to_string(),
                );
            };

            let (ref reserve, blocks_left) = target_reserve;
            let status = reserve_status_label(blocks_left).unwrap_or("UNKNOWN");
            println!("\nTarget reserve:");
            println!("  Address:  {} (ID: {})", reserve.reserve_address, reserve.reserve_id);
            println!(
                "  Status:   {status} (~{blocks_left} blocks / ~{} min remaining)",
                blocks_left * 10
            );

            // 4. Auto-pay if btc_wallet available, otherwise manual instructions
            if let Some(ref btc_wallet) = ow.wallet.btc_wallet {
                // Estimate fee
                println!("\nEstimating transaction fee...");
                let estimated_fee = match nyks_wallet::wallet::btc_wallet::send::estimate_btc_fee(
                    &nyks_wallet::wallet::btc_wallet::send::SendBtcParams {
                        btc_wallet,
                        destination: reserve.reserve_address.clone(),
                        amount_sats,
                        fee_rate_sat_vb: None,
                    },
                )
                .await
                {
                    Ok(fee) => {
                        println!("  Estimated fee: {fee} sats");
                        fee
                    }
                    Err(e) => {
                        eprintln!(
                            "  Warning: Could not estimate fee ({e}), using 2000 sat buffer"
                        );
                        2_000
                    }
                };

                let required_sats = amount_sats + estimated_fee;
                println!("Checking BTC balance for {btc_addr}...");
                let btc_balance = nyks_wallet::wallet::wallet::fetch_btc_balance(&btc_addr)
                    .await
                    .map_err(|e| format!("Failed to fetch BTC balance: {e}"))?;

                if btc_balance.confirmed_sats < required_sats {
                    return Err(format!(
                        "Insufficient BTC balance. You need at least {required_sats} sats but your address \
                         has only {} confirmed sats.\n  \
                         (Amount: {amount_sats} sats + estimated fee: {estimated_fee} sats = {required_sats} sats needed)\n\n\
                         Please send at least {} more sats to your BTC address:\n  {btc_addr}",
                        btc_balance.confirmed_sats,
                        required_sats - btc_balance.confirmed_sats,
                    ));
                }
                println!(
                    "  Balance OK: {} confirmed sats (need {required_sats} sats incl. estimated fee)\n",
                    btc_balance.confirmed_sats
                );

                // Send
                println!("Sending {amount_sats} sats to reserve...");
                match nyks_wallet::wallet::btc_wallet::send::send_btc(
                    nyks_wallet::wallet::btc_wallet::send::SendBtcParams {
                        btc_wallet,
                        destination: reserve.reserve_address.clone(),
                        amount_sats,
                        fee_rate_sat_vb: None,
                    },
                )
                .await
                {
                    Ok(result) => {
                        println!("\nBTC payment sent successfully!");
                        println!("  TX ID: {}", result.txid);
                        println!("  Fee:   {} sats", result.fee_sats);
                        save_deposit_record(
                            &ow,
                            &btc_addr,
                            &tw_addr,
                            Some(reserve.reserve_address.clone()),
                            Some(reserve.reserve_id),
                            amount_sats,
                            0,
                            None,
                            Some(result.txid.clone()),
                            "sent",
                        );
                        // Save BTC transfer record for bitcoin-wallet history
                        if let Some(db) = ow.get_db_manager() {
                            let record = nyks_wallet::database::models::NewDbBtcTransfer {
                                wallet_id: db.get_wallet_id().to_string(),
                                network_type: nyks_wallet::config::BTC_NETWORK_TYPE.clone(),
                                from_address: btc_addr.clone(),
                                to_address: reserve.reserve_address.clone(),
                                amount: amount_sats as i64,
                                fee: result.fee_sats as i64,
                                tx_id: Some(result.txid.clone()),
                                status: "broadcast".to_string(),
                                confirmations: 0,
                                created_at: chrono::Utc::now().naive_utc(),
                                updated_at: chrono::Utc::now().naive_utc(),
                            };
                            if let Err(e) = db.save_btc_transfer(record) {
                                eprintln!("Warning: Failed to save transfer to DB: {e}");
                            }
                        }
                        println!("\nCheck status with: wallet deposit-status");
                    }
                    Err(e) => {
                        eprintln!("\nFailed to send BTC: {e}");
                        eprintln!("You can retry or send manually:");
                        eprintln!(
                            "  Send {amount_sats} sats from {btc_addr} to {}",
                            reserve.reserve_address
                        );
                        save_deposit_record(
                            &ow,
                            &btc_addr,
                            &tw_addr,
                            Some(reserve.reserve_address.clone()),
                            Some(reserve.reserve_id),
                            amount_sats,
                            0,
                            None,
                            None,
                            "pending",
                        );
                    }
                }
            } else {
                // No btc_wallet — manual flow
                println!("\nNote: BTC wallet not loaded — manual payment required.");
                println!("  To enable auto-pay, load a mnemonic:");
                println!("  bitcoin-wallet update-bitcoin-wallet --mnemonic <phrase>\n");

                save_deposit_record(
                    &ow,
                    &btc_addr,
                    &tw_addr,
                    Some(reserve.reserve_address.clone()),
                    Some(reserve.reserve_id),
                    amount_sats,
                    0,
                    None,
                    None,
                    "pending",
                );

                println!(
                    "\nSend {amount_sats} sats from your registered BTC address ONLY:"
                );
                println!("  From: {btc_addr}");
                println!("  To:   {}", reserve.reserve_address);
                println!(
                    "\nIMPORTANT: You MUST send from {btc_addr} (the registered address)."
                );
                println!(
                    "Sending from any other address will NOT be credited to your account."
                );
                println!("\nAfter sending, check status with: wallet deposit-status");
            }
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::DepositBtc { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),

        #[cfg(any(feature = "sqlite", feature = "postgresql"))]
        WalletCmd::Faucet {
            wallet_id,
            password,
        } => {
            if nyks_wallet::config::NETWORK_TYPE.as_str() == "mainnet" {
                return Err("faucet is only available on testnet. Use `wallet register-btc` for mainnet deposits.".to_string());
            }
            let mut ow = resolve_order_wallet(wallet_id, password).await?;
            let tw_addr = ow.wallet.twilightaddress.clone();

            println!("Requesting test tokens for {tw_addr}...");
            nyks_wallet::wallet::wallet::get_test_tokens(&mut ow.wallet)
                .await
                .map_err(|e| format!("Failed to get test tokens: {e}"))?;

            let balance = ow
                .wallet
                .update_balance()
                .await
                .map_err(|e| e.to_string())?;
            println!("\nUpdated balance:");
            println!("  NYKS: {}", balance.nyks);
            println!("  SATS: {}", balance.sats);
            Ok(())
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
        WalletCmd::Faucet { .. } => Err(
            "Database features (sqlite/postgresql) not enabled. Rebuild with --features sqlite"
                .to_string(),
        ),
    }
}
