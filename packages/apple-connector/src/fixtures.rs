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
const CALENDAR_SCHEMA: &str = include_str!("../fixtures/calendar/calendar.schema.sql");
const CALENDAR_SEED: &str = include_str!("../fixtures/calendar/seed.sql");
const CONTACTS_SCHEMA: &str = include_str!("../fixtures/contacts/contacts.schema.sql");
const CONTACTS_SEED: &str = include_str!("../fixtures/contacts/seed.sql");

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

pub const SEED_CALENDAR_ACCOUNT_ID: &str = "store-icloud";
pub const SEED_CALENDAR_ID: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
pub const SEED_EVENT_ID: &str = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
pub const SEED_RECURRING_EVENT_ID: &str = "cccccccc-cccc-cccc-cccc-cccccccccccc";
pub const SEED_EVENT_ATTACHMENT_ID: &str = "dddddddd-dddd-dddd-dddd-dddddddddddd";

pub const SEED_CONTAINER_ID: &str = "11111111-1111-1111-1111-111111111111";
pub const SEED_GROUP_ID: &str = "22222222-2222-2222-2222-222222222222";
pub const SEED_CONTACT_ID: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";

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
    sqlx::query!("DROP TRIGGER IF EXISTS verify_chat_insert")
        .execute(&mut *connection)
        .await?;
    sqlx::query!("DROP TRIGGER IF EXISTS verify_chat_update")
        .execute(&mut *connection)
        .await?;
    Ok(())
}

async fn seed_data(connection: &mut SqliteConnection) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO handle (id, service) VALUES (?1, 'iMessage')",
        SEED_HANDLE_ID
    )
    .execute(&mut *connection)
    .await?;

    sqlx::query!(
        "INSERT INTO chat (guid, style, chat_identifier, service_name) VALUES (?1, 45, ?2, 'iMessage')",
        SEED_CHAT_GUID,
        SEED_HANDLE_ID
    )
    .execute(&mut *connection)
    .await?;

    sqlx::query!(
        "INSERT INTO message (guid, text, service, is_from_me) VALUES (?1, 'fixture seed message', 'iMessage', 1)",
        SEED_MESSAGE_GUID
    )
    .execute(&mut *connection)
    .await?;

    sqlx::query!(
        "INSERT INTO chat_message_join (chat_id, message_id, message_date) \
         SELECT chat.ROWID, message.ROWID, 0 FROM chat, message \
         WHERE chat.guid = ?1 AND message.guid = ?2",
        SEED_CHAT_GUID,
        SEED_MESSAGE_GUID
    )
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

pub struct CalendarFixtureDb {
    _temp_dir: TempDir,
    path: PathBuf,
}

impl CalendarFixtureDb {
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
        let path = temp_dir.path().join("Calendar.sqlitedb");
        apply_calendar_schema(&path, seed).await?;
        Ok(Self {
            _temp_dir: temp_dir,
            path,
        })
    }
}

async fn apply_calendar_schema(path: &Path, seed: bool) -> io::Result<()> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);

    let mut connection = SqliteConnection::connect_with(&options)
        .await
        .map_err(io::Error::other)?;

    sqlx::raw_sql(CALENDAR_SCHEMA)
        .execute(&mut connection)
        .await
        .map_err(io::Error::other)?;

    if seed {
        sqlx::raw_sql(CALENDAR_SEED)
            .execute(&mut connection)
            .await
            .map_err(io::Error::other)?;
    }

    connection.close().await.ok();
    Ok(())
}

pub struct ContactsFixtureDb {
    _temp_dir: TempDir,
    path: PathBuf,
}

impl ContactsFixtureDb {
    pub async fn empty() -> io::Result<Self> {
        Self::with_seed(false).await
    }

    pub async fn seeded() -> io::Result<Self> {
        Self::with_seed(true).await
    }

    pub async fn seeded_with_batch_contacts(count: u32) -> io::Result<Self> {
        let fixture = Self::seeded().await?;
        seed_extra_contacts(fixture.path(), count).await?;
        Ok(fixture)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    async fn with_seed(seed: bool) -> io::Result<Self> {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("AddressBook-v22.abcddb");
        apply_contacts_schema(&path, seed).await?;
        Ok(Self {
            _temp_dir: temp_dir,
            path,
        })
    }
}

async fn apply_contacts_schema(path: &Path, seed: bool) -> io::Result<()> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);

    let mut connection = SqliteConnection::connect_with(&options)
        .await
        .map_err(io::Error::other)?;

    sqlx::raw_sql(CONTACTS_SCHEMA)
        .execute(&mut connection)
        .await
        .map_err(io::Error::other)?;

    if seed {
        sqlx::raw_sql(CONTACTS_SEED)
            .execute(&mut connection)
            .await
            .map_err(io::Error::other)?;
    }

    connection.close().await.ok();
    Ok(())
}

/// Inserts additional contact rows for batch-hydration tests.
pub async fn seed_extra_contacts(path: &Path, count: u32) -> io::Result<()> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .read_only(false)
        .create_if_missing(false);

    let mut connection = SqliteConnection::connect_with(&options)
        .await
        .map_err(io::Error::other)?;

    for index in 0..count {
        let pk = 100_i64 + i64::from(index);
        let uuid = format!("{:08x}-0000-0000-0000-{index:012x}", index);
        sqlx::query(
            "INSERT INTO ZABCDRECORD (Z_PK, Z_ENT, Z_OPT, ZCONTAINER, ZFIRSTNAME, ZUNIQUEID, ZCREATIONDATE, ZMODIFICATIONDATE) VALUES (?1, 22, 1, 1, ?2, ?3, 1700000000, 1700000000)",
        )
        .bind(pk)
        .bind(format!("Person{index}"))
        .bind(format!("{uuid}:ABContact"))
        .execute(&mut connection)
        .await
        .map_err(io::Error::other)?;
    }

    connection.close().await.ok();
    Ok(())
}

/// Runs `EXPLAIN QUERY PLAN` for dynamic SQL (test helper only).
pub async fn explain_query_plan(
    pool: &sqlx::SqlitePool,
    sql: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let explain_sql = format!("EXPLAIN QUERY PLAN {sql}");
    let rows: Vec<(i64, i64, i64, String)> = sqlx::query_as(sqlx::AssertSqlSafe(explain_sql))
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|(_, _, _, detail)| detail).collect())
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqliteConnectOptions;

    use super::{
        FixtureDb, NotesFixtureDb, RemindersFixtureDb, SEED_CHAT_GUID, SEED_CHECKLIST_NOTE_ID,
        SEED_HANDLE_ID, SEED_LOCKED_NOTE_ID, SEED_MESSAGE_GUID, SEED_NOTES_FOLDER_ID,
        SEED_PLAIN_TEXT_NOTE_ID,
    };

    #[tokio::test]
    async fn empty_fixture_is_deterministic() -> Result<(), Box<dyn std::error::Error>> {
        let first = FixtureDb::empty().await?;
        let second = FixtureDb::empty().await?;

        assert_ne!(first.path(), second.path());
        assert!(first.path().is_file());
        assert!(second.path().is_file());
        Ok(())
    }

    #[tokio::test]
    async fn seeded_fixture_contains_expected_rows() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = FixtureDb::seeded().await?;
        let options = SqliteConnectOptions::new()
            .filename(fixture.path())
            .read_only(true);
        let pool = sqlx::SqlitePool::connect_with(options).await?;

        let handle_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"count!\" FROM handle WHERE id = ?1",
            SEED_HANDLE_ID
        )
        .fetch_one(&pool)
        .await?;

        let chat_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"count!\" FROM chat WHERE guid = ?1",
            SEED_CHAT_GUID
        )
        .fetch_one(&pool)
        .await?;

        let message_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"count!\" FROM message WHERE guid = ?1",
            SEED_MESSAGE_GUID
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(handle_count, 1);
        assert_eq!(chat_count, 1);
        assert_eq!(message_count, 1);
        Ok(())
    }

    #[tokio::test]
    async fn empty_reminders_fixture_loads_schema() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = RemindersFixtureDb::empty().await?;
        let options = SqliteConnectOptions::new()
            .filename(fixture.path())
            .read_only(true);
        let pool = sqlx::SqlitePool::connect_with(options).await?;

        let table_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"count!\" FROM sqlite_master WHERE name = 'ZREMCDREMINDER'"
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(table_count, 1);
        Ok(())
    }

    #[tokio::test]
    async fn seeded_reminders_fixture_contains_expected_rows()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = RemindersFixtureDb::seeded().await?;
        let options = SqliteConnectOptions::new()
            .filename(fixture.path())
            .read_only(true);
        let pool = sqlx::SqlitePool::connect_with(options).await?;

        let reminder_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"count!\" FROM ZREMCDREMINDER WHERE ZMARKEDFORDELETION = 0"
        )
        .fetch_one(&pool)
        .await?;

        assert!(reminder_count >= 2);
        Ok(())
    }

    #[tokio::test]
    async fn empty_notes_fixture_loads_schema() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = NotesFixtureDb::empty().await?;
        let options = SqliteConnectOptions::new()
            .filename(fixture.path())
            .read_only(true);
        let pool = sqlx::SqlitePool::connect_with(options).await?;

        let table_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"count!\" FROM sqlite_master WHERE name = 'ZICCLOUDSYNCINGOBJECT'"
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(table_count, 1);
        Ok(())
    }

    #[tokio::test]
    async fn seeded_notes_fixture_contains_expected_rows() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = NotesFixtureDb::seeded().await?;
        let options = SqliteConnectOptions::new()
            .filename(fixture.path())
            .read_only(true);
        let pool = sqlx::SqlitePool::connect_with(options).await?;

        let folder_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"count!\" FROM ZICCLOUDSYNCINGOBJECT \
             WHERE Z_ENT = 15 AND ZIDENTIFIER = ?1",
            SEED_NOTES_FOLDER_ID
        )
        .fetch_one(&pool)
        .await?;

        let note_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"count!\" FROM ZICCLOUDSYNCINGOBJECT \
             WHERE Z_ENT = 12 AND ZMARKEDFORDELETION = 0"
        )
        .fetch_one(&pool)
        .await?;

        let locked_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"count!\" FROM ZICCLOUDSYNCINGOBJECT \
             WHERE Z_ENT = 12 AND ZIDENTIFIER = ?1 AND ZISPASSWORDPROTECTED = 1",
            SEED_LOCKED_NOTE_ID
        )
        .fetch_one(&pool)
        .await?;

        let checklist_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"count!\" FROM ZICCLOUDSYNCINGOBJECT \
             WHERE Z_ENT = 12 AND ZIDENTIFIER = ?1 AND ZHASCHECKLIST = 1",
            SEED_CHECKLIST_NOTE_ID
        )
        .fetch_one(&pool)
        .await?;

        let body_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"count!\" FROM ZICNOTEDATA nd \
             JOIN ZICCLOUDSYNCINGOBJECT n ON nd.ZNOTE = n.Z_PK \
             WHERE n.ZIDENTIFIER = ?1 AND length(nd.ZDATA) > 0",
            SEED_PLAIN_TEXT_NOTE_ID
        )
        .fetch_one(&pool)
        .await?;

        let hashtag_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"count!\" FROM ZICCLOUDSYNCINGOBJECT \
             WHERE ZTYPEUTI1 = 'com.apple.notes.inlinetextattachment.hashtag' \
               AND ZNOTE1 = 6 AND ZALTTEXT = '#reading'"
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(folder_count, 1);
        assert!(note_count >= 5);
        assert_eq!(locked_count, 1);
        assert_eq!(checklist_count, 1);
        assert_eq!(body_count, 1);
        assert_eq!(hashtag_count, 1);
        Ok(())
    }
}
