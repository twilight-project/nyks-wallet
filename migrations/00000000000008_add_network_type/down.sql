-- Reverse: remove network_type column from all tables

-- 1. zk_accounts
CREATE TABLE zk_accounts_old (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT NOT NULL,
    account_index BIGINT NOT NULL,
    qq_address TEXT NOT NULL,
    balance BIGINT NOT NULL,
    account TEXT NOT NULL,
    scalar TEXT NOT NULL,
    io_type_value INTEGER NOT NULL,
    on_chain BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(wallet_id, account_index)
);
INSERT INTO zk_accounts_old (id, wallet_id, account_index, qq_address, balance, account, scalar, io_type_value, on_chain, created_at, updated_at)
    SELECT id, wallet_id, account_index, qq_address, balance, account, scalar, io_type_value, on_chain, created_at, updated_at FROM zk_accounts;
DROP TABLE zk_accounts;
ALTER TABLE zk_accounts_old RENAME TO zk_accounts;

-- 2. encrypted_wallets: NO CHANGE — was not modified in up.sql

-- 3. order_wallets
CREATE TABLE order_wallets_old (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT NOT NULL UNIQUE,
    chain_id TEXT NOT NULL,
    seed_encrypted BLOB NOT NULL,
    seed_salt BLOB NOT NULL,
    seed_nonce BLOB NOT NULL,
    relayer_api_endpoint TEXT NOT NULL,
    zkos_server_endpoint TEXT NOT NULL,
    relayer_program_json_path TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
INSERT INTO order_wallets_old (id, wallet_id, chain_id, seed_encrypted, seed_salt, seed_nonce, relayer_api_endpoint, zkos_server_endpoint, relayer_program_json_path, is_active, created_at, updated_at)
    SELECT id, wallet_id, chain_id, seed_encrypted, seed_salt, seed_nonce, relayer_api_endpoint, zkos_server_endpoint, relayer_program_json_path, is_active, created_at, updated_at FROM order_wallets;
DROP TABLE order_wallets;
ALTER TABLE order_wallets_old RENAME TO order_wallets;

-- 4. utxo_details
CREATE TABLE utxo_details_old (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT NOT NULL,
    account_index BIGINT NOT NULL,
    utxo_data TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(wallet_id, account_index)
);
INSERT INTO utxo_details_old (id, wallet_id, account_index, utxo_data, created_at, updated_at)
    SELECT id, wallet_id, account_index, utxo_data, created_at, updated_at FROM utxo_details;
DROP TABLE utxo_details;
ALTER TABLE utxo_details_old RENAME TO utxo_details;

-- 5. request_ids
CREATE TABLE request_ids_old (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT NOT NULL,
    account_index BIGINT NOT NULL,
    request_id TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(wallet_id, account_index)
);
INSERT INTO request_ids_old (id, wallet_id, account_index, request_id, created_at, updated_at)
    SELECT id, wallet_id, account_index, request_id, created_at, updated_at FROM request_ids;
DROP TABLE request_ids;
ALTER TABLE request_ids_old RENAME TO request_ids;

-- 6 & 7: SQLite cannot drop columns, so we recreate
CREATE TABLE order_history_old (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT NOT NULL,
    account_index BIGINT NOT NULL,
    request_id TEXT NOT NULL,
    action TEXT NOT NULL,
    order_type TEXT NOT NULL,
    position_type TEXT,
    amount BIGINT NOT NULL,
    price DOUBLE,
    leverage BIGINT,
    pnl DOUBLE,
    status TEXT NOT NULL,
    tx_hash TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
INSERT INTO order_history_old SELECT id, wallet_id, account_index, request_id, action, order_type, position_type, amount, price, leverage, pnl, status, tx_hash, created_at FROM order_history;
DROP TABLE order_history;
ALTER TABLE order_history_old RENAME TO order_history;

CREATE TABLE transfer_history_old (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT NOT NULL,
    direction TEXT NOT NULL,
    from_index BIGINT,
    to_index BIGINT,
    amount BIGINT NOT NULL,
    tx_hash TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
INSERT INTO transfer_history_old SELECT id, wallet_id, direction, from_index, to_index, amount, tx_hash, created_at FROM transfer_history;
DROP TABLE transfer_history;
ALTER TABLE transfer_history_old RENAME TO transfer_history;
