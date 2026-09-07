use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ContactsError {
    #[error("item not found")]
    NotFound,
    #[error("Contacts access denied")]
    AccessDenied,
    #[error("container is read-only")]
    ReadOnlyContainer,
    #[error("validation failed: {0}")]
    ValidationFailed(String),
    #[error("ambiguous match: {0}")]
    AmbiguousMatch(String),
    #[error("Contacts is unavailable on this platform")]
    UnsupportedPlatform,
    #[error("Contacts framework error: {0}")]
    Framework(String),
    #[error("Contacts operation timed out")]
    Timeout,
}

pub type ContactsResult<T> = Result<T, ContactsError>;

/// Classifies an `NSError` from `CNErrorDomain` into a category the API can act on.
///
/// The read-only codes matter most: they are how the framework — the only authority on whether a
/// container accepts writes — reports a refusal, and they must reach the client as 403 rather
/// than collapsing into an internal error.
pub(crate) fn map_cn_error(err: objc2::rc::Retained<objc2_foundation::NSError>) -> ContactsError {
    use objc2_contacts::CNErrorCode;

    let cn_domain = unsafe { objc2_contacts::CNErrorDomain };
    if cn_domain.is_some_and(|domain| err.domain().to_string() == domain.to_string()) {
        let code = CNErrorCode(err.code());
        if code == CNErrorCode::RecordDoesNotExist {
            return ContactsError::NotFound;
        }
        if code == CNErrorCode::AuthorizationDenied {
            return ContactsError::AccessDenied;
        }
        if code == CNErrorCode::RecordNotWritable
            || code == CNErrorCode::ParentContainerNotWritable
            || code == CNErrorCode::NoAccessableWritableContainers
        {
            return ContactsError::ReadOnlyContainer;
        }
        if code == CNErrorCode::ValidationMultipleErrors
            || code == CNErrorCode::ValidationTypeMismatch
            || code == CNErrorCode::ValidationConfigurationError
        {
            return ContactsError::ValidationFailed(err.localizedDescription().to_string());
        }
    }

    ContactsError::Framework(err.localizedDescription().to_string())
}

#[cfg(test)]
mod tests {
    use objc2_contacts::CNErrorCode;
    use objc2_foundation::{NSError, NSString};

    use super::{ContactsError, map_cn_error};

    fn cn_error(code: CNErrorCode) -> objc2::rc::Retained<NSError> {
        let domain = unsafe { objc2_contacts::CNErrorDomain }
            .map(objc2::rc::Retained::from)
            .unwrap_or_else(|| NSString::from_str("CNErrorDomain"));
        unsafe { NSError::errorWithDomain_code_userInfo(&domain, code.0, None) }
    }

    /// Writability is decided by the framework at save time, so these codes are the whole
    /// read-only story now that the stale SQLite hint is gone.
    #[test]
    fn not_writable_codes_map_to_read_only_container() {
        for code in [
            CNErrorCode::RecordNotWritable,
            CNErrorCode::ParentContainerNotWritable,
            CNErrorCode::NoAccessableWritableContainers,
        ] {
            assert_eq!(
                map_cn_error(cn_error(code)),
                ContactsError::ReadOnlyContainer
            );
        }
    }

    #[test]
    fn missing_records_and_denials_stay_distinct() {
        assert_eq!(
            map_cn_error(cn_error(CNErrorCode::RecordDoesNotExist)),
            ContactsError::NotFound
        );
        assert_eq!(
            map_cn_error(cn_error(CNErrorCode::AuthorizationDenied)),
            ContactsError::AccessDenied
        );
    }

    #[test]
    fn unsupported_platform_error_is_distinct() {
        assert_eq!(
            ContactsError::UnsupportedPlatform.to_string(),
            "Contacts is unavailable on this platform"
        );
    }

    #[test]
    fn read_only_container_error_message() {
        assert_eq!(
            ContactsError::ReadOnlyContainer.to_string(),
            "container is read-only"
        );
    }
}
