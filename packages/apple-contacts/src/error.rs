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
    #[error("Contacts is unavailable on this platform")]
    UnsupportedPlatform,
    #[error("Contacts framework error: {0}")]
    Framework(String),
    #[error("Contacts operation timed out")]
    Timeout,
}

pub type ContactsResult<T> = Result<T, ContactsError>;

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
    use super::ContactsError;

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
