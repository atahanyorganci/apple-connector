use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentPathError {
    MissingAccountId,
    MissingFilename,
    UnresolvablePath,
    EscapesRoot,
    NotAFile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedAttachmentPath {
    pub canonical_path: PathBuf,
}

pub fn resolve_attachment_path(
    root: &Path,
    account_id: &str,
    filename: &str,
) -> Result<PathBuf, AttachmentPathError> {
    let account = account_id.trim();
    if account.is_empty() {
        return Err(AttachmentPathError::MissingAccountId);
    }

    let trimmed = filename.trim();
    if trimmed.is_empty() {
        return Err(AttachmentPathError::MissingFilename);
    }
    if trimmed.contains("..") || trimmed.starts_with('/') {
        return Err(AttachmentPathError::EscapesRoot);
    }

    Ok(root.join(account).join(trimmed))
}

pub fn validate_attachment_path(
    root: &Path,
    account_id: &str,
    filename: &str,
) -> Result<ValidatedAttachmentPath, AttachmentPathError> {
    let resolved = resolve_attachment_path(root, account_id, filename)?;

    let canonical_root =
        std::fs::canonicalize(root).map_err(|_| AttachmentPathError::UnresolvablePath)?;

    let account_root = canonical_root.join(account_id.trim());
    let canonical_account =
        std::fs::canonicalize(&account_root).map_err(|_| AttachmentPathError::UnresolvablePath)?;

    let metadata =
        std::fs::symlink_metadata(&resolved).map_err(|_| AttachmentPathError::NotAFile)?;
    if metadata.file_type().is_symlink() {
        let canonical =
            std::fs::canonicalize(&resolved).map_err(|_| AttachmentPathError::NotAFile)?;
        return validated_file_in_root(canonical, &canonical_account);
    }

    if !metadata.is_file() {
        return Err(AttachmentPathError::NotAFile);
    }

    let canonical = std::fs::canonicalize(&resolved).map_err(|_| AttachmentPathError::NotAFile)?;
    validated_file_in_root(canonical, &canonical_account)
}

fn validated_file_in_root(
    canonical: PathBuf,
    canonical_account_root: &Path,
) -> Result<ValidatedAttachmentPath, AttachmentPathError> {
    if !canonical.starts_with(canonical_account_root) {
        return Err(AttachmentPathError::EscapesRoot);
    }
    if !canonical.is_file() {
        return Err(AttachmentPathError::NotAFile);
    }
    Ok(ValidatedAttachmentPath {
        canonical_path: canonical,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{AttachmentPathError, validate_attachment_path};

    #[test]
    fn rejects_path_traversal() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("Accounts");
        let account = root.join("11111111-1111-1111-1111-111111111111");
        fs::create_dir_all(&account)?;
        let outside = temp.path().join("outside.txt");
        fs::write(&outside, b"secret")?;

        let result = validate_attachment_path(
            &root,
            "11111111-1111-1111-1111-111111111111",
            "../outside.txt",
        );
        assert!(matches!(result, Err(AttachmentPathError::EscapesRoot)));
        Ok(())
    }
}
