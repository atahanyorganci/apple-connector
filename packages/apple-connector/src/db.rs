use std::{
    io::{Error as IoError, ErrorKind},
    path::Path,
    time::Duration,
};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use tracing::warn;

const MAX_CONNECTIONS: u32 = 5;
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Upper bound for individual read queries against `chat.db`.
pub const QUERY_TIMEOUT: Duration = Duration::from_secs(15);

pub async fn run_timed_query<T, F, Fut>(query: F) -> Result<T, sqlx::Error>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
{
    match tokio::time::timeout(QUERY_TIMEOUT, query()).await {
        Ok(result) => result,
        Err(_) => Err(sqlx::Error::PoolTimedOut),
    }
}

#[derive(Debug)]
pub enum DatabaseError {
    NotFound,
    PermissionDenied,
    Connect(String),
}

impl DatabaseError {
    pub fn startup_message(&self, path: &Path) -> String {
        match self {
            Self::NotFound => format!("Messages database not found at {}", path.display()),
            Self::PermissionDenied => format!(
                "Could not open {}. Grant Full Disk Access to this terminal and try again.",
                path.display()
            ),
            Self::Connect(message) => format!("Could not open {}: {message}", path.display()),
        }
    }

    fn from_sqlx(path: &Path, error: sqlx::Error) -> Self {
        match &error {
            sqlx::Error::Database(db_error) if db_error.code().as_deref() == Some("14") => {
                Self::PermissionDenied
            }
            sqlx::Error::Io(io_error) if io_error.kind() == ErrorKind::PermissionDenied => {
                Self::PermissionDenied
            }
            _ if error.to_string().contains("unable to open database file") => {
                if path.is_file() {
                    Self::PermissionDenied
                } else {
                    Self::NotFound
                }
            }
            _ => Self::Connect(error.to_string()),
        }
    }
}

pub fn ensure_database_exists(path: &Path) -> Result<(), DatabaseError> {
    if path.is_file() {
        Ok(())
    } else {
        Err(DatabaseError::NotFound)
    }
}

pub async fn connect_pool(path: &Path) -> Result<SqlitePool, DatabaseError> {
    ensure_database_exists(path)?;

    let options = SqliteConnectOptions::new()
        .filename(path)
        .read_only(true)
        .create_if_missing(false)
        .busy_timeout(BUSY_TIMEOUT);

    SqlitePoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .acquire_timeout(ACQUIRE_TIMEOUT)
        .connect_with(options)
        .await
        .map_err(|error| DatabaseError::from_sqlx(path, error))
}

pub(crate) async fn is_pool_healthy(pool: &SqlitePool) -> bool {
    match pool.acquire().await {
        Ok(mut connection) => sqlx::query_scalar!("SELECT 1")
            .fetch_one(&mut *connection)
            .await
            .is_ok(),
        Err(error) => {
            warn!(%error, "database health check failed to acquire connection");
            false
        }
    }
}

pub fn database_open_failure(error: &DatabaseError, path: &Path) -> IoError {
    IoError::new(
        match error {
            DatabaseError::NotFound => ErrorKind::NotFound,
            DatabaseError::PermissionDenied => ErrorKind::PermissionDenied,
            DatabaseError::Connect(_) => ErrorKind::Other,
        },
        error.startup_message(path),
    )
}

#[cfg(test)]
mod tests {
    use super::{connect_pool, ensure_database_exists, is_pool_healthy};
    use crate::fixtures::FixtureDb;

    #[tokio::test]
    async fn read_only_pool_can_run_health_query() {
        let fixture = FixtureDb::empty().await.expect("fixture database");
        let pool = connect_pool(fixture.path())
            .await
            .expect("connect read-only pool");

        assert!(is_pool_healthy(&pool).await);
    }

    #[tokio::test]
    async fn missing_database_is_reported_without_creating_file() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let path = temp_dir.path().join("missing-chat.db");

        assert!(matches!(
            ensure_database_exists(&path),
            Err(super::DatabaseError::NotFound)
        ));
        assert!(!path.exists());
    }
}
