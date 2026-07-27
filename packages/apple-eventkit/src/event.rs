use objc2::AnyThread;
use objc2_core_location::CLLocation;
use objc2_event_kit::{EKEvent, EKEventStore, EKSpan, EKStructuredLocation};
use objc2_foundation::{NSString, NSURL};

use crate::{
    alarm::{AlarmInput, apply_alarms_to_item},
    calendar_resolve::{CalendarResolveHint, resolve_event_calendar},
    datetime::{retained_date_to_unix, unix_to_ns_date},
    error::{EventKitError, EventKitResult},
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
        self.ensure_events()?;
        validate_range(input.start, input.end)?;
        self.run_on_main(move |store| {
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
        self.ensure_events()?;
        if let (Some(start), Some(end)) = (input.start, input.end) {
            validate_range(start, end)?;
        }
        let api_id = api_id.to_owned();
        let external_id = external_id.map(str::to_owned);
        self.run_on_main(move |store| {
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
        self.ensure_events()?;
        let api_id = api_id.to_owned();
        let external_id = external_id.map(str::to_owned);
        self.run_on_main(move |store| {
            let event = lookup_event(
                store,
                &api_id,
                external_id.as_deref(),
                input.occurrence_start,
            )?;
            match unsafe { store.removeEvent_span_error(&event, input.span.to_ek()) } {
                Ok(()) => Ok(()),
                Err(err) => Err(EventKitError::Framework(
                    err.localizedDescription().to_string(),
                )),
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
        Err(err) => Err(EventKitError::Framework(
            err.localizedDescription().to_string(),
        )),
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
        let start = input
            .start
            .unwrap_or_else(|| retained_date_to_unix(&unsafe { event.startDate() }));
        let end = input
            .end
            .unwrap_or_else(|| retained_date_to_unix(&unsafe { event.endDate() }));
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

fn validate_range(start: i64, end: i64) -> EventKitResult<()> {
    if end < start {
        return Err(EventKitError::ValidationFailed(
            "end must be greater than or equal to start".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_before_start_is_invalid() {
        assert!(validate_range(10, 5).is_err());
    }
}
