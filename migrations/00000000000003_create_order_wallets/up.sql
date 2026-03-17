CREATE TABLE IF NOT EXISTS order_wallets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT UNIQUE NOT NULL,
    chain_id TEXT NOT NULL,
    seed_encrypted BLOB NOT NULL,
    seed_salt BLOB NOT NULL,
    seed_nonce BLOB NOT NULL,
    relayer_api_endpoint TEXT NOT NULL,
    zkos_server_endpoint TEXT NOT NULL,
    relayer_program_json_path TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
