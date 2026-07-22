use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use super::{
    attachments::resolve_attachment_path,
    model::{AttachmentKind, AttachmentKind::*},
};

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

/// Default Messages attachment directory derived from `chat.db` location.
pub fn default_attachment_root(database_path: &Path) -> PathBuf {
    database_path
        .parent()
        .map(|parent| parent.join("Attachments"))
        .unwrap_or_else(|| PathBuf::from("Attachments"))
}

pub fn canonicalize_attachment_root(path: &Path) -> std::io::Result<PathBuf> {
    if path.exists() {
        std::fs::canonicalize(path)
    } else {
        Ok(path.to_path_buf())
    }
}

pub fn is_present_on_disk(root: &Path, filename: Option<&str>) -> bool {
    filename.is_some_and(|value| validate_attachment_path(root, value).is_ok())
}

pub fn validate_attachment_path(
    root: &Path,
    filename: &str,
) -> Result<ValidatedAttachmentPath, AttachmentPathError> {
    let resolved = resolve_attachment_path(filename).ok_or(AttachmentPathError::MissingFilename)?;

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
    if !is_contained_in(&canonical, canonical_root) {
        return Err(AttachmentPathError::EscapesRoot);
    }
    if !canonical.is_file() {
        return Err(AttachmentPathError::NotAFile);
    }
    Ok(ValidatedAttachmentPath {
        canonical_path: canonical,
    })
}

fn is_contained_in(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}

pub fn sanitize_download_filename(
    transfer_name: Option<&str>,
    mime_type: Option<&str>,
    guid: &str,
) -> String {
    let mut candidate = transfer_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            Path::new(value)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(value)
                .to_owned()
        })
        .unwrap_or_else(|| fallback_filename(mime_type, guid));

    candidate = candidate
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect();

    if candidate.is_empty() {
        candidate = fallback_filename(mime_type, guid);
    }

    candidate.truncate(200);
    candidate
}

fn fallback_filename(mime_type: Option<&str>, guid: &str) -> String {
    let extension = mime_type
        .and_then(|value| value.split('/').nth(1))
        .map(|subtype| {
            subtype
                .split('+')
                .next()
                .unwrap_or(subtype)
                .chars()
                .filter(|ch| ch.is_ascii_alphanumeric())
                .collect::<String>()
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "bin".to_owned());

    format!("{guid}.{extension}")
}

pub fn resolve_content_type(mime_type: Option<&str>) -> String {
    let Some(raw) = mime_type.map(str::trim).filter(|value| !value.is_empty()) else {
        return "application/octet-stream".to_owned();
    };

    if raw
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '-' | '+' | '_'))
        && raw.contains('/')
        && !raw.starts_with('/')
        && !raw.ends_with('/')
    {
        raw.to_ascii_lowercase()
    } else {
        "application/octet-stream".to_owned()
    }
}

pub fn content_disposition(kind: &AttachmentKind, filename: &str) -> String {
    let disposition = match kind {
        Image | Video | Audio | Sticker { .. } => "inline",
        File | Unknown => "attachment",
    };
    format!("{disposition}; filename=\"{filename}\"")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileValidators {
    pub etag: String,
    pub last_modified: String,
    pub content_length: u64,
}

pub fn file_validators(path: &Path) -> std::io::Result<FileValidators> {
    let metadata = std::fs::metadata(path)?;
    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);

    #[cfg(unix)]
    let etag = {
        use std::os::unix::fs::MetadataExt;
        format!(
            "\"{}-{}-{}\"",
            metadata.ino(),
            metadata.len(),
            metadata.mtime()
        )
    };
    #[cfg(not(unix))]
    let etag = {
        use std::time::UNIX_EPOCH;
        let modified_secs = modified
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format!("\"{}-{}\"", metadata.len(), modified_secs)
    };

    Ok(FileValidators {
        etag,
        last_modified: httpdate::fmt_http_date(modified),
        content_length: metadata.len(),
    })
}

pub fn if_none_match_satisfied(header_value: &str, etag: &str) -> bool {
    header_value
        .split(',')
        .any(|candidate| candidate.trim() == etag || candidate.trim() == "*")
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        os::unix::fs::symlink,
        path::{Path, PathBuf},
    };

    use super::{
        AttachmentPathError, canonicalize_attachment_root, default_attachment_root,
        sanitize_download_filename, validate_attachment_path,
    };

    fn fixture_layout() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("Attachments");
        fs::create_dir_all(&root).expect("attachments dir");
        let outside = temp.path().join("outside");
        fs::create_dir_all(&outside).expect("outside dir");
        (temp, root, outside)
    }

    #[test]
    fn default_root_is_sibling_attachments_directory() {
        assert_eq!(
            default_attachment_root(Path::new("/Users/test/Library/Messages/chat.db")),
            PathBuf::from("/Users/test/Library/Messages/Attachments")
        );
    }

    #[test]
    fn sanitize_strips_path_components_and_unsafe_characters() {
        let sanitized =
            sanitize_download_filename(Some("../../etc/passwd"), Some("image/jpeg"), "at-guid");
        assert_eq!(sanitized, "passwd");

        let sanitized = sanitize_download_filename(Some("my photo?.jpg"), None, "at-guid");
        assert_eq!(sanitized, "my_photo_.jpg");
    }

    #[test]
    fn allows_regular_file_inside_root() {
        let (_temp, root, _outside) = fixture_layout();
        let file = root.join("photo.jpg");
        fs::write(&file, b"hello").expect("write file");
        let canonical_root = canonicalize_attachment_root(&root).expect("canonical root");

        let validated =
            validate_attachment_path(&canonical_root, file.to_str().unwrap()).expect("valid file");
        assert!(validated.canonical_path.is_file());
    }

    #[test]
    fn rejects_traversal_and_symlink_escape() {
        let (_temp, root, outside) = fixture_layout();
        let secret = outside.join("secret.bin");
        fs::write(&secret, b"secret").expect("write secret");
        let canonical_root = canonicalize_attachment_root(&root).expect("canonical root");

        let traversal = root.join("../outside/secret.bin");
        assert_eq!(
            validate_attachment_path(&canonical_root, traversal.to_str().unwrap()),
            Err(AttachmentPathError::EscapesRoot)
        );

        let link = root.join("escape-link");
        symlink(&secret, &link).expect("symlink");
        assert_eq!(
            validate_attachment_path(&canonical_root, link.to_str().unwrap()),
            Err(AttachmentPathError::EscapesRoot)
        );
    }

    #[test]
    fn rejects_directories_and_missing_files() {
        let (_temp, root, _outside) = fixture_layout();
        let canonical_root = canonicalize_attachment_root(&root).expect("canonical root");

        assert_eq!(
            validate_attachment_path(&canonical_root, root.to_str().unwrap()),
            Err(AttachmentPathError::NotAFile)
        );
        assert_eq!(
            validate_attachment_path(&canonical_root, root.join("missing.bin").to_str().unwrap()),
            Err(AttachmentPathError::NotAFile)
        );
    }
}
