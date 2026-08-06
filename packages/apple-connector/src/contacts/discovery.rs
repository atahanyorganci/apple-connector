use std::{
    io::{Error as IoError, ErrorKind},
    path::{Path, PathBuf},
};

use sqlx::{Connection, sqlite::SqliteConnectOptions};
use tracing::debug;

use crate::apple_types::SourceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    SourcesDirNotFound,
    NoSourcesFound,
    Connect(String),
}

impl DiscoveryError {
    pub fn message(&self) -> String {
        match self {
            Self::SourcesDirNotFound => "AddressBook sources directory not found".to_owned(),
            Self::NoSourcesFound => "No AddressBook source databases found".to_owned(),
            Self::Connect(message) => format!("Could not inspect AddressBook source: {message}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiscoveredSource {
    pub source_id: SourceId,
    pub path: PathBuf,
    pub contact_count: i64,
}

pub fn default_contacts_sources_dir() -> Result<PathBuf, IoError> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| IoError::new(ErrorKind::NotFound, "HOME is not set"))?;
    Ok(PathBuf::from(home).join("Library/Application Support/AddressBook/Sources"))
}

pub async fn discover_contacts_sources(
    sources_dir: &Path,
) -> Result<Vec<DiscoveredSource>, DiscoveryError> {
    if !sources_dir.is_dir() {
        return Err(DiscoveryError::SourcesDirNotFound);
    }

    let mut sources = Vec::new();
    let mut read_dir = tokio::fs::read_dir(sources_dir)
        .await
        .map_err(|error| DiscoveryError::Connect(error.to_string()))?;

    while let Some(entry) = read_dir
        .next_entry()
        .await
        .map_err(|error| DiscoveryError::Connect(error.to_string()))?
    {
        let source_path = entry.path();
        if !source_path.is_dir() {
            continue;
        }

        let Some(source_uuid) = source_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(SourceId::new)
        else {
            continue;
        };

        let db_path = find_addressbook_db(&source_path).await?;
        let Some(db_path) = db_path else {
            continue;
        };

        let contact_count = count_contacts(&db_path).await?;
        sources.push(DiscoveredSource {
            source_id: source_uuid,
            path: db_path,
            contact_count,
        });
    }

    sources.sort_by(|left, right| {
        right
            .contact_count
            .cmp(&left.contact_count)
            .then_with(|| left.source_id.as_str().cmp(right.source_id.as_str()))
    });

    if sources.is_empty() {
        return Err(DiscoveryError::NoSourcesFound);
    }

    for source in &sources {
        debug!(
            source = %source.source_id,
            path = %source.path.display(),
            contacts = source.contact_count,
            "discovered AddressBook source"
        );
    }

    Ok(sources)
}

async fn find_addressbook_db(source_dir: &Path) -> Result<Option<PathBuf>, DiscoveryError> {
    let mut read_dir = tokio::fs::read_dir(source_dir)
        .await
        .map_err(|error| DiscoveryError::Connect(error.to_string()))?;

    let mut best: Option<(PathBuf, u64)> = None;
    while let Some(entry) = read_dir
        .next_entry()
        .await
        .map_err(|error| DiscoveryError::Connect(error.to_string()))?
    {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.starts_with("AddressBook-v") || !name.ends_with(".abcddb") {
            continue;
        }
        let mtime = entry
            .metadata()
            .await
            .ok()
            .and_then(|meta| meta.modified().ok())
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        if best
            .as_ref()
            .is_none_or(|(_, best_mtime)| mtime >= *best_mtime)
        {
            best = Some((path, mtime));
        }
    }

    Ok(best.map(|(path, _)| path))
}

async fn count_contacts(path: &Path) -> Result<i64, DiscoveryError> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .read_only(true)
        .create_if_missing(false);

    let mut connection = sqlx::SqliteConnection::connect_with(&options)
        .await
        .map_err(|error| DiscoveryError::Connect(error.to_string()))?;

    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*)
        FROM ZABCDRECORD
        WHERE Z_ENT = 22
        "#
    )
    .fetch_one(&mut connection)
    .await
    .map_err(|error| DiscoveryError::Connect(error.to_string()))?;

    connection.close().await.ok();
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::default_contacts_sources_dir;

    #[test]
    fn default_sources_dir_is_under_application_support() -> Result<(), Box<dyn std::error::Error>>
    {
        let path = default_contacts_sources_dir()?;
        assert!(
            path.to_string_lossy()
                .contains("Application Support/AddressBook/Sources")
        );
        Ok(())
    }
}
