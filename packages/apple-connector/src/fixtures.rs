use std::{
    io,
    path::{Path, PathBuf},
};

use sqlx::{
    Connection,
    sqlite::{SqliteConnectOptions, SqliteConnection},
};
use tempfile::TempDir;

const CHAT_SCHEMA: &str = include_str!("../fixtures/messages/chat.schema.sql");
const REMINDERS_SCHEMA: &str = include_str!("../fixtures/reminders/reminders.schema.sql");
const REMINDERS_SEED: &str = include_str!("../fixtures/reminders/seed.sql");

const SEED_HANDLE_ID: &str = "+15551234567";
const SEED_CHAT_GUID: &str = "fixture-chat-guid";
const SEED_MESSAGE_GUID: &str = "fixture-message-guid";

pub struct FixtureDb {
    _temp_dir: TempDir,
    path: PathBuf,
}

impl FixtureDb {
    pub async fn empty() -> io::Result<Self> {
        Self::with_seed(false).await
    }

    pub async fn seeded() -> io::Result<Self> {
        Self::with_seed(true).await
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    async fn with_seed(seed: bool) -> io::Result<Self> {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("chat.db");
        apply_schema(&path, seed).await?;
        Ok(Self {
            _temp_dir: temp_dir,
            path,
        })
    }
}

async fn apply_schema(path: &Path, seed: bool) -> io::Result<()> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);

    let mut connection = SqliteConnection::connect_with(&options)
        .await
        .map_err(io::Error::other)?;

    sqlx::raw_sql(CHAT_SCHEMA)
        .execute(&mut connection)
        .await
        .map_err(io::Error::other)?;

    if seed {
        disable_apple_triggers(&mut connection)
            .await
            .map_err(io::Error::other)?;
        seed_data(&mut connection).await.map_err(io::Error::other)?;
    }

    connection.close().await.ok();
    Ok(())
}

async fn disable_apple_triggers(connection: &mut SqliteConnection) -> sqlx::Result<()> {
    sqlx::query("DROP TRIGGER IF EXISTS verify_chat_insert")
        .execute(&mut *connection)
        .await?;
    sqlx::query("DROP TRIGGER IF EXISTS verify_chat_update")
        .execute(&mut *connection)
        .await?;
    Ok(())
}

async fn seed_data(connection: &mut SqliteConnection) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO handle (id, service) VALUES (?1, 'iMessage')")
        .bind(SEED_HANDLE_ID)
        .execute(&mut *connection)
        .await?;

    sqlx::query(
        "INSERT INTO chat (guid, style, chat_identifier, service_name) VALUES (?1, 45, ?2, 'iMessage')",
    )
    .bind(SEED_CHAT_GUID)
    .bind(SEED_HANDLE_ID)
    .execute(&mut *connection)
    .await?;

    sqlx::query(
        "INSERT INTO message (guid, text, service, is_from_me) VALUES (?1, 'fixture seed message', 'iMessage', 1)",
    )
    .bind(SEED_MESSAGE_GUID)
    .execute(&mut *connection)
    .await?;

    sqlx::query(
        "INSERT INTO chat_message_join (chat_id, message_id, message_date) \
         SELECT chat.ROWID, message.ROWID, 0 FROM chat, message \
         WHERE chat.guid = ?1 AND message.guid = ?2",
    )
    .bind(SEED_CHAT_GUID)
    .bind(SEED_MESSAGE_GUID)
    .execute(&mut *connection)
    .await?;

    Ok(())
}

pub struct RemindersFixtureDb {
    _temp_dir: TempDir,
    path: PathBuf,
}

impl RemindersFixtureDb {
    pub async fn empty() -> io::Result<Self> {
        Self::with_seed(false).await
    }

    pub async fn seeded() -> io::Result<Self> {
        Self::with_seed(true).await
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    async fn with_seed(seed: bool) -> io::Result<Self> {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("reminders.db");
        apply_reminders_schema(&path, seed).await?;
        Ok(Self {
            _temp_dir: temp_dir,
            path,
        })
    }
}

async fn apply_reminders_schema(path: &Path, seed: bool) -> io::Result<()> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);

    let mut connection = SqliteConnection::connect_with(&options)
        .await
        .map_err(io::Error::other)?;

    sqlx::raw_sql(REMINDERS_SCHEMA)
        .execute(&mut connection)
        .await
        .map_err(io::Error::other)?;

    if seed {
        sqlx::raw_sql(REMINDERS_SEED)
            .execute(&mut connection)
            .await
            .map_err(io::Error::other)?;
    }

    connection.close().await.ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::{Row, sqlite::SqliteConnectOptions};

    use super::{FixtureDb, RemindersFixtureDb, SEED_CHAT_GUID, SEED_HANDLE_ID, SEED_MESSAGE_GUID};

    #[tokio::test]
    async fn empty_fixture_is_deterministic() {
        let first = FixtureDb::empty().await.expect("first fixture");
        let second = FixtureDb::empty().await.expect("second fixture");

        assert_ne!(first.path(), second.path());
        assert!(first.path().is_file());
        assert!(second.path().is_file());
    }

    #[tokio::test]
    async fn seeded_fixture_contains_expected_rows() {
        let fixture = FixtureDb::seeded().await.expect("seeded fixture");
        let options = SqliteConnectOptions::new()
            .filename(fixture.path())
            .read_only(true);
        let pool = sqlx::SqlitePool::connect_with(options)
            .await
            .expect("connect read-only pool");

        let handle_count: i64 = sqlx::query("SELECT COUNT(*) AS count FROM handle WHERE id = ?1")
            .bind(SEED_HANDLE_ID)
            .fetch_one(&pool)
            .await
            .expect("handle count")
            .get("count");

        let chat_count: i64 = sqlx::query("SELECT COUNT(*) AS count FROM chat WHERE guid = ?1")
            .bind(SEED_CHAT_GUID)
            .fetch_one(&pool)
            .await
            .expect("chat count")
            .get("count");

        let message_count: i64 =
            sqlx::query("SELECT COUNT(*) AS count FROM message WHERE guid = ?1")
                .bind(SEED_MESSAGE_GUID)
                .fetch_one(&pool)
                .await
                .expect("message count")
                .get("count");

        assert_eq!(handle_count, 1);
        assert_eq!(chat_count, 1);
        assert_eq!(message_count, 1);
    }

    #[tokio::test]
    async fn empty_reminders_fixture_loads_schema() {
        let fixture = RemindersFixtureDb::empty()
            .await
            .expect("empty reminders fixture");
        let options = SqliteConnectOptions::new()
            .filename(fixture.path())
            .read_only(true);
        let pool = sqlx::SqlitePool::connect_with(options)
            .await
            .expect("connect read-only pool");

        let table_count: i64 = sqlx::query(
            "SELECT COUNT(*) AS count FROM sqlite_master WHERE name = 'ZREMCDREMINDER'",
        )
        .fetch_one(&pool)
        .await
        .expect("table count")
        .get("count");

        assert_eq!(table_count, 1);
    }

    #[tokio::test]
    async fn seeded_reminders_fixture_contains_expected_rows() {
        let fixture = RemindersFixtureDb::seeded()
            .await
            .expect("seeded reminders fixture");
        let options = SqliteConnectOptions::new()
            .filename(fixture.path())
            .read_only(true);
        let pool = sqlx::SqlitePool::connect_with(options)
            .await
            .expect("connect read-only pool");

        let reminder_count: i64 = sqlx::query(
            "SELECT COUNT(*) AS count FROM ZREMCDREMINDER WHERE ZMARKEDFORDELETION = 0",
        )
        .fetch_one(&pool)
        .await
        .expect("reminder count")
        .get("count");

        assert!(reminder_count >= 2);
    }
}
