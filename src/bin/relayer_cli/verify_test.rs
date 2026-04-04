use nyks_wallet::relayer_module::order_wallet::OrderWallet;

use crate::commands::VerifyTestCmd;

/// Print a step header for verify-test output.
fn vt_step(step: u32, total: u32, desc: &str) {
    println!("\n[{step}/{total}] {desc}");
    println!("{}", "-".repeat(60));
}

/// Print PASS / FAIL result for a verify-test step.
fn vt_result(name: &str, result: &Result<(), String>) {
    match result {
        Ok(()) => println!("  PASS: {name}"),
        Err(e) => println!("  FAIL: {name} -> {e}"),
    }
}

pub(crate) async fn handle_verify_test(cmd: VerifyTestCmd) -> Result<(), String> {
    if nyks_wallet::config::NETWORK_TYPE.as_str() == "mainnet" {
        return Err(
            "verify-test is only available on testnet. Set NETWORK_TYPE=testnet to use this command."
                .to_string(),
        );
    }

    println!("==========================================================");
    println!("  VERIFY-TEST  (testnet)");
    println!("  Network: {}", nyks_wallet::config::NETWORK_TYPE.as_str());
    println!("==========================================================");

    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut skipped = 0u32;

    match cmd {
        VerifyTestCmd::Wallet => {
            verify_wallet(&mut passed, &mut failed, &mut skipped).await;
        }
        VerifyTestCmd::Market => {
            verify_market(&mut passed, &mut failed, &mut skipped).await;
        }
        VerifyTestCmd::Zkaccount => {
            verify_zkaccount(&mut passed, &mut failed, &mut skipped).await;
        }
        VerifyTestCmd::Order => {
            verify_order(&mut passed, &mut failed, &mut skipped).await;
        }
        VerifyTestCmd::All => {
            verify_wallet(&mut passed, &mut failed, &mut skipped).await;
            verify_market(&mut passed, &mut failed, &mut skipped).await;
            verify_zkaccount(&mut passed, &mut failed, &mut skipped).await;
            verify_order(&mut passed, &mut failed, &mut skipped).await;
        }
    }

    let total = passed + failed + skipped;
    println!("\n==========================================================");
    println!("  RESULTS: {total} tests — {passed} passed, {failed} failed, {skipped} skipped");
    println!("==========================================================");

    if failed > 0 {
        Err(format!("{failed} test(s) failed"))
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// verify-test: wallet
// ---------------------------------------------------------------------------

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
async fn verify_wallet(passed: &mut u32, failed: &mut u32, _skipped: &mut u32) {
    use secrecy::SecretString;

    let total_steps = 10;
    let test_wallet_id = format!("verify-test-{}", chrono::Utc::now().timestamp());
    let test_password = "verify-test-password";

    println!("\n## Wallet Verification");
    println!("  Test wallet ID: {test_wallet_id}");

    // 1. Create wallet
    vt_step(1, total_steps, "wallet create");
    let create_result = (|| -> Result<OrderWallet, String> {
        let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;
        let pwd = Some(SecretString::new(test_password.into()));
        ow.with_db(pwd, Some(test_wallet_id.clone()))?;
        Ok(ow)
    })();
    let mut ow = match create_result {
        Ok(ow) => {
            println!("  PASS: wallet create");
            println!("    Address: {}", ow.wallet.twilightaddress);
            println!("    BTC addr: {}", ow.wallet.btc_address);
            *passed += 1;
            ow
        }
        Err(e) => {
            println!("  FAIL: wallet create -> {e}");
            *failed += 1;
            println!("  Cannot continue wallet tests without a wallet.");
            return;
        }
    };

    // 2. Wallet info (read-only)
    vt_step(2, total_steps, "wallet info");
    let info_result: Result<(), String> = {
        let addr = &ow.wallet.twilightaddress;
        let btc = &ow.wallet.btc_address;
        if addr.starts_with("twilight1") && !btc.is_empty() {
            println!("    Twilight address format: OK");
            println!("    BTC address present: OK");
            Ok(())
        } else {
            Err(format!("Unexpected address format: {addr} / {btc}"))
        }
    };
    vt_result("wallet info", &info_result);
    if info_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }

    // 3. Faucet (get test tokens)
    vt_step(3, total_steps, "wallet faucet (get test tokens)");
    let faucet_result = nyks_wallet::wallet::wallet::get_test_tokens(&mut ow.wallet)
        .await
        .map_err(|e| format!("{e}"));
    vt_result("wallet faucet", &faucet_result);
    if faucet_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }

    // 4. Balance check
    vt_step(4, total_steps, "wallet balance");
    let balance_result = ow.wallet.update_balance().await.map_err(|e| format!("{e}"));
    match &balance_result {
        Ok(bal) => {
            println!("  PASS: wallet balance");
            println!("    NYKS: {}", bal.nyks);
            println!("    SATS: {}", bal.sats);
            *passed += 1;
        }
        Err(e) => {
            println!("  FAIL: wallet balance -> {e}");
            *failed += 1;
        }
    }

    // 5. Export wallet to JSON
    vt_step(5, total_steps, "wallet export");
    let export_path = format!("/tmp/verify-test-{}.json", chrono::Utc::now().timestamp());
    let export_result = ow
        .wallet
        .export_to_json(&export_path)
        .map(|_| {
            println!("    Exported to: {export_path}");
            let _ = std::fs::remove_file(&export_path); // cleanup
        })
        .map_err(|e| format!("{e}"));
    vt_result("wallet export", &export_result);
    if export_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }

    // 6. Wallet accounts (ZkOS account list)
    vt_step(6, total_steps, "wallet accounts");
    {
        let count = ow.zk_accounts.accounts.len();
        println!("  PASS: wallet accounts ({count} ZkOS accounts)");
        *passed += 1;
    }

    // 7. Reserves query
    vt_step(7, total_steps, "wallet reserves");
    let reserves_result = ow
        .wallet
        .fetch_btc_reserves()
        .await
        .map_err(|e| format!("{e}"));
    match &reserves_result {
        Ok(reserves) => {
            println!(
                "  PASS: wallet reserves ({} reserves found)",
                reserves.len()
            );
            *passed += 1;
        }
        Err(e) => {
            println!("  FAIL: wallet reserves -> {e}");
            *failed += 1;
        }
    }

    // 8. Send tokens (small amount to self)
    vt_step(8, total_steps, "wallet send (1 nyks to self)");
    let self_addr = ow.wallet.twilightaddress.clone();
    let send_result = ow
        .wallet
        .send_tokens(&self_addr, 1, "nyks")
        .await
        .map(|tx_hash| {
            println!("    TX hash: {tx_hash}");
        })
        .map_err(|e| format!("{e}"));
    vt_result("wallet send", &send_result);
    if send_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }

    // 9. Backup/restore
    vt_step(9, total_steps, "wallet backup");
    let backup_result = if let Some(db_manager) = ow.get_db_manager() {
        db_manager.export_backup_json().map(|json| {
            println!("    Backup JSON length: {} bytes", json.len());
        })
    } else {
        Err("No DB manager".to_string())
    };
    vt_result("wallet backup", &backup_result);
    if backup_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }

    // 10. Sync nonce
    vt_step(10, total_steps, "wallet sync-nonce");
    let nonce_result = ow
        .wallet
        .update_account_info()
        .await
        .map(|_| {
            println!("    Nonce synced OK");
        })
        .map_err(|e| format!("{e}"));
    vt_result("wallet sync-nonce", &nonce_result);
    if nonce_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }
}

#[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
async fn verify_wallet(_passed: &mut u32, failed: &mut u32, _skipped: &mut u32) {
    println!("\n## Wallet Verification");
    println!("  FAIL: Database features not enabled. Rebuild with --features sqlite");
    *failed += 1;
}

// ---------------------------------------------------------------------------
// verify-test: market
// ---------------------------------------------------------------------------

async fn verify_market(passed: &mut u32, failed: &mut u32, _skipped: &mut u32) {
    use nyks_wallet::relayer_module::relayer_api::RelayerJsonRpcClient;

    let total_steps = 5;
    println!("\n## Market Verification");

    let endpoint = nyks_wallet::config::RELAYER_API_RPC_SERVER_URL.to_string();
    let client = match RelayerJsonRpcClient::new(&endpoint) {
        Ok(c) => c,
        Err(e) => {
            println!("  FAIL: Cannot create RelayerJsonRpcClient: {e}");
            *failed += 1;
            return;
        }
    };
    println!("  Endpoint: {endpoint}");

    // 1. BTC/USD price
    vt_step(1, total_steps, "market price");
    let price_result = client
        .btc_usd_price()
        .await
        .map(|p| {
            println!("    BTC/USD: ${:.2}", p.price);
        })
        .map_err(|e| format!("{e}"));
    vt_result("market price", &price_result);
    if price_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }

    // 2. Orderbook
    vt_step(2, total_steps, "market orderbook");
    let book_result = client
        .open_limit_orders()
        .await
        .map(|book| {
            let count = book.bid.len() + book.ask.len();
            println!(
                "    Orders: {count} (bid: {}, ask: {})",
                book.bid.len(),
                book.ask.len()
            );
        })
        .map_err(|e| format!("{e}"));
    vt_result("market orderbook", &book_result);
    if book_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }

    // 3. Funding rate
    vt_step(3, total_steps, "market funding-rate");
    let fund_result = client
        .get_funding_rate()
        .await
        .map(|f| {
            println!("    Funding rate: {:.6}%", f.rate);
        })
        .map_err(|e| format!("{e}"));
    vt_result("market funding-rate", &fund_result);
    if fund_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }

    // 4. Lend pool info
    vt_step(4, total_steps, "market lend-pool-info");
    let pool_result = client
        .lend_pool_info()
        .await
        .map(|pool| {
            println!("    Lend pool share: {:.2}", pool.total_pool_share);
        })
        .map_err(|e| format!("{e}"));
    vt_result("market lend-pool-info", &pool_result);
    if pool_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }

    // 5. Server time
    vt_step(5, total_steps, "market server-time");
    let time_result = client
        .server_time()
        .await
        .map(|t| {
            println!("    Server time: {t}");
        })
        .map_err(|e| format!("{e}"));
    vt_result("market server-time", &time_result);
    if time_result.is_ok() {
        *passed += 1;
    } else {
        *failed += 1;
    }
}

// ---------------------------------------------------------------------------
// verify-test: zkaccount
// ---------------------------------------------------------------------------

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
async fn verify_zkaccount(passed: &mut u32, failed: &mut u32, skipped: &mut u32) {
    use secrecy::SecretString;

    let total_steps = 3;
    let test_wallet_id = format!("verify-zk-{}", chrono::Utc::now().timestamp());
    let test_password = "verify-test-password";

    println!("\n## ZkAccount Verification");
    println!("  Test wallet ID: {test_wallet_id}");

    // Setup: create wallet and get test tokens
    println!("\n  [setup] Creating wallet and funding via faucet...");
    let setup_result = async {
        let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;
        let pwd = Some(SecretString::new(test_password.into()));
        ow.with_db(pwd, Some(test_wallet_id.clone()))?;
        nyks_wallet::wallet::wallet::get_test_tokens(&mut ow.wallet)
            .await
            .map_err(|e| format!("{e}"))?;
        let bal = ow
            .wallet
            .update_balance()
            .await
            .map_err(|e| format!("{e}"))?;
        println!("    Wallet funded: {} sats", bal.sats);
        if bal.sats < 10000 {
            return Err("Insufficient sats for zkaccount tests (need >= 10000)".to_string());
        }
        Ok(ow)
    }
    .await;

    let mut ow = match setup_result {
        Ok(ow) => ow,
        Err(e) => {
            println!("  FAIL: zkaccount setup -> {e}");
            *failed += 1;
            *skipped += total_steps as u32;
            return;
        }
    };

    // 1. Fund ZkOS account
    vt_step(1, total_steps, "zkaccount fund (10000 sats)");
    let fund_result = ow.funding_to_trading(10000).await;
    let account_index = match &fund_result {
        Ok((tx, idx)) => {
            println!("  PASS: zkaccount fund");
            println!("    TX hash: {}", tx.tx_hash);
            println!("    Account index: {idx}");
            *passed += 1;
            *idx
        }
        Err(e) => {
            println!("  FAIL: zkaccount fund -> {e}");
            *failed += 1;
            *skipped += 2;
            println!("  Skipping remaining zkaccount tests.");
            return;
        }
    };

    // 2. Query ZkOS account
    vt_step(2, total_steps, "zkaccount query");
    let query_result: Result<(), String> = {
        match ow.zk_accounts.accounts.get(&account_index) {
            Some(acct) => {
                println!("  PASS: zkaccount query");
                println!("    Index: {}", acct.index);
                println!("    Balance: {}", acct.balance);
                println!("    On-chain: {}", acct.on_chain);
                *passed += 1;
                Ok(())
            }
            None => {
                let err = format!("Account index {account_index} not found");
                println!("  FAIL: zkaccount query -> {err}");
                *failed += 1;
                Err(err)
            }
        }
    };

    // 3. Withdraw ZkOS account back to on-chain
    vt_step(3, total_steps, "zkaccount withdraw");
    if query_result.is_ok() {
        let withdraw_result = ow.trading_to_funding(account_index).await;
        match &withdraw_result {
            Ok(()) => {
                println!("  PASS: zkaccount withdraw (trading_to_funding)");
                *passed += 1;
            }
            Err(e) => {
                println!("  FAIL: zkaccount withdraw -> {e}");
                *failed += 1;
            }
        }
    } else {
        println!("  SKIP: zkaccount withdraw (no account to withdraw)");
        *skipped += 1;
    }
}

#[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
async fn verify_zkaccount(_passed: &mut u32, failed: &mut u32, _skipped: &mut u32) {
    println!("\n## ZkAccount Verification");
    println!("  FAIL: Database features not enabled. Rebuild with --features sqlite");
    *failed += 1;
}

// ---------------------------------------------------------------------------
// verify-test: order
// ---------------------------------------------------------------------------

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
async fn verify_order(passed: &mut u32, failed: &mut u32, skipped: &mut u32) {
    use secrecy::SecretString;
    use twilight_client_sdk::relayer_types::{OrderType, PositionType};

    let total_steps = 3;
    let test_wallet_id = format!("verify-order-{}", chrono::Utc::now().timestamp());
    let test_password = "verify-test-password";

    println!("\n## Order Verification");
    println!("  Test wallet ID: {test_wallet_id}");

    // Setup: create wallet, fund, create ZkOS account
    println!("\n  [setup] Creating wallet, funding, and creating ZkOS account...");
    let setup_result = async {
        let mut ow = OrderWallet::new(None).map_err(|e| e.to_string())?;
        let pwd = Some(SecretString::new(test_password.into()));
        ow.with_db(pwd, Some(test_wallet_id.clone()))?;
        nyks_wallet::wallet::wallet::get_test_tokens(&mut ow.wallet)
            .await
            .map_err(|e| format!("{e}"))?;
        let bal = ow
            .wallet
            .update_balance()
            .await
            .map_err(|e| format!("{e}"))?;
        println!("    Wallet funded: {} sats", bal.sats);
        if bal.sats < 20000 {
            return Err("Insufficient sats for order tests (need >= 20000)".to_string());
        }
        let (_, idx) = ow.funding_to_trading(100).await?;
        println!("    ZkOS account created: index {idx}");
        Ok((ow, idx))
    }
    .await;

    let (mut ow, account_index) = match setup_result {
        Ok(v) => v,
        Err(e) => {
            println!("  FAIL: order setup -> {e}");
            *failed += 1;
            *skipped += total_steps as u32;
            return;
        }
    };

    // 1. Open a MARKET LONG order
    vt_step(1, total_steps, "order open-trade (MARKET LONG)");
    let open_result = ow
        .open_trader_order(
            account_index,
            OrderType::MARKET,
            PositionType::LONG,
            75000,
            2,
        )
        .await;
    match &open_result {
        Ok(request_id) => {
            println!("  PASS: order open-trade");
            println!("    Request ID: {request_id}");
            *passed += 1;
        }
        Err(e) => {
            println!("  FAIL: order open-trade -> {e}");
            *failed += 1;
            *skipped += 2;
            println!("  Skipping remaining order tests.");
            return;
        }
    }

    // 2. Query the order
    vt_step(2, total_steps, "order query-trade");
    // Brief wait for order to be processed
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    let query_result = ow.query_trader_order(account_index).await;
    match &query_result {
        Ok(order) => {
            println!("  PASS: order query-trade");
            println!("    Status: {:?}", order.order_status);
            *passed += 1;
        }
        Err(e) => {
            println!("  FAIL: order query-trade -> {e}");
            *failed += 1;
        }
    }

    // 3. Close the order
    vt_step(3, total_steps, "order close-trade");
    let close_result = ow
        .close_trader_order(account_index, OrderType::MARKET, 0.0)
        .await;
    match &close_result {
        Ok(request_id) => {
            println!("  PASS: order close-trade");
            println!("    Request ID: {request_id}");
            *passed += 1;
        }
        Err(e) => {
            println!("  FAIL: order close-trade -> {e}");
            *failed += 1;
        }
    }
}

#[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
async fn verify_order(_passed: &mut u32, failed: &mut u32, _skipped: &mut u32) {
    println!("\n## Order Verification");
    println!("  FAIL: Database features not enabled. Rebuild with --features sqlite");
    *failed += 1;
}
