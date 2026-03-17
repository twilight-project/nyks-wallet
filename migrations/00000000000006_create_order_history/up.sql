CREATE TABLE IF NOT EXISTS order_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT NOT NULL,
    account_index INTEGER NOT NULL,
    request_id TEXT NOT NULL,
    action TEXT NOT NULL,
    order_type TEXT NOT NULL,
    position_type TEXT,
    amount INTEGER NOT NULL,
    price REAL,
    leverage INTEGER,
    pnl REAL,
    status TEXT NOT NULL,
    tx_hash TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
