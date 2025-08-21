#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use diesel::prelude::*;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use diesel::r2d2::ConnectionManager;
use log::debug;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use r2d2::{Pool, PooledConnection};
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use std::sync::{Once, OnceLock};
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use std::{env, time::Duration};

#[cfg(feature = "sqlite")]
pub type DbConnection = diesel::SqliteConnection;

#[cfg(all(feature = "postgresql", not(feature = "sqlite")))]
pub type DbConnection = diesel::PgConnection;

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
pub type DbPool = Pool<ConnectionManager<DbConnection>>;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
pub type PooledConn = PooledConnection<ConnectionManager<DbConnection>>;

#[cfg(feature = "sqlite")]
mod sqlite_tuning {
    use diesel::prelude::*;
    use diesel::sql_query;
    use diesel::sql_types::Text;
    use diesel::sqlite::SqliteConnection;
    use r2d2::CustomizeConnection;

    #[derive(Debug)]
    pub struct SqlitePragmas;

    impl CustomizeConnection<SqliteConnection, diesel::r2d2::Error> for SqlitePragmas {
        fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
            // Fast + safe defaults for app-level pooling

            sql_query("PRAGMA foreign_keys=ON;")
                .execute(conn)
                .map_err(|e| diesel::r2d2::Error::QueryError(e.into()))?;
            sql_query("PRAGMA busy_timeout=5000;")
                .execute(conn)
                .map_err(|e| diesel::r2d2::Error::QueryError(e.into()))?;
            Ok(())
        }
    }
    #[derive(QueryableByName)]
    #[diesel(check_for_backend(diesel::sqlite::Sqlite))]
    struct JournalModeRow {
        #[diesel(sql_type = Text)]
        journal_mode: String,
    }

    pub fn set_persistent_sqlite_pragmas(conn: &mut SqliteConnection) -> Result<(), String> {
        // Read current journal_mode
        let current = sql_query("PRAGMA journal_mode;")
            .load::<JournalModeRow>(conn)
            .map_err(|e| format!("PRAGMA journal_mode read failed: {e}"))?
            .into_iter()
            .next()
            .map(|r| r.journal_mode.to_lowercase())
            .unwrap_or_default();

        if current != "wal" {
            sql_query("PRAGMA journal_mode=WAL;")
                .execute(conn)
                .map_err(|e| format!("PRAGMA journal_mode=WAL failed: {e}"))?;
        }

        sql_query("PRAGMA synchronous=NORMAL;")
            .execute(conn)
            .map_err(|e| format!("PRAGMA synchronous=NORMAL failed: {e}"))?;

        Ok(())
    }
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
fn database_url_from_env() -> Result<String, String> {
    #[cfg(feature = "sqlite")]
    {
        Ok(env::var("DATABASE_URL_SQLITE").unwrap_or_else(|_| "./wallet_data.db".to_string()))
    }
    #[cfg(all(feature = "postgresql", not(feature = "sqlite")))]
    {
        env::var("DATABASE_URL_POSTGRESQL")
            .map_err(|_| "DATABASE_URL environment variable is not set".to_string())
    }
}
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
pub fn init_pool(db_url: Option<String>) -> Result<DbPool, String> {
    let url: String = match db_url {
        Some(url) => url,
        None => {
            let url = database_url_from_env()?;
            url
        }
    };
    let manager = ConnectionManager::<DbConnection>::new(url.clone());
    #[cfg(feature = "sqlite")]
    {
        use sqlite_tuning::SqlitePragmas;
        debug!("Using SQLite database URL: {}", url);
        return Pool::builder()
            .max_size(15)
            .min_idle(Some(2))
            .connection_timeout(Duration::from_secs(8))
            .connection_customizer(Box::new(SqlitePragmas))
            .build(manager)
            .map_err(|e| format!("Failed to build DB pool: {e}"));
    }

    #[cfg(all(feature = "postgresql", not(feature = "sqlite")))]
    {
        debug!("Using PostgreSQL database URL: {}", url);
        return Pool::builder()
            .max_size(15)
            .min_idle(Some(2))
            .connection_timeout(Duration::from_secs(8))
            .build(manager)
            .map_err(|e| format!("Failed to build DB pool: {e}"));
    }
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
pub fn get_conn(pool: &DbPool) -> Result<PooledConn, String> {
    pool.get()
        .map_err(|e| format!("Failed to get pooled connection: {e}"))
}

// Ensure migrations run exactly once per process.
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
pub fn run_migrations_once(pool: &DbPool) -> Result<(), String> {
    static ONCE: Once = Once::new();
    static RESULT: OnceLock<Result<(), String>> = OnceLock::new();

    ONCE.call_once(|| {
        let res = (|| {
            let mut conn = get_conn(pool)?;
            run_migrations(&mut conn)
        })();
        let _ = RESULT.set(res);
    });

    RESULT.get().cloned().unwrap_or_else(|| Ok(()))
}

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
pub fn run_migrations(conn: &mut DbConnection) -> Result<(), String> {
    #[cfg(feature = "sqlite")]
    {
        use sqlite_tuning::set_persistent_sqlite_pragmas;
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
        set_persistent_sqlite_pragmas(conn)?;
    }

    #[cfg(all(feature = "postgresql", not(feature = "sqlite")))]
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
