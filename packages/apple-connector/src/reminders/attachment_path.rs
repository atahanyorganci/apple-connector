use std::path::{Path, PathBuf};

use crate::api::blocking_io::{BlockingIoError, BlockingIoPool};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentPathError {
    MissingFilename,
    UnresolvablePath,
    EscapesRoot,
    NotAFile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedAttachmentPath {
    pub canonical_path: PathBuf,
}

pub fn resolve_attachment_path(root: &Path, filename: &str) -> Option<PathBuf> {
    let trimmed = filename.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(root.join(trimmed))
}

pub fn validate_attachment_path(
    root: &Path,
    filename: &str,
) -> Result<ValidatedAttachmentPath, AttachmentPathError> {
    let resolved =
        resolve_attachment_path(root, filename).ok_or(AttachmentPathError::MissingFilename)?;

    let canonical_root =
        std::fs::canonicalize(root).map_err(|_| AttachmentPathError::UnresolvablePath)?;

    let metadata =
        std::fs::symlink_metadata(&resolved).map_err(|_| AttachmentPathError::NotAFile)?;
    if metadata.file_type().is_symlink() {
        let canonical =
            std::fs::canonicalize(&resolved).map_err(|_| AttachmentPathError::NotAFile)?;
        return validated_file_in_root(canonical, &canonical_root);
    }

    if !metadata.is_file() {
        return Err(AttachmentPathError::NotAFile);
    }

    let canonical = std::fs::canonicalize(&resolved).map_err(|_| AttachmentPathError::NotAFile)?;
    validated_file_in_root(canonical, &canonical_root)
}

fn validated_file_in_root(
    canonical: PathBuf,
    canonical_root: &Path,
) -> Result<ValidatedAttachmentPath, AttachmentPathError> {
    if !canonical.starts_with(canonical_root) {
        return Err(AttachmentPathError::EscapesRoot);
    }
    if !canonical.is_file() {
        return Err(AttachmentPathError::NotAFile);
    }
    Ok(ValidatedAttachmentPath {
        canonical_path: canonical,
    })
}

pub async fn validate_attachment_path_async(
    pool: &BlockingIoPool,
    root: PathBuf,
    filename: String,
) -> Result<Result<ValidatedAttachmentPath, AttachmentPathError>, BlockingIoError> {
    pool.run(move || validate_attachment_path(&root, &filename))
        .await
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{AttachmentPathError, validate_attachment_path};

    #[test]
    fn rejects_path_traversal() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("support");
        fs::create_dir_all(&root)?;
        let outside = temp.path().join("outside.txt");
        fs::write(&outside, b"secret")?;

        let result = validate_attachment_path(&root, "../outside.txt");
        assert!(matches!(result, Err(AttachmentPathError::EscapesRoot)));
        Ok(())
    }
}
