use std::{
    io::{Error as IoError, ErrorKind},
    path::{Path, PathBuf},
};

pub fn default_notes_database_path() -> Result<PathBuf, IoError> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| IoError::new(ErrorKind::NotFound, "HOME is not set"))?;
    Ok(PathBuf::from(home).join("Library/Group Containers/group.com.apple.notes/NoteStore.sqlite"))
}

pub fn default_notes_attachment_root() -> Result<PathBuf, IoError> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| IoError::new(ErrorKind::NotFound, "HOME is not set"))?;
    Ok(PathBuf::from(home).join("Library/Group Containers/group.com.apple.notes/Accounts"))
}

pub fn notes_attachment_root_for_database(database_path: &Path) -> Result<PathBuf, IoError> {
    let _ = database_path;
    default_notes_attachment_root()
}

#[cfg(test)]
mod tests {
    use super::{default_notes_attachment_root, default_notes_database_path};

    #[test]
    fn default_database_path_is_under_group_containers() -> Result<(), Box<dyn std::error::Error>> {
        let path = default_notes_database_path()?;
        assert!(path.to_string_lossy().contains("group.com.apple.notes"));
        assert!(path.to_string_lossy().ends_with("NoteStore.sqlite"));
        Ok(())
    }

    #[test]
    fn default_attachment_root_points_at_accounts() -> Result<(), Box<dyn std::error::Error>> {
        let path = default_notes_attachment_root()?;
        assert!(path.to_string_lossy().contains("group.com.apple.notes"));
        assert!(path.ends_with("Accounts"));
        Ok(())
    }
}
