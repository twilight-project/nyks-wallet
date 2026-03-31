#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use diesel::r2d2::ConnectionManager;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use log::debug;
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use r2d2::{Pool, PooledConnection};
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use std::sync::{Once, OnceLock};
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
use std::{env, time::Duration};

#[cfg(any(feature = "sqlite", feature = "postgresql"))]
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

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
            .max_size(4)
            .min_idle(Some(1))
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
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| format!("Failed to run migrations: {}", e))?;

    #[cfg(feature = "sqlite")]
    sqlite_tuning::set_persistent_sqlite_pragmas(conn)?;

    Ok(())
}
