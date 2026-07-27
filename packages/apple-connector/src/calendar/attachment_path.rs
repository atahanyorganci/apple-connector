use std::path::{Component, Path, PathBuf};

pub fn resolve_attachment_path(
    root: &Path,
    local_path: &str,
) -> Result<PathBuf, AttachmentPathError> {
    let relative = Path::new(local_path);
    if relative.is_absolute() {
        return Err(AttachmentPathError::AbsolutePath);
    }
    for component in relative.components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(AttachmentPathError::Traversal);
            }
            _ => {}
        }
    }
    let joined = root.join(relative);
    let canonical_root = root
        .canonicalize()
        .map_err(|_| AttachmentPathError::RootUnavailable)?;
    let canonical = joined
        .canonicalize()
        .map_err(|_| AttachmentPathError::NotFound)?;
    if !canonical.starts_with(&canonical_root) {
        return Err(AttachmentPathError::Traversal);
    }
    if !canonical.is_file() {
        return Err(AttachmentPathError::NotFound);
    }
    Ok(canonical)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentPathError {
    AbsolutePath,
    Traversal,
    NotFound,
    RootUnavailable,
}

impl AttachmentPathError {
    pub fn message(self) -> &'static str {
        match self {
            Self::AbsolutePath => "attachment path must be relative",
            Self::Traversal => "attachment path escapes the attachment root",
            Self::NotFound => "attachment file not found",
            Self::RootUnavailable => "attachment root is unavailable",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write};

    use tempfile::TempDir;

    use super::{AttachmentPathError, resolve_attachment_path};

    #[test]
    fn resolves_relative_attachment_path() {
        let temp = TempDir::new().expect("temp");
        let root = temp.path().join("Attachments");
        fs::create_dir_all(&root).expect("dir");
        let file = root.join("agenda.pdf");
        fs::File::create(&file)
            .expect("file")
            .write_all(b"pdf")
            .expect("write");
        let resolved = resolve_attachment_path(&root, "agenda.pdf").expect("resolve");
        assert_eq!(resolved, file.canonicalize().expect("canonical"));
    }

    #[test]
    fn rejects_path_traversal() {
        let temp = TempDir::new().expect("temp");
        let root = temp.path().join("Attachments");
        fs::create_dir_all(&root).expect("dir");
        let error = resolve_attachment_path(&root, "../secret.txt").unwrap_err();
        assert_eq!(error, AttachmentPathError::Traversal);
    }
}
