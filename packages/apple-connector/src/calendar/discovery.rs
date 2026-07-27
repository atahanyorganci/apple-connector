use std::{
    io::{Error as IoError, ErrorKind},
    path::{Path, PathBuf},
};

pub fn default_calendar_database_path() -> Result<PathBuf, IoError> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| IoError::new(ErrorKind::NotFound, "HOME is not set"))?;
    Ok(PathBuf::from(home)
        .join("Library/Group Containers/group.com.apple.calendar/Calendar.sqlitedb"))
}

pub fn default_calendar_attachment_root() -> Result<PathBuf, IoError> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| IoError::new(ErrorKind::NotFound, "HOME is not set"))?;
    Ok(PathBuf::from(home).join("Library/Group Containers/group.com.apple.calendar/Attachments"))
}

pub fn calendar_attachment_root_for_database(database_path: &Path) -> Result<PathBuf, IoError> {
    let _ = database_path;
    default_calendar_attachment_root()
}

pub fn legacy_calendar_database_paths() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME") else {
        return Vec::new();
    };
    let home = PathBuf::from(home);
    vec![
        home.join("Library/Calendars/Calendar.sqlitedb"),
        home.join("Library/Application Support/Calendar/Calendar.sqlitedb"),
    ]
}

#[cfg(test)]
mod tests {
    use super::{default_calendar_attachment_root, default_calendar_database_path};

    #[test]
    fn default_database_path_is_under_group_containers() {
        let path = default_calendar_database_path().expect("database path");
        assert!(path.to_string_lossy().contains("group.com.apple.calendar"));
        assert!(path.to_string_lossy().ends_with("Calendar.sqlitedb"));
    }

    #[test]
    fn default_attachment_root_points_at_attachments() {
        let path = default_calendar_attachment_root().expect("attachment root");
        assert!(path.to_string_lossy().contains("group.com.apple.calendar"));
        assert!(path.ends_with("Attachments"));
    }
}
