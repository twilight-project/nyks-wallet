#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use crate::database::{
    connection::get_conn,
    models::{
        DbBtcDeposit, DbBtcWithdrawal, DbOrderHistory, DbOrderWallet, DbRequestId,
        DbTransferHistory, DbUtxoDetail, DbZkAccount, EncryptedWallet,
    },
    operations::DatabaseManager,
    schema::{
        btc_deposits, btc_withdrawals, encrypted_wallets, order_history, order_wallets,
        request_ids, transfer_history, utxo_details, zk_accounts,
    },
};
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use diesel::prelude::*;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use log::debug;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use serde::{Deserialize, Serialize};

/// Version of the backup format. Increment when the schema changes.
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
const BACKUP_FORMAT_VERSION: u32 = 3;

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
fn current_network_type() -> String {
    crate::config::NETWORK_TYPE.to_string()
}

/// A full, serializable snapshot of all database state for a single wallet.
///
/// Binary fields (encrypted_data, salt, nonce, seed_encrypted, etc.) are
/// stored as base64-encoded strings so the backup is valid JSON / plain text.
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBackup {
    pub format_version: u32,
    pub wallet_id: String,
    pub network_type: String,
    pub created_at: String,
    pub zk_accounts: Vec<DbZkAccount>,
    pub encrypted_wallet: Option<EncryptedWallet>,
    pub order_wallet: Option<DbOrderWallet>,
    pub utxo_details: Vec<DbUtxoDetail>,
    pub request_ids: Vec<DbRequestId>,
    pub order_history: Vec<DbOrderHistory>,
    pub transfer_history: Vec<DbTransferHistory>,
    #[serde(default)]
    pub btc_deposits: Vec<DbBtcDeposit>,
    #[serde(default)]
    pub btc_withdrawals: Vec<DbBtcWithdrawal>,
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
impl DatabaseManager {
    /// Export all database state for this wallet into a serializable backup.
    pub fn export_backup(&self) -> Result<WalletBackup, String> {
        let net = current_network_type();
        let mut conn = get_conn(self.pool())?;

        let zk_accts: Vec<DbZkAccount> = zk_accounts::table
            .filter(zk_accounts::wallet_id.eq(self.get_wallet_id()))
            .filter(zk_accounts::network_type.eq(&net))
            .load(&mut conn)
            .map_err(|e| format!("Failed to export zk_accounts: {}", e))?;

        let enc_wallet: Option<EncryptedWallet> = encrypted_wallets::table
            .filter(encrypted_wallets::wallet_id.eq(self.get_wallet_id()))
            .first(&mut conn)
            .optional()
            .map_err(|e| format!("Failed to export encrypted_wallet: {}", e))?;

        let ord_wallet: Option<DbOrderWallet> = order_wallets::table
            .filter(order_wallets::wallet_id.eq(self.get_wallet_id()))
            .filter(order_wallets::network_type.eq(&net))
            .first(&mut conn)
            .optional()
            .map_err(|e| format!("Failed to export order_wallet: {}", e))?;

        let utxos: Vec<DbUtxoDetail> = utxo_details::table
            .filter(utxo_details::wallet_id.eq(self.get_wallet_id()))
            .filter(utxo_details::network_type.eq(&net))
            .load(&mut conn)
            .map_err(|e| format!("Failed to export utxo_details: {}", e))?;

        let req_ids: Vec<DbRequestId> = request_ids::table
            .filter(request_ids::wallet_id.eq(self.get_wallet_id()))
            .filter(request_ids::network_type.eq(&net))
            .load(&mut conn)
            .map_err(|e| format!("Failed to export request_ids: {}", e))?;

        let ord_hist: Vec<DbOrderHistory> = order_history::table
            .filter(order_history::wallet_id.eq(self.get_wallet_id()))
            .filter(order_history::network_type.eq(&net))
            .order(order_history::created_at.desc())
            .load(&mut conn)
            .map_err(|e| format!("Failed to export order_history: {}", e))?;

        let xfer_hist: Vec<DbTransferHistory> = transfer_history::table
            .filter(transfer_history::wallet_id.eq(self.get_wallet_id()))
            .filter(transfer_history::network_type.eq(&net))
            .order(transfer_history::created_at.desc())
            .load(&mut conn)
            .map_err(|e| format!("Failed to export transfer_history: {}", e))?;

        let btc_deps: Vec<DbBtcDeposit> = btc_deposits::table
            .filter(btc_deposits::wallet_id.eq(self.get_wallet_id()))
            .filter(btc_deposits::network_type.eq(&net))
            .order(btc_deposits::created_at.desc())
            .load(&mut conn)
            .map_err(|e| format!("Failed to export btc_deposits: {}", e))?;

        let btc_wds: Vec<DbBtcWithdrawal> = btc_withdrawals::table
            .filter(btc_withdrawals::wallet_id.eq(self.get_wallet_id()))
            .filter(btc_withdrawals::network_type.eq(&net))
            .order(btc_withdrawals::created_at.desc())
            .load(&mut conn)
            .map_err(|e| format!("Failed to export btc_withdrawals: {}", e))?;

        let backup = WalletBackup {
            format_version: BACKUP_FORMAT_VERSION,
            wallet_id: self.get_wallet_id().to_string(),
            network_type: net,
            created_at: chrono::Utc::now().naive_utc().to_string(),
            zk_accounts: zk_accts,
            encrypted_wallet: enc_wallet,
            order_wallet: ord_wallet,
            utxo_details: utxos,
            request_ids: req_ids,
            order_history: ord_hist,
            transfer_history: xfer_hist,
            btc_deposits: btc_deps,
            btc_withdrawals: btc_wds,
        };

        debug!(
            "Exported backup for wallet {}: {} zk_accounts, {} utxos, {} order_history, {} transfer_history",
            self.get_wallet_id(),
            backup.zk_accounts.len(),
            backup.utxo_details.len(),
            backup.order_history.len(),
            backup.transfer_history.len(),
        );

        Ok(backup)
    }

    /// Serialize a backup to a JSON string.
    pub fn export_backup_json(&self) -> Result<String, String> {
        let backup = self.export_backup()?;
        serde_json::to_string_pretty(&backup)
            .map_err(|e| format!("Failed to serialize backup: {}", e))
    }

    /// Import a backup, replacing all existing data for this wallet on the current network.
    ///
    /// The backup's `wallet_id` must match this DatabaseManager's wallet_id
    /// (or `force` must be true to re-map data to the current wallet_id).
    pub fn import_backup(&self, backup: &WalletBackup, force: bool) -> Result<(), String> {
        // Accept both v1 and v2 backups
        if backup.format_version > BACKUP_FORMAT_VERSION {
            return Err(format!(
                "Unsupported backup format version: {} (max supported: {})",
                backup.format_version, BACKUP_FORMAT_VERSION
            ));
        }

        if backup.wallet_id != self.get_wallet_id() && !force {
            return Err(format!(
                "Backup wallet_id '{}' does not match current wallet_id '{}'. Pass force=true to override.",
                backup.wallet_id, self.get_wallet_id()
            ));
        }

        let net = current_network_type();
        let mut conn = get_conn(self.pool())?;
        let wallet_id = self.get_wallet_id();

        // Delete existing data for this wallet on this network (in dependency order)
        diesel::delete(
            btc_withdrawals::table
                .filter(btc_withdrawals::wallet_id.eq(wallet_id))
                .filter(btc_withdrawals::network_type.eq(&net)),
        )
        .execute(&mut conn)
        .map_err(|e| format!("Failed to clear btc_withdrawals: {}", e))?;

        diesel::delete(
            btc_deposits::table
                .filter(btc_deposits::wallet_id.eq(wallet_id))
                .filter(btc_deposits::network_type.eq(&net)),
        )
        .execute(&mut conn)
        .map_err(|e| format!("Failed to clear btc_deposits: {}", e))?;

        diesel::delete(
            transfer_history::table
                .filter(transfer_history::wallet_id.eq(wallet_id))
                .filter(transfer_history::network_type.eq(&net)),
        )
        .execute(&mut conn)
        .map_err(|e| format!("Failed to clear transfer_history: {}", e))?;

        diesel::delete(
            order_history::table
                .filter(order_history::wallet_id.eq(wallet_id))
                .filter(order_history::network_type.eq(&net)),
        )
        .execute(&mut conn)
        .map_err(|e| format!("Failed to clear order_history: {}", e))?;

        diesel::delete(
            request_ids::table
                .filter(request_ids::wallet_id.eq(wallet_id))
                .filter(request_ids::network_type.eq(&net)),
        )
        .execute(&mut conn)
        .map_err(|e| format!("Failed to clear request_ids: {}", e))?;

        diesel::delete(
            utxo_details::table
                .filter(utxo_details::wallet_id.eq(wallet_id))
                .filter(utxo_details::network_type.eq(&net)),
        )
        .execute(&mut conn)
        .map_err(|e| format!("Failed to clear utxo_details: {}", e))?;

        diesel::delete(
            order_wallets::table
                .filter(order_wallets::wallet_id.eq(wallet_id))
                .filter(order_wallets::network_type.eq(&net)),
        )
        .execute(&mut conn)
        .map_err(|e| format!("Failed to clear order_wallets: {}", e))?;

        diesel::delete(
            encrypted_wallets::table
                .filter(encrypted_wallets::wallet_id.eq(wallet_id)),
        )
        .execute(&mut conn)
        .map_err(|e| format!("Failed to clear encrypted_wallets: {}", e))?;

        diesel::delete(
            zk_accounts::table
                .filter(zk_accounts::wallet_id.eq(wallet_id))
                .filter(zk_accounts::network_type.eq(&net)),
        )
        .execute(&mut conn)
        .map_err(|e| format!("Failed to clear zk_accounts: {}", e))?;

        // Insert backup data (re-mapping wallet_id and network_type)
        for mut acct in backup.zk_accounts.clone() {
            acct.id = None;
            acct.wallet_id = wallet_id.to_string();
            acct.network_type = net.clone();
            diesel::insert_into(zk_accounts::table)
                .values(&acct)
                .execute(&mut conn)
                .map_err(|e| format!("Failed to import zk_account: {}", e))?;
        }

        if let Some(mut enc) = backup.encrypted_wallet.clone() {
            enc.id = None;
            enc.wallet_id = wallet_id.to_string();
            diesel::insert_into(encrypted_wallets::table)
                .values(&enc)
                .execute(&mut conn)
                .map_err(|e| format!("Failed to import encrypted_wallet: {}", e))?;
        }

        if let Some(mut ow) = backup.order_wallet.clone() {
            ow.id = None;
            ow.wallet_id = wallet_id.to_string();
            ow.network_type = net.clone();
            diesel::insert_into(order_wallets::table)
                .values(&ow)
                .execute(&mut conn)
                .map_err(|e| format!("Failed to import order_wallet: {}", e))?;
        }

        for mut utxo in backup.utxo_details.clone() {
            utxo.id = None;
            utxo.wallet_id = wallet_id.to_string();
            utxo.network_type = net.clone();
            diesel::insert_into(utxo_details::table)
                .values(&utxo)
                .execute(&mut conn)
                .map_err(|e| format!("Failed to import utxo_detail: {}", e))?;
        }

        for mut rid in backup.request_ids.clone() {
            rid.id = None;
            rid.wallet_id = wallet_id.to_string();
            rid.network_type = net.clone();
            diesel::insert_into(request_ids::table)
                .values(&rid)
                .execute(&mut conn)
                .map_err(|e| format!("Failed to import request_id: {}", e))?;
        }

        for mut oh in backup.order_history.clone() {
            oh.id = None;
            oh.wallet_id = wallet_id.to_string();
            oh.network_type = net.clone();
            diesel::insert_into(order_history::table)
                .values(&oh)
                .execute(&mut conn)
                .map_err(|e| format!("Failed to import order_history: {}", e))?;
        }

        for mut th in backup.transfer_history.clone() {
            th.id = None;
            th.wallet_id = wallet_id.to_string();
            th.network_type = net.clone();
            diesel::insert_into(transfer_history::table)
                .values(&th)
                .execute(&mut conn)
                .map_err(|e| format!("Failed to import transfer_history: {}", e))?;
        }

        for mut dep in backup.btc_deposits.clone() {
            dep.id = None;
            dep.wallet_id = wallet_id.to_string();
            dep.network_type = net.clone();
            diesel::insert_into(btc_deposits::table)
                .values(&dep)
                .execute(&mut conn)
                .map_err(|e| format!("Failed to import btc_deposit: {}", e))?;
        }

        for mut wd in backup.btc_withdrawals.clone() {
            wd.id = None;
            wd.wallet_id = wallet_id.to_string();
            wd.network_type = net.clone();
            diesel::insert_into(btc_withdrawals::table)
                .values(&wd)
                .execute(&mut conn)
                .map_err(|e| format!("Failed to import btc_withdrawal: {}", e))?;
        }

        debug!(
            "Imported backup for wallet {}: {} zk_accounts, {} utxos, {} order_history, {} transfer_history",
            wallet_id,
            backup.zk_accounts.len(),
            backup.utxo_details.len(),
            backup.order_history.len(),
            backup.transfer_history.len(),
        );

        Ok(())
    }

    /// Deserialize a JSON string and import it.
    pub fn import_backup_json(&self, json: &str, force: bool) -> Result<(), String> {
        let backup: WalletBackup = serde_json::from_str(json)
            .map_err(|e| format!("Failed to deserialize backup: {}", e))?;
        self.import_backup(&backup, force)
    }

    /// Export backup directly to a file path.
    pub fn export_backup_to_file(&self, path: &str) -> Result<(), String> {
        let json = self.export_backup_json()?;
        std::fs::write(path, &json)
            .map_err(|e| format!("Failed to write backup file '{}': {}", path, e))?;
        debug!("Backup written to {}", path);
        Ok(())
    }

    /// Import backup from a file path.
    pub fn import_backup_from_file(&self, path: &str, force: bool) -> Result<(), String> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read backup file '{}': {}", path, e))?;
        self.import_backup_json(&json, force)
    }
}
