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
const NOTES_SCHEMA: &str = include_str!("../fixtures/notes/notes.schema.sql");
const NOTES_SEED: &str = include_str!("../fixtures/notes/seed.sql");

const SEED_HANDLE_ID: &str = "+15551234567";
const SEED_CHAT_GUID: &str = "fixture-chat-guid";
const SEED_MESSAGE_GUID: &str = "fixture-message-guid";

pub const SEED_NOTES_ACCOUNT_ID: &str = "fixture-notes-account-0001-0000-000000000001";
pub const SEED_NOTES_FOLDER_ID: &str = "11111111-1111-1111-1111-111111111111";
pub const SEED_PROJECTS_FOLDER_ID: &str = "22222222-2222-2222-2222-222222222222";
pub const SEED_DELETED_FOLDER_ID: &str = "33333333-3333-3333-3333-333333333333";
pub const SEED_PLAIN_TEXT_NOTE_ID: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
pub const SEED_CHECKLIST_NOTE_ID: &str = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
pub const SEED_LOCKED_NOTE_ID: &str = "cccccccc-cccc-cccc-cccc-cccccccccccc";
pub const SEED_SUMMARY_NOTE_ID: &str = "dddddddd-dddd-dddd-dddd-dddddddddddd";
pub const SEED_ATTACHMENT_NOTE_ID: &str = "eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee";
pub const SEED_ATTACHMENT_ID: &str = "ffffffff-ffff-ffff-ffff-ffffffffffff";

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

pub struct NotesFixtureDb {
    _temp_dir: TempDir,
    path: PathBuf,
}

impl NotesFixtureDb {
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
        let path = temp_dir.path().join("notes.db");
        apply_notes_schema(&path, seed).await?;
        Ok(Self {
            _temp_dir: temp_dir,
            path,
        })
    }
}

async fn apply_notes_schema(path: &Path, seed: bool) -> io::Result<()> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);

    let mut connection = SqliteConnection::connect_with(&options)
        .await
        .map_err(io::Error::other)?;

    sqlx::raw_sql(NOTES_SCHEMA)
        .execute(&mut connection)
        .await
        .map_err(io::Error::other)?;

    if seed {
        sqlx::raw_sql(NOTES_SEED)
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

    use super::{
        FixtureDb, NotesFixtureDb, RemindersFixtureDb, SEED_CHAT_GUID, SEED_CHECKLIST_NOTE_ID,
        SEED_HANDLE_ID, SEED_LOCKED_NOTE_ID, SEED_MESSAGE_GUID, SEED_NOTES_FOLDER_ID,
        SEED_PLAIN_TEXT_NOTE_ID,
    };

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

    #[tokio::test]
    async fn empty_notes_fixture_loads_schema() {
        let fixture = NotesFixtureDb::empty().await.expect("empty notes fixture");
        let options = SqliteConnectOptions::new()
            .filename(fixture.path())
            .read_only(true);
        let pool = sqlx::SqlitePool::connect_with(options)
            .await
            .expect("connect read-only pool");

        let table_count: i64 = sqlx::query(
            "SELECT COUNT(*) AS count FROM sqlite_master WHERE name = 'ZICCLOUDSYNCINGOBJECT'",
        )
        .fetch_one(&pool)
        .await
        .expect("table count")
        .get("count");

        assert_eq!(table_count, 1);
    }

    #[tokio::test]
    async fn seeded_notes_fixture_contains_expected_rows() {
        let fixture = NotesFixtureDb::seeded()
            .await
            .expect("seeded notes fixture");
        let options = SqliteConnectOptions::new()
            .filename(fixture.path())
            .read_only(true);
        let pool = sqlx::SqlitePool::connect_with(options)
            .await
            .expect("connect read-only pool");

        let folder_count: i64 = sqlx::query(
            "SELECT COUNT(*) AS count FROM ZICCLOUDSYNCINGOBJECT \
             WHERE Z_ENT = 15 AND ZIDENTIFIER = ?1",
        )
        .bind(SEED_NOTES_FOLDER_ID)
        .fetch_one(&pool)
        .await
        .expect("folder count")
        .get("count");

        let note_count: i64 = sqlx::query(
            "SELECT COUNT(*) AS count FROM ZICCLOUDSYNCINGOBJECT \
             WHERE Z_ENT = 12 AND ZMARKEDFORDELETION = 0",
        )
        .fetch_one(&pool)
        .await
        .expect("note count")
        .get("count");

        let locked_count: i64 = sqlx::query(
            "SELECT COUNT(*) AS count FROM ZICCLOUDSYNCINGOBJECT \
             WHERE Z_ENT = 12 AND ZIDENTIFIER = ?1 AND ZISPASSWORDPROTECTED = 1",
        )
        .bind(SEED_LOCKED_NOTE_ID)
        .fetch_one(&pool)
        .await
        .expect("locked count")
        .get("count");

        let checklist_count: i64 = sqlx::query(
            "SELECT COUNT(*) AS count FROM ZICCLOUDSYNCINGOBJECT \
             WHERE Z_ENT = 12 AND ZIDENTIFIER = ?1 AND ZHASCHECKLIST = 1",
        )
        .bind(SEED_CHECKLIST_NOTE_ID)
        .fetch_one(&pool)
        .await
        .expect("checklist count")
        .get("count");

        let body_count: i64 = sqlx::query(
            "SELECT COUNT(*) AS count FROM ZICNOTEDATA nd \
             JOIN ZICCLOUDSYNCINGOBJECT n ON nd.ZNOTE = n.Z_PK \
             WHERE n.ZIDENTIFIER = ?1 AND length(nd.ZDATA) > 0",
        )
        .bind(SEED_PLAIN_TEXT_NOTE_ID)
        .fetch_one(&pool)
        .await
        .expect("body count")
        .get("count");

        let hashtag_count: i64 = sqlx::query(
            "SELECT COUNT(*) AS count FROM ZICCLOUDSYNCINGOBJECT \
             WHERE ZTYPEUTI1 = 'com.apple.notes.inlinetextattachment.hashtag' \
               AND ZNOTE1 = 6 AND ZALTTEXT = '#reading'",
        )
        .fetch_one(&pool)
        .await
        .expect("hashtag count")
        .get("count");

        assert_eq!(folder_count, 1);
        assert!(note_count >= 5);
        assert_eq!(locked_count, 1);
        assert_eq!(checklist_count, 1);
        assert_eq!(body_count, 1);
        assert_eq!(hashtag_count, 1);
    }
}
