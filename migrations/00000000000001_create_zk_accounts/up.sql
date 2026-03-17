CREATE TABLE IF NOT EXISTS zk_accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT NOT NULL,
    account_index INTEGER NOT NULL,
    qq_address TEXT NOT NULL,
    balance INTEGER NOT NULL,
    account TEXT NOT NULL,
    scalar TEXT NOT NULL,
    io_type_value INTEGER NOT NULL,
    on_chain BOOLEAN NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(wallet_id, account_index)
);
