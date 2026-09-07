use objc2::rc::Retained;
use objc2_foundation::NSError;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EventKitError {
    #[error("item not found")]
    NotFound,
    #[error("EventKit access denied")]
    AccessDenied,
    #[error("calendar is read-only")]
    ReadOnlyCalendar,
    #[error("validation failed: {0}")]
    ValidationFailed(String),
    #[error("ambiguous match: {0}")]
    AmbiguousMatch(String),
    #[error("EventKit is unavailable on this platform")]
    UnsupportedPlatform,
    #[error("EventKit framework error: {0}")]
    Framework(String),
    #[error("EventKit operation timed out")]
    Timeout,
}

pub type EventKitResult<T> = Result<T, EventKitError>;

/// Classifies an `NSError` from `EKErrorDomain` into a category the API can act on.
///
/// Without this every framework refusal — a read-only calendar, inverted dates, a priority out of
/// range — collapses into `Framework` and surfaces as HTTP 500. Codes outside the EventKit domain,
/// and codes with no useful category, stay `Framework`.
pub(crate) fn map_ek_error(error: Retained<NSError>) -> EventKitError {
    use objc2_event_kit::EKErrorCode;

    let ek_domain = unsafe { objc2_event_kit::EKErrorDomain };
    if ek_domain.is_some_and(|domain| error.domain().to_string() == domain.to_string()) {
        let code = EKErrorCode(error.code());
        let description = error.localizedDescription().to_string();

        if code == EKErrorCode::EventStoreNotAuthorized {
            return EventKitError::AccessDenied;
        }
        if code == EKErrorCode::OSNotSupported {
            return EventKitError::UnsupportedPlatform;
        }
        if matches!(
            code,
            EKErrorCode::CalendarReadOnly
                | EKErrorCode::CalendarIsImmutable
                | EKErrorCode::EventNotMutable
                | EKErrorCode::ProcedureAlarmsNotMutable
                | EKErrorCode::CalendarSourceCannotBeModified
                | EKErrorCode::CalendarDoesNotAllowEvents
                | EKErrorCode::CalendarDoesNotAllowReminders
                | EKErrorCode::SourceDoesNotAllowEvents
                | EKErrorCode::SourceDoesNotAllowReminders
                | EKErrorCode::SourceDoesNotAllowCalendarAddDelete
        ) {
            return EventKitError::ReadOnlyCalendar;
        }
        if matches!(
            code,
            EKErrorCode::NoCalendar | EKErrorCode::CalendarHasNoSource
        ) {
            return EventKitError::NotFound;
        }
        if matches!(
            code,
            EKErrorCode::DatesInverted
                | EKErrorCode::NoStartDate
                | EKErrorCode::NoEndDate
                | EKErrorCode::DurationGreaterThanRecurrence
                | EKErrorCode::AlarmGreaterThanRecurrence
                | EKErrorCode::StartDateTooFarInFuture
                | EKErrorCode::StartDateCollidesWithOtherOccurrence
                | EKErrorCode::ObjectBelongsToDifferentStore
                | EKErrorCode::InvitesCannotBeMoved
                | EKErrorCode::InvalidSpan
                | EKErrorCode::InvalidEntityType
                | EKErrorCode::InvalidInviteReplyCalendar
                | EKErrorCode::RecurringReminderRequiresDueDate
                | EKErrorCode::StructuredLocationsNotSupported
                | EKErrorCode::ReminderLocationsNotSupported
                | EKErrorCode::AlarmProximityNotSupported
                | EKErrorCode::ReminderAlarmContainsEmailOrUrl
                | EKErrorCode::PriorityIsInvalid
                | EKErrorCode::SourceMismatch
        ) {
            return EventKitError::ValidationFailed(description);
        }
    }

    EventKitError::Framework(error.localizedDescription().to_string())
}

#[cfg(test)]
mod tests {
    use objc2_event_kit::EKErrorCode;
    use objc2_foundation::{NSError, NSString, ns_string};

    use super::{EventKitError, map_ek_error};

    fn ek_error(code: EKErrorCode) -> objc2::rc::Retained<NSError> {
        let domain =
            unsafe { objc2_event_kit::EKErrorDomain }.unwrap_or(ns_string!("EKErrorDomain"));
        unsafe { NSError::errorWithDomain_code_userInfo(domain, code.0, None) }
    }

    fn foreign_error() -> objc2::rc::Retained<NSError> {
        unsafe {
            NSError::errorWithDomain_code_userInfo(
                &NSString::from_str("NSCocoaErrorDomain"),
                6,
                None,
            )
        }
    }

    #[test]
    fn read_only_codes_map_to_read_only_calendar() {
        for code in [
            EKErrorCode::CalendarReadOnly,
            EKErrorCode::CalendarIsImmutable,
            EKErrorCode::EventNotMutable,
            EKErrorCode::SourceDoesNotAllowReminders,
        ] {
            assert_eq!(
                map_ek_error(ek_error(code)),
                EventKitError::ReadOnlyCalendar
            );
        }
    }

    #[test]
    fn authorization_and_platform_codes_are_distinct() {
        assert_eq!(
            map_ek_error(ek_error(EKErrorCode::EventStoreNotAuthorized)),
            EventKitError::AccessDenied
        );
        assert_eq!(
            map_ek_error(ek_error(EKErrorCode::OSNotSupported)),
            EventKitError::UnsupportedPlatform
        );
    }

    #[test]
    fn validation_codes_carry_the_framework_description() {
        let mapped = map_ek_error(ek_error(EKErrorCode::DatesInverted));
        assert!(matches!(mapped, EventKitError::ValidationFailed(_)));
    }

    #[test]
    fn unclassified_and_foreign_errors_stay_framework() {
        assert!(matches!(
            map_ek_error(ek_error(EKErrorCode::InternalFailure)),
            EventKitError::Framework(_)
        ));
        assert!(matches!(
            map_ek_error(foreign_error()),
            EventKitError::Framework(_)
        ));
    }
}
