use objc2::AnyThread;
use objc2_core_location::CLLocation;
use objc2_event_kit::{EKEvent, EKEventStore, EKSpan, EKStructuredLocation};
use objc2_foundation::{NSString, NSURL};

use crate::{
    alarm::{AlarmInput, apply_alarms_to_item},
    calendar_resolve::{CalendarResolveHint, resolve_event_calendar},
    datetime::{retained_date_to_unix, start_of_local_day, unix_to_ns_date},
    error::{EventKitError, EventKitResult, map_ek_error},
    item_lookup::lookup_event,
    recurrence::{RecurrenceInput, apply_recurrence_to_item},
    reminder::LocationInput,
    store::EventKitStore,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSpan {
    This,
    Future,
    All,
}

impl EventSpan {
    fn to_ek(self) -> EKSpan {
        match self {
            Self::This => EKSpan::ThisEvent,
            Self::Future | Self::All => EKSpan::FutureEvents,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventStatusInput {
    Confirmed,
    Tentative,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct CreateEventInput {
    pub summary: String,
    pub description: Option<String>,
    pub start: i64,
    pub end: i64,
    pub all_day: bool,
    pub url: Option<String>,
    pub status: Option<EventStatusInput>,
    pub location: Option<LocationInput>,
    pub alarms: Vec<AlarmInput>,
    pub recurrence: Option<RecurrenceInput>,
}

#[derive(Debug, Clone)]
pub struct UpdateEventInput {
    pub summary: Option<String>,
    pub description: Option<Option<String>>,
    pub start: Option<i64>,
    pub end: Option<i64>,
    pub all_day: Option<bool>,
    pub url: Option<Option<String>>,
    pub status: Option<EventStatusInput>,
    pub calendar_hint: Option<CalendarResolveHint>,
    pub location: Option<Option<LocationInput>>,
    pub alarms: Option<Vec<AlarmInput>>,
    pub recurrence: Option<Option<RecurrenceInput>>,
    pub span: EventSpan,
}

#[derive(Debug, Clone)]
pub struct DeleteEventInput {
    pub span: EventSpan,
    pub occurrence_start: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SavedEvent {
    pub external_id: String,
    pub calendar_item_id: String,
}

impl EventKitStore {
    pub async fn create_event(
        &self,
        calendar_hint: CalendarResolveHint,
        input: CreateEventInput,
    ) -> EventKitResult<SavedEvent> {
        self.ensure_events().await?;
        validate_range(input.start, input.end)?;
        self.run(move |store| {
            let calendar = resolve_event_calendar(store, &calendar_hint)?;
            let event = unsafe { EKEvent::eventWithEventStore(store) };
            unsafe { event.setCalendar(Some(&calendar)) };
            apply_create_fields(&event, &input)?;
            save_event(store, &event, EventSpan::This)
        })
        .await
    }

    pub async fn update_event(
        &self,
        api_id: &str,
        external_id: Option<&str>,
        occurrence_start: Option<i64>,
        input: UpdateEventInput,
    ) -> EventKitResult<SavedEvent> {
        self.ensure_events().await?;
        let api_id = api_id.to_owned();
        let external_id = external_id.map(str::to_owned);
        self.run(move |store| {
            let event = lookup_event(store, &api_id, external_id.as_deref(), occurrence_start)?;
            apply_update_fields(store, &event, input.clone())?;
            save_event(store, &event, input.span)
        })
        .await
    }

    pub async fn delete_event(
        &self,
        api_id: &str,
        external_id: Option<&str>,
        input: DeleteEventInput,
    ) -> EventKitResult<()> {
        self.ensure_events().await?;
        let api_id = api_id.to_owned();
        let external_id = external_id.map(str::to_owned);
        self.run(move |store| {
            let event = lookup_event(
                store,
                &api_id,
                external_id.as_deref(),
                input.occurrence_start,
            )?;
            match unsafe { store.removeEvent_span_error(&event, input.span.to_ek()) } {
                Ok(()) => Ok(()),
                Err(err) => Err(map_ek_error(err)),
            }
        })
        .await
    }
}

fn save_event(
    store: &EKEventStore,
    event: &EKEvent,
    span: EventSpan,
) -> EventKitResult<SavedEvent> {
    match unsafe { store.saveEvent_span_error(event, span.to_ek()) } {
        Ok(()) => {
            let external_id = unsafe {
                event
                    .calendarItemExternalIdentifier()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| event.calendarItemIdentifier().to_string())
            };
            let calendar_item_id = unsafe { event.calendarItemIdentifier().to_string() };
            Ok(SavedEvent {
                external_id,
                calendar_item_id,
            })
        }
        Err(err) => Err(map_ek_error(err)),
    }
}

fn apply_create_fields(event: &EKEvent, input: &CreateEventInput) -> EventKitResult<()> {
    let title = NSString::from_str(&input.summary);
    unsafe { event.setTitle(Some(&title)) };
    if let Some(description) = &input.description {
        let ns = NSString::from_str(description);
        unsafe { event.setNotes(Some(&ns)) };
    }
    apply_dates(event, input.start, input.end, input.all_day)?;
    if let Some(url) = &input.url {
        let ns = NSURL::URLWithString(&NSString::from_str(url))
            .ok_or_else(|| EventKitError::ValidationFailed("invalid event url".into()))?;
        unsafe { event.setURL(Some(&ns)) };
    }
    let _ = input.status;
    apply_structured_location(event, input.location.as_ref())?;
    apply_alarms_to_item(event, &input.alarms)?;
    if let Some(recurrence) = &input.recurrence {
        apply_recurrence_to_item(event, std::slice::from_ref(recurrence))?;
    }
    Ok(())
}

fn apply_update_fields(
    store: &EKEventStore,
    event: &EKEvent,
    input: UpdateEventInput,
) -> EventKitResult<()> {
    if let Some(summary) = input.summary {
        let ns = NSString::from_str(&summary);
        unsafe { event.setTitle(Some(&ns)) };
    }
    if let Some(description) = input.description {
        match description {
            Some(value) => {
                let ns = NSString::from_str(&value);
                unsafe { event.setNotes(Some(&ns)) };
            }
            None => unsafe { event.setNotes(None) },
        }
    }
    if input.start.is_some() || input.end.is_some() || input.all_day.is_some() {
        let existing = (
            retained_date_to_unix(&unsafe { event.startDate() }),
            retained_date_to_unix(&unsafe { event.endDate() }),
        );
        let (start, end) = merged_range(existing, input.start, input.end)?;
        let all_day = input.all_day.unwrap_or_else(|| unsafe { event.isAllDay() });
        apply_dates(event, start, end, all_day)?;
    }
    if let Some(url) = input.url {
        match url {
            Some(value) => {
                let ns = NSURL::URLWithString(&NSString::from_str(&value))
                    .ok_or_else(|| EventKitError::ValidationFailed("invalid event url".into()))?;
                unsafe { event.setURL(Some(&ns)) };
            }
            None => unsafe { event.setURL(None) },
        }
    }
    let _ = input.status;
    if let Some(calendar_hint) = input.calendar_hint {
        let calendar = resolve_event_calendar(store, &calendar_hint)?;
        unsafe { event.setCalendar(Some(&calendar)) };
    }
    if let Some(location) = input.location {
        apply_structured_location(event, location.as_ref())?;
    }
    if let Some(alarms) = input.alarms {
        apply_alarms_to_item(event, &alarms)?;
    }
    if let Some(recurrence) = input.recurrence {
        match recurrence {
            Some(rule) => apply_recurrence_to_item(event, std::slice::from_ref(&rule))?,
            None => unsafe { event.setRecurrenceRules(None) },
        }
    }
    Ok(())
}

fn apply_dates(event: &EKEvent, start: i64, end: i64, all_day: bool) -> EventKitResult<()> {
    // EventKit reads an all-day event's dates as calendar days in the default timezone, so an
    // instant taken from anywhere else in the day can name the day either side of the one the
    // caller meant. Snapping both ends to local midnight pins the days that get stored.
    let (start, end) = if all_day {
        (start_of_local_day(start)?, start_of_local_day(end)?)
    } else {
        (start, end)
    };
    let start_date = unix_to_ns_date(start)?;
    let end_date = unix_to_ns_date(end)?;
    unsafe { event.setAllDay(all_day) };
    unsafe { event.setStartDate(Some(&start_date)) };
    unsafe { event.setEndDate(Some(&end_date)) };
    Ok(())
}

fn apply_structured_location(
    event: &EKEvent,
    location: Option<&LocationInput>,
) -> EventKitResult<()> {
    match location {
        Some(value) => {
            let structured = unsafe {
                EKStructuredLocation::locationWithTitle(&NSString::from_str(
                    &value.title.clone().unwrap_or_default(),
                ))
            };
            if let (Some(lat), Some(lng)) = (value.latitude, value.longitude) {
                let cl = unsafe {
                    CLLocation::initWithLatitude_longitude(CLLocation::alloc(), lat, lng)
                };
                unsafe { structured.setGeoLocation(Some(&cl)) };
            }
            unsafe { event.setStructuredLocation(Some(&structured)) };
        }
        None => unsafe { event.setStructuredLocation(None) },
    }
    Ok(())
}

/// Merges a partial update's dates with the stored ones, then checks the result.
///
/// A start-only or end-only update inverts the range against dates the request never mentioned,
/// so validating the request on its own cannot catch it — the check has to happen here, after the
/// stored event has been read.
fn merged_range(
    existing: (i64, i64),
    start: Option<i64>,
    end: Option<i64>,
) -> EventKitResult<(i64, i64)> {
    let (existing_start, existing_end) = existing;
    let start = start.unwrap_or(existing_start);
    let end = end.unwrap_or(existing_end);
    validate_range(start, end)?;
    Ok((start, end))
}

fn validate_range(start: i64, end: i64) -> EventKitResult<()> {
    if end < start {
        return Err(EventKitError::EndBeforeStart);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const STORED: (i64, i64) = (1_000, 2_000);

    #[test]
    fn end_before_start_is_invalid() {
        assert_eq!(validate_range(10, 5), Err(EventKitError::EndBeforeStart));
    }

    #[test]
    fn an_empty_range_is_valid() {
        assert_eq!(validate_range(10, 10), Ok(()));
    }

    #[test]
    fn moving_start_past_the_stored_end_is_rejected() {
        assert_eq!(
            merged_range(STORED, Some(3_000), None),
            Err(EventKitError::EndBeforeStart)
        );
    }

    #[test]
    fn moving_end_before_the_stored_start_is_rejected() {
        assert_eq!(
            merged_range(STORED, None, Some(500)),
            Err(EventKitError::EndBeforeStart)
        );
    }

    #[test]
    fn a_partial_update_keeps_the_field_it_does_not_mention() {
        assert_eq!(merged_range(STORED, Some(1_500), None), Ok((1_500, 2_000)));
        assert_eq!(merged_range(STORED, None, Some(2_500)), Ok((1_000, 2_500)));
        assert_eq!(merged_range(STORED, None, None), Ok(STORED));
    }

    #[test]
    fn both_fields_together_are_still_checked() {
        assert_eq!(
            merged_range(STORED, Some(9_000), Some(8_000)),
            Err(EventKitError::EndBeforeStart)
        );
        assert_eq!(
            merged_range(STORED, Some(8_000), Some(9_000)),
            Ok((8_000, 9_000))
        );
    }
}
