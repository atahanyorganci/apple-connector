use objc2_event_kit::{EKCalendarItem, EKEvent, EKEventStore, EKReminder};
use objc2_foundation::NSString;

use crate::{
    datetime::retained_date_to_unix,
    error::{EventKitError, EventKitResult},
};

pub(crate) fn lookup_reminder(
    store: &EKEventStore,
    api_id: &str,
    external_id: Option<&str>,
) -> EventKitResult<objc2::rc::Retained<EKReminder>> {
    let items = find_items(store, api_id, external_id)?;
    let reminders: Vec<_> = items
        .into_iter()
        .filter_map(|item| item.downcast::<EKReminder>().ok())
        .collect();
    match reminders.len() {
        0 => Err(EventKitError::NotFound),
        1 => Ok(reminders.into_iter().next().ok_or_else(|| {
            EventKitError::Framework("reminder iterator unexpectedly empty".into())
        })?),
        n => Err(EventKitError::AmbiguousMatch(format!(
            "{n} reminders matched identifier '{api_id}'"
        ))),
    }
}

pub(crate) fn lookup_event(
    store: &EKEventStore,
    api_id: &str,
    external_id: Option<&str>,
    occurrence_start: Option<i64>,
) -> EventKitResult<objc2::rc::Retained<EKEvent>> {
    let mut events: Vec<_> = find_items(store, api_id, external_id)?
        .into_iter()
        .filter_map(|item| item.downcast::<EKEvent>().ok())
        .collect();

    if events.is_empty() {
        let ns_id = NSString::from_str(api_id);
        if let Some(event) = unsafe { store.eventWithIdentifier(&ns_id) } {
            events.push(event);
        }
    }

    if let Some(start) = occurrence_start {
        let matches: Vec<_> = events
            .into_iter()
            .filter(|event| event_start(event) == start)
            .collect();
        return match matches.len() {
            0 => Err(EventKitError::NotFound),
            1 => Ok(matches.into_iter().next().ok_or_else(|| {
                EventKitError::Framework("event iterator unexpectedly empty".into())
            })?),
            n => Err(EventKitError::AmbiguousMatch(format!(
                "{n} event occurrences matched start {start}"
            ))),
        };
    }

    match events.len() {
        0 => Err(EventKitError::NotFound),
        1 => Ok(events
            .into_iter()
            .next()
            .ok_or_else(|| EventKitError::Framework("event iterator unexpectedly empty".into()))?),
        n => Err(EventKitError::AmbiguousMatch(format!(
            "{n} events matched identifier '{api_id}'; pass occurrence_start to disambiguate"
        ))),
    }
}

fn find_items(
    store: &EKEventStore,
    api_id: &str,
    external_id: Option<&str>,
) -> EventKitResult<Vec<objc2::rc::Retained<EKCalendarItem>>> {
    let mut collected = Vec::new();
    let mut seen = Vec::new();

    let ns_id = NSString::from_str(api_id);
    if let Some(item) = unsafe { store.calendarItemWithIdentifier(&ns_id) } {
        let id = unsafe { item.calendarItemIdentifier().to_string() };
        seen.push(id.clone());
        collected.push(item);
    }

    let mut candidates = vec![api_id.to_owned()];
    if let Some(external_id) = external_id {
        candidates.push(external_id.to_owned());
    }

    for candidate in candidates {
        let ns = NSString::from_str(&candidate);
        let items = unsafe { store.calendarItemsWithExternalIdentifier(&ns) };
        for item in items {
            let id = unsafe { item.calendarItemIdentifier().to_string() };
            if seen.iter().any(|existing| existing == &id) {
                continue;
            }
            seen.push(id);
            collected.push(item);
        }
    }

    if collected.is_empty() {
        return Err(EventKitError::NotFound);
    }

    Ok(collected)
}

fn event_start(event: &EKEvent) -> i64 {
    retained_date_to_unix(&unsafe { event.startDate() })
}

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder() {}
}
