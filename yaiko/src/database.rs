use std::sync::Arc;

#[cfg(feature = "postgres")]
use sqlx::{Pool, Postgres};

#[cfg(feature = "sqlite")]
use sqlx::{Pool, Sqlite};

/// Database connection pool wrapper
/// 
/// Supports both PostgreSQL and SQLite depending on enabled features.
/// 
/// # Example (SQLite)
/// ```rust,no_run
/// # use yaiko_core::Database;
/// # async fn demo() -> Result<(), sqlx::Error> {
/// let db = Database::sqlite("sqlite:./app.db").await?;
/// # let _ = db;
/// # Ok(())
/// # }
/// ```
/// 
/// # Example (PostgreSQL)
/// ```rust,no_run
/// # #[cfg(feature = "postgres")]
/// # {
/// # use yaiko_core::Database;
/// # async fn demo() -> Result<(), sqlx::Error> {
/// let db = Database::postgres("postgres://user:pass@localhost/db").await?;
/// # let _ = db;
/// # Ok(())
/// # }
/// # }
/// ```
#[derive(Clone)]
pub enum Database {
    #[cfg(feature = "postgres")]
    Postgres(Arc<Pool<Postgres>>),
    #[cfg(feature = "sqlite")]
    Sqlite(Arc<Pool<Sqlite>>),
}

impl Database {
    /// Connect to a PostgreSQL database
    #[cfg(feature = "postgres")]
    pub async fn postgres(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        
        Ok(Database::Postgres(Arc::new(pool)))
    }

    /// Connect to a SQLite database
    #[cfg(feature = "sqlite")]
    pub async fn sqlite(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        
        Ok(Database::Sqlite(Arc::new(pool)))
    }

    /// Execute a raw SQL query and return affected rows
    #[cfg(feature = "postgres")]
    pub async fn execute_pg(&self, query: &str) -> Result<u64, sqlx::Error> {
        match self {
            Database::Postgres(pool) => {
                let result = sqlx::query(query).execute(&**pool).await?;
                Ok(result.rows_affected())
            }
            #[allow(unreachable_patterns)]
            _ => Err(sqlx::Error::Configuration("Wrong database type".into())),
        }
    }

    #[cfg(feature = "sqlite")]
    pub async fn execute_sqlite(&self, query: &str) -> Result<u64, sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                let result = sqlx::query(query).execute(&**pool).await?;
                Ok(result.rows_affected())
            }
            #[allow(unreachable_patterns)]
            _ => Err(sqlx::Error::Configuration("Wrong database type".into())),
        }
    }

    /// Get the underlying PostgreSQL pool
    #[cfg(feature = "postgres")]
    pub fn pg_pool(&self) -> Option<&Pool<Postgres>> {
        match self {
            Database::Postgres(pool) => Some(pool),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }

    /// Get the underlying SQLite pool
    #[cfg(feature = "sqlite")]
    pub fn sqlite_pool(&self) -> Option<&Pool<Sqlite>> {
        match self {
            Database::Sqlite(pool) => Some(pool),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }
}

/// Trait for bindable parameters
pub trait Bindable: Send + Sync {
    #[cfg(feature = "postgres")]
    fn bind_pg(&self, args: &mut sqlx::postgres::PgArguments);
    #[cfg(feature = "sqlite")]
    fn bind_sqlite(&self, args: &mut sqlx::sqlite::SqliteArguments<'_>);
}
