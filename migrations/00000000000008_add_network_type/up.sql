-- Add network_type column to all tables.
-- SQLite does not support ADD CONSTRAINT, so for tables with unique constraints
-- we recreate the table with the new composite unique constraint.

-- 1. zk_accounts: UNIQUE(wallet_id, account_index) -> UNIQUE(wallet_id, network_type, account_index)
CREATE TABLE zk_accounts_new (
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
INSERT INTO zk_accounts_new (id, wallet_id, network_type, account_index, qq_address, balance, account, scalar, io_type_value, on_chain, created_at, updated_at)
    SELECT id, wallet_id, 'mainnet', account_index, qq_address, balance, account, scalar, io_type_value, on_chain, created_at, updated_at FROM zk_accounts;
DROP TABLE zk_accounts;
ALTER TABLE zk_accounts_new RENAME TO zk_accounts;

-- 2. encrypted_wallets: NO CHANGE — wallet identity is shared across networks

-- 3. order_wallets: UNIQUE(wallet_id) -> UNIQUE(wallet_id, network_type)
CREATE TABLE order_wallets_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT NOT NULL,
    network_type TEXT NOT NULL DEFAULT 'mainnet',
    chain_id TEXT NOT NULL,
    seed_encrypted BLOB NOT NULL,
    seed_salt BLOB NOT NULL,
    seed_nonce BLOB NOT NULL,
    relayer_api_endpoint TEXT NOT NULL,
    zkos_server_endpoint TEXT NOT NULL,
    relayer_program_json_path TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(wallet_id, network_type)
);
INSERT INTO order_wallets_new (id, wallet_id, network_type, chain_id, seed_encrypted, seed_salt, seed_nonce, relayer_api_endpoint, zkos_server_endpoint, relayer_program_json_path, is_active, created_at, updated_at)
    SELECT id, wallet_id, 'mainnet', chain_id, seed_encrypted, seed_salt, seed_nonce, relayer_api_endpoint, zkos_server_endpoint, relayer_program_json_path, is_active, created_at, updated_at FROM order_wallets;
DROP TABLE order_wallets;
ALTER TABLE order_wallets_new RENAME TO order_wallets;

-- 4. utxo_details: UNIQUE(wallet_id, account_index) -> UNIQUE(wallet_id, network_type, account_index)
CREATE TABLE utxo_details_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT NOT NULL,
    network_type TEXT NOT NULL DEFAULT 'mainnet',
    account_index BIGINT NOT NULL,
    utxo_data TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(wallet_id, network_type, account_index)
);
INSERT INTO utxo_details_new (id, wallet_id, network_type, account_index, utxo_data, created_at, updated_at)
    SELECT id, wallet_id, 'mainnet', account_index, utxo_data, created_at, updated_at FROM utxo_details;
DROP TABLE utxo_details;
ALTER TABLE utxo_details_new RENAME TO utxo_details;

-- 5. request_ids: UNIQUE(wallet_id, account_index) -> UNIQUE(wallet_id, network_type, account_index)
CREATE TABLE request_ids_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT NOT NULL,
    network_type TEXT NOT NULL DEFAULT 'mainnet',
    account_index BIGINT NOT NULL,
    request_id TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(wallet_id, network_type, account_index)
);
INSERT INTO request_ids_new (id, wallet_id, network_type, account_index, request_id, created_at, updated_at)
    SELECT id, wallet_id, 'mainnet', account_index, request_id, created_at, updated_at FROM request_ids;
DROP TABLE request_ids;
ALTER TABLE request_ids_new RENAME TO request_ids;

-- 6. order_history: append-only, no unique constraint to change
ALTER TABLE order_history ADD COLUMN network_type TEXT NOT NULL DEFAULT 'mainnet';

-- 7. transfer_history: append-only, no unique constraint to change
ALTER TABLE transfer_history ADD COLUMN network_type TEXT NOT NULL DEFAULT 'mainnet';
