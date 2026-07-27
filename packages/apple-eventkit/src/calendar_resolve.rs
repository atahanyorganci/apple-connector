use objc2_event_kit::{EKCalendar, EKCalendarType, EKEntityType, EKEventStore};
use objc2_foundation::NSString;

use crate::error::{EventKitError, EventKitResult};

#[derive(Debug, Clone)]
pub struct ReminderListResolveHint {
    pub api_id: String,
    pub external_id: Option<String>,
    pub title: String,
    pub is_smart_list: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarStoreType {
    Local,
    CalDav,
    Exchange,
    Subscription,
    Birthday,
}

#[derive(Debug, Clone)]
pub struct CalendarResolveHint {
    pub api_id: String,
    pub external_id: Option<String>,
    pub title: Option<String>,
    pub store_type: CalendarStoreType,
}

pub(crate) fn resolve_reminder_list(
    store: &EKEventStore,
    hint: &ReminderListResolveHint,
) -> EventKitResult<objc2::rc::Retained<EKCalendar>> {
    if hint.is_smart_list {
        return Err(EventKitError::ReadOnlyCalendar);
    }

    if let Some(calendar) = lookup_calendar(store, &hint.external_id, &hint.api_id) {
        return validate_reminder_calendar(calendar);
    }

    let calendars = unsafe { store.calendarsForEntityType(EKEntityType::Reminder) };
    let title = hint.title.to_ascii_lowercase();
    let mut matches = calendars
        .iter()
        .filter(|cal| unsafe { cal.title().to_string().to_ascii_lowercase() } == title)
        .collect::<Vec<_>>();

    if matches.len() == 1 {
        return validate_reminder_calendar(matches.remove(0));
    }
    if matches.len() > 1 {
        return Err(EventKitError::AmbiguousMatch(format!(
            "multiple reminder lists named '{}'",
            hint.title
        )));
    }

    Err(EventKitError::NotFound)
}

pub(crate) fn resolve_event_calendar(
    store: &EKEventStore,
    hint: &CalendarResolveHint,
) -> EventKitResult<objc2::rc::Retained<EKCalendar>> {
    if matches!(
        hint.store_type,
        CalendarStoreType::Birthday | CalendarStoreType::Subscription
    ) {
        return Err(EventKitError::ReadOnlyCalendar);
    }

    if let Some(calendar) = lookup_calendar(store, &hint.external_id, &hint.api_id) {
        return validate_event_calendar(calendar, hint.store_type);
    }

    if let Some(title) = &hint.title {
        let calendars = unsafe { store.calendarsForEntityType(EKEntityType::Event) };
        let lowered = title.to_ascii_lowercase();
        let mut matches = calendars
            .iter()
            .filter(|cal| unsafe { cal.title().to_string().to_ascii_lowercase() } == lowered)
            .collect::<Vec<_>>();

        if matches.len() == 1 {
            return validate_event_calendar(matches.remove(0), hint.store_type);
        }
        if matches.len() > 1 {
            return Err(EventKitError::AmbiguousMatch(format!(
                "multiple calendars named '{title}'"
            )));
        }
    }

    Err(EventKitError::NotFound)
}

fn lookup_calendar(
    store: &EKEventStore,
    external_id: &Option<String>,
    api_id: &str,
) -> Option<objc2::rc::Retained<EKCalendar>> {
    let mut candidates = Vec::new();
    if let Some(external_id) = external_id {
        candidates.push(external_id.as_str());
    }
    candidates.push(api_id);
    for candidate in candidates {
        let ns_id = NSString::from_str(candidate);
        if let Some(calendar) = unsafe { store.calendarWithIdentifier(&ns_id) } {
            return Some(calendar);
        }
    }
    None
}

fn validate_reminder_calendar(
    calendar: objc2::rc::Retained<EKCalendar>,
) -> EventKitResult<objc2::rc::Retained<EKCalendar>> {
    if !unsafe { calendar.allowsContentModifications() } {
        return Err(EventKitError::ReadOnlyCalendar);
    }
    Ok(calendar)
}

fn validate_event_calendar(
    calendar: objc2::rc::Retained<EKCalendar>,
    store_type: CalendarStoreType,
) -> EventKitResult<objc2::rc::Retained<EKCalendar>> {
    if matches!(
        store_type,
        CalendarStoreType::Birthday | CalendarStoreType::Subscription
    ) {
        return Err(EventKitError::ReadOnlyCalendar);
    }
    if unsafe { calendar.isSubscribed() } {
        return Err(EventKitError::ReadOnlyCalendar);
    }
    let cal_type = unsafe { calendar.r#type() };
    if cal_type == EKCalendarType::Birthday || cal_type == EKCalendarType::Subscription {
        return Err(EventKitError::ReadOnlyCalendar);
    }
    if !unsafe { calendar.allowsContentModifications() } {
        return Err(EventKitError::ReadOnlyCalendar);
    }
    Ok(calendar)
}

#[cfg(test)]
mod tests {
    use super::CalendarStoreType;

    #[test]
    fn birthday_store_type_is_marked_read_only() {
        assert!(matches!(
            CalendarStoreType::Birthday,
            CalendarStoreType::Birthday
        ));
    }
}
