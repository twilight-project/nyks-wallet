-- SQLite does not support DROP COLUMN directly, so we recreate the table
CREATE TABLE zk_accounts_backup (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT NOT NULL,
    network_type TEXT NOT NULL DEFAULT 'mainnet',
    account_index BIGINT NOT NULL,
    qq_address TEXT NOT NULL,
    balance BIGINT NOT NULL,
    account TEXT NOT NULL,
    scalar TEXT NOT NULL,
    io_type_value INTEGER NOT NULL,
    on_chain BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(wallet_id, network_type, account_index)
);
INSERT INTO zk_accounts_backup (id, wallet_id, network_type, account_index, qq_address, balance, account, scalar, io_type_value, on_chain, created_at, updated_at)
    SELECT id, wallet_id, network_type, account_index, qq_address, balance, account, scalar, io_type_value, on_chain, created_at, updated_at FROM zk_accounts;
DROP TABLE zk_accounts;
ALTER TABLE zk_accounts_backup RENAME TO zk_accounts;
