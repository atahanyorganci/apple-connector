use std::{
    io::{Error as IoError, ErrorKind},
    path::{Path, PathBuf},
};

use sqlx::{Connection, sqlite::SqliteConnectOptions};
use tracing::debug;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    StoresDirNotFound,
    NoStoresFound,
    Connect(String),
}

impl DiscoveryError {
    pub fn message(&self) -> String {
        match self {
            Self::StoresDirNotFound => "Reminders stores directory not found".to_owned(),
            Self::NoStoresFound => "No Reminders store databases found".to_owned(),
            Self::Connect(message) => format!("Could not inspect Reminders store: {message}"),
        }
    }
}

pub fn default_reminders_stores_dir() -> Result<PathBuf, IoError> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| IoError::new(ErrorKind::NotFound, "HOME is not set"))?;
    Ok(PathBuf::from(home)
        .join("Library/Group Containers/group.com.apple.reminders/Container_v1/Stores"))
}

pub fn default_reminders_attachment_root(store_path: &Path) -> PathBuf {
    let parent = store_path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = store_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");

    if let Some(uuid) = file_name
        .strip_prefix("Data-")
        .and_then(|rest| rest.strip_suffix(".sqlite"))
    {
        return parent.join(format!(".Data-{uuid}_SUPPORT"));
    }

    parent.to_path_buf()
}

pub async fn discover_reminders_database(stores_dir: &Path) -> Result<PathBuf, DiscoveryError> {
    if !stores_dir.is_dir() {
        return Err(DiscoveryError::StoresDirNotFound);
    }

    let mut candidates = Vec::new();
    let mut read_dir = tokio::fs::read_dir(stores_dir)
        .await
        .map_err(|error| DiscoveryError::Connect(error.to_string()))?;

    while let Some(entry) = read_dir
        .next_entry()
        .await
        .map_err(|error| DiscoveryError::Connect(error.to_string()))?
    {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.starts_with("Data-") || !file_name.ends_with(".sqlite") {
            continue;
        }

        let reminder_count = count_active_reminders(&path).await?;
        let mtime = entry
            .metadata()
            .await
            .ok()
            .and_then(|meta| meta.modified().ok())
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs());
        let z_max = max_reminder_primary_key(&path).await.ok().flatten();

        candidates.push(StoreCandidate {
            path,
            reminder_count,
            mtime,
            z_max,
        });
    }

    candidates.sort_by(|left, right| {
        right
            .reminder_count
            .cmp(&left.reminder_count)
            .then_with(|| right.mtime.cmp(&left.mtime))
            .then_with(|| right.z_max.cmp(&left.z_max))
    });

    let selected = candidates
        .into_iter()
        .next()
        .ok_or(DiscoveryError::NoStoresFound)?;

    debug!(
        path = %selected.path.display(),
        reminders = selected.reminder_count,
        "discovered Reminders store"
    );
    Ok(selected.path)
}

#[derive(Debug)]
struct StoreCandidate {
    path: PathBuf,
    reminder_count: i64,
    mtime: Option<u64>,
    z_max: Option<i64>,
}

async fn count_active_reminders(path: &Path) -> Result<i64, DiscoveryError> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .read_only(true)
        .create_if_missing(false);

    let mut connection = sqlx::SqliteConnection::connect_with(&options)
        .await
        .map_err(|error| DiscoveryError::Connect(error.to_string()))?;

    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) AS \"count!: i64\" FROM ZREMCDREMINDER WHERE ZMARKEDFORDELETION = 0"
    )
    .fetch_one(&mut connection)
    .await
    .map_err(|error| DiscoveryError::Connect(error.to_string()))?;

    connection.close().await.ok();
    Ok(count)
}

async fn max_reminder_primary_key(path: &Path) -> Result<Option<i64>, DiscoveryError> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .read_only(true)
        .create_if_missing(false);

    let mut connection = sqlx::SqliteConnection::connect_with(&options)
        .await
        .map_err(|error| DiscoveryError::Connect(error.to_string()))?;

    let max = sqlx::query_scalar!("SELECT Z_MAX FROM Z_PRIMARYKEY WHERE Z_NAME = 'REMCDReminder'")
        .fetch_optional(&mut connection)
        .await
        .map_err(|error| DiscoveryError::Connect(error.to_string()))?;

    connection.close().await.ok();
    Ok(max.flatten())
}

#[cfg(test)]
mod tests {
    use super::{default_reminders_attachment_root, default_reminders_stores_dir};

    #[test]
    fn default_stores_dir_is_under_group_containers() -> Result<(), Box<dyn std::error::Error>> {
        let path = default_reminders_stores_dir()?;
        assert!(path.to_string_lossy().contains("group.com.apple.reminders"));
        Ok(())
    }

    #[test]
    fn attachment_root_derives_support_directory_from_store_name() {
        let store =
            std::path::Path::new("/Stores/Data-C4B33194-D5FB-428C-BD59-84C67F54B563.sqlite");
        let root = default_reminders_attachment_root(store);
        assert_eq!(
            root,
            std::path::PathBuf::from("/Stores/.Data-C4B33194-D5FB-428C-BD59-84C67F54B563_SUPPORT")
        );
    }
}
