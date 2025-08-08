#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use diesel::prelude::*;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use std::env;

#[cfg(all(feature = "sqlite", not(feature = "postgresql")))]
pub type DbConnection = diesel::SqliteConnection;

#[cfg(feature = "postgresql")]
pub type DbConnection = diesel::PgConnection;

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
pub fn establish_connection() -> Result<DbConnection, String> {
    #[cfg(all(feature = "sqlite", not(feature = "postgresql")))]
    {
        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "./wallet_data.db".to_string());

        SqliteConnection::establish(&database_url)
            .map_err(|e| format!("Error connecting to SQLite database: {}", e))
    }

    #[cfg(feature = "postgresql")]
    {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL must be set for PostgreSQL".to_string())?;

        PgConnection::establish(&database_url)
            .map_err(|e| format!("Error connecting to PostgreSQL database: {}", e))
    }
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
pub fn run_migrations(conn: &mut DbConnection) -> Result<(), String> {
    #[cfg(all(feature = "sqlite", not(feature = "postgresql")))]
    {
        // SQLite migrations
        diesel::sql_query(
            r#"
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
            "#,
        )
        .execute(conn)
        .map_err(|e| format!("Failed to create zk_accounts table: {}", e))?;

        diesel::sql_query(
            r#"
            CREATE TABLE IF NOT EXISTS encrypted_wallets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                wallet_id TEXT UNIQUE NOT NULL,
                encrypted_data BLOB NOT NULL,
                salt BLOB NOT NULL,
                nonce BLOB NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(conn)
        .map_err(|e| format!("Failed to create encrypted_wallets table: {}", e))?;

        diesel::sql_query(
            r#"
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
            "#,
        )
        .execute(conn)
        .map_err(|e| format!("Failed to create order_wallets table: {}", e))?;

        diesel::sql_query(
            r#"
            CREATE TABLE IF NOT EXISTS utxo_details (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                wallet_id TEXT NOT NULL,
                account_index INTEGER NOT NULL,
                utxo_data TEXT NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(wallet_id, account_index)
            );
            "#,
        )
        .execute(conn)
        .map_err(|e| format!("Failed to create utxo_details table: {}", e))?;

        diesel::sql_query(
            r#"
            CREATE TABLE IF NOT EXISTS request_ids (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                wallet_id TEXT NOT NULL,
                account_index INTEGER NOT NULL,
                request_id TEXT NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(wallet_id, account_index)
            );
            "#,
        )
        .execute(conn)
        .map_err(|e| format!("Failed to create request_ids table: {}", e))?;
    }

    #[cfg(feature = "postgresql")]
    {
        // PostgreSQL migrations
        diesel::sql_query(
            r#"
            CREATE TABLE IF NOT EXISTS zk_accounts (
                id SERIAL PRIMARY KEY,
                wallet_id VARCHAR NOT NULL,
                account_index BIGINT NOT NULL,
                qq_address VARCHAR NOT NULL,
                balance BIGINT NOT NULL,
                account VARCHAR NOT NULL,
                scalar VARCHAR NOT NULL,
                io_type_value INTEGER NOT NULL,
                on_chain BOOLEAN NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
                UNIQUE(wallet_id, account_index)
            );
            "#,
        )
        .execute(conn)
        .map_err(|e| format!("Failed to create zk_accounts table: {}", e))?;

        diesel::sql_query(
            r#"
            CREATE TABLE IF NOT EXISTS encrypted_wallets (
                id SERIAL PRIMARY KEY,
                wallet_id VARCHAR UNIQUE NOT NULL,
                encrypted_data BYTEA NOT NULL,
                salt BYTEA NOT NULL,
                nonce BYTEA NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMP NOT NULL DEFAULT NOW()
            );
            "#,
        )
        .execute(conn)
        .map_err(|e| format!("Failed to create encrypted_wallets table: {}", e))?;

        diesel::sql_query(
            r#"
            CREATE TABLE IF NOT EXISTS order_wallets (
                id SERIAL PRIMARY KEY,
                wallet_id VARCHAR UNIQUE NOT NULL,
                chain_id VARCHAR NOT NULL,
                seed_encrypted BYTEA NOT NULL,
                seed_salt BYTEA NOT NULL,
                seed_nonce BYTEA NOT NULL,
                relayer_api_endpoint VARCHAR NOT NULL,
                zkos_server_endpoint VARCHAR NOT NULL,
                relayer_program_json_path VARCHAR NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT TRUE,
                created_at TIMESTAMP NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMP NOT NULL DEFAULT NOW()
            );
            "#,
        )
        .execute(conn)
        .map_err(|e| format!("Failed to create order_wallets table: {}", e))?;

        diesel::sql_query(
            r#"
            CREATE TABLE IF NOT EXISTS utxo_details (
                id SERIAL PRIMARY KEY,
                wallet_id VARCHAR NOT NULL,
                account_index BIGINT NOT NULL,
                utxo_data TEXT NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
                UNIQUE(wallet_id, account_index)
            );
            "#,
        )
        .execute(conn)
        .map_err(|e| format!("Failed to create utxo_details table: {}", e))?;

        diesel::sql_query(
            r#"
            CREATE TABLE IF NOT EXISTS request_ids (
                id SERIAL PRIMARY KEY,
                wallet_id VARCHAR NOT NULL,
                account_index BIGINT NOT NULL,
                request_id VARCHAR NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
                UNIQUE(wallet_id, account_index)
            );
            "#,
        )
        .execute(conn)
        .map_err(|e| format!("Failed to create request_ids table: {}", e))?;
    }

    Ok(())
}
