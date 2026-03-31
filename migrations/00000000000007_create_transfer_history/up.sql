CREATE TABLE IF NOT EXISTS transfer_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT NOT NULL,
    direction TEXT NOT NULL,
    from_index INTEGER,
    to_index INTEGER,
    amount INTEGER NOT NULL,
    tx_hash TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
