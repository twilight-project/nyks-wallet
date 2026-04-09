CREATE TABLE IF NOT EXISTS btc_deposits (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT NOT NULL,
    network_type TEXT NOT NULL DEFAULT 'mainnet',
    btc_address TEXT NOT NULL,
    twilight_address TEXT NOT NULL,
    reserve_address TEXT,
    amount INTEGER NOT NULL,
    staking_amount INTEGER NOT NULL DEFAULT 10000,
    registration_tx_hash TEXT,
    status TEXT NOT NULL DEFAULT 'registered',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS btc_withdrawals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT NOT NULL,
    network_type TEXT NOT NULL DEFAULT 'mainnet',
    withdraw_address TEXT NOT NULL,
    twilight_address TEXT NOT NULL,
    reserve_id INTEGER NOT NULL,
    amount INTEGER NOT NULL,
    tx_hash TEXT,
    status TEXT NOT NULL DEFAULT 'submitted',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
