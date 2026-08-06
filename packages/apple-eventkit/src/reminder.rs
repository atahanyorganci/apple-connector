use objc2_event_kit::{EKEventStore, EKReminder};
use objc2_foundation::{NSString, NSURL};

use crate::{
    alarm::{AlarmInput, apply_alarms_to_item},
    calendar_resolve::{ReminderListResolveHint, resolve_reminder_list},
    datetime::unix_to_date_components,
    error::{EventKitError, EventKitResult},
    item_lookup::lookup_reminder,
    recurrence::{RecurrenceInput, apply_recurrence_to_item},
    store::EventKitStore,
};

#[derive(Debug, Clone)]
pub struct DueInput {
    pub at: i64,
    pub all_day: bool,
}

#[derive(Debug, Clone)]
pub struct LocationInput {
    pub title: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct CreateReminderInput {
    pub title: String,
    pub notes: Option<String>,
    pub due: Option<DueInput>,
    pub completed: Option<bool>,
    pub priority: Option<i64>,
    pub url: Option<String>,
    pub location: Option<LocationInput>,
    pub alarms: Vec<AlarmInput>,
    pub recurrence: Option<RecurrenceInput>,
}

#[derive(Debug, Clone)]
pub struct UpdateReminderInput {
    pub title: Option<String>,
    pub notes: Option<Option<String>>,
    pub due: Option<Option<DueInput>>,
    pub completed: Option<bool>,
    pub priority: Option<i64>,
    pub url: Option<Option<String>>,
    pub list_hint: Option<ReminderListResolveHint>,
    pub location: Option<Option<LocationInput>>,
    pub alarms: Option<Vec<AlarmInput>>,
    pub recurrence: Option<Option<RecurrenceInput>>,
}

#[derive(Debug, Clone)]
pub struct SavedReminder {
    pub external_id: String,
    pub calendar_item_id: String,
}

impl EventKitStore {
    pub async fn create_reminder(
        &self,
        list_hint: ReminderListResolveHint,
        input: CreateReminderInput,
    ) -> EventKitResult<SavedReminder> {
        self.ensure_reminders()?;
        self.run_on_main(move |store| {
            let calendar = resolve_reminder_list(store, &list_hint)?;
            let reminder = unsafe { EKReminder::reminderWithEventStore(store) };
            unsafe { reminder.setCalendar(Some(&calendar)) };
            apply_create_fields(&reminder, &input)?;
            save_reminder(store, &reminder)
        })
        .await
    }

    pub async fn update_reminder(
        &self,
        api_id: &str,
        external_id: Option<&str>,
        input: UpdateReminderInput,
    ) -> EventKitResult<SavedReminder> {
        self.ensure_reminders()?;
        let api_id = api_id.to_owned();
        let external_id = external_id.map(str::to_owned);
        self.run_on_main(move |store| {
            let reminder = lookup_reminder(store, &api_id, external_id.as_deref())?;
            apply_update_fields(store, &reminder, input)?;
            save_reminder(store, &reminder)
        })
        .await
    }

    pub async fn delete_reminder(
        &self,
        api_id: &str,
        external_id: Option<&str>,
    ) -> EventKitResult<()> {
        self.ensure_reminders()?;
        let api_id = api_id.to_owned();
        let external_id = external_id.map(str::to_owned);
        self.run_on_main(move |store| {
            let reminder = lookup_reminder(store, &api_id, external_id.as_deref())?;
            match unsafe { store.removeReminder_commit_error(&reminder, true) } {
                Ok(()) => Ok(()),
                Err(err) => Err(EventKitError::Framework(
                    err.localizedDescription().to_string(),
                )),
            }
        })
        .await
    }
}

fn save_reminder(store: &EKEventStore, reminder: &EKReminder) -> EventKitResult<SavedReminder> {
    match unsafe { store.saveReminder_commit_error(reminder, true) } {
        Ok(()) => {
            let external_id = unsafe {
                reminder
                    .calendarItemExternalIdentifier()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| reminder.calendarItemIdentifier().to_string())
            };
            let calendar_item_id = unsafe { reminder.calendarItemIdentifier().to_string() };
            Ok(SavedReminder {
                external_id,
                calendar_item_id,
            })
        }
        Err(err) => Err(EventKitError::Framework(
            err.localizedDescription().to_string(),
        )),
    }
}

fn apply_create_fields(reminder: &EKReminder, input: &CreateReminderInput) -> EventKitResult<()> {
    validate_priority(input.priority)?;
    let title = NSString::from_str(&input.title);
    unsafe { reminder.setTitle(Some(&title)) };
    if let Some(notes) = &input.notes {
        let ns = NSString::from_str(notes);
        unsafe { reminder.setNotes(Some(&ns)) };
    }
    if let Some(due) = &input.due {
        let components = unix_to_date_components(due.at, due.all_day)?;
        unsafe { reminder.setDueDateComponents(Some(&components)) };
    }
    if let Some(completed) = input.completed {
        unsafe { reminder.setCompleted(completed) };
    }
    if let Some(priority) = input.priority {
        unsafe { reminder.setPriority(priority_as_usize(priority)?) };
    }
    if let Some(url) = &input.url {
        let ns = NSURL::URLWithString(&NSString::from_str(url))
            .ok_or_else(|| EventKitError::ValidationFailed("invalid reminder url".into()))?;
        unsafe { reminder.setURL(Some(&ns)) };
    }
    apply_location(reminder, input.location.as_ref())?;
    apply_alarms_to_item(reminder, &input.alarms)?;
    if let Some(recurrence) = &input.recurrence {
        apply_recurrence_to_item(reminder, std::slice::from_ref(recurrence))?;
    }
    Ok(())
}

fn apply_update_fields(
    store: &EKEventStore,
    reminder: &EKReminder,
    input: UpdateReminderInput,
) -> EventKitResult<()> {
    if let Some(title) = input.title {
        let ns = NSString::from_str(&title);
        unsafe { reminder.setTitle(Some(&ns)) };
    }
    if let Some(notes) = input.notes {
        match notes {
            Some(value) => {
                let ns = NSString::from_str(&value);
                unsafe { reminder.setNotes(Some(&ns)) };
            }
            None => unsafe { reminder.setNotes(None) },
        }
    }
    if let Some(due) = input.due {
        match due {
            Some(value) => {
                let components = unix_to_date_components(value.at, value.all_day)?;
                unsafe { reminder.setDueDateComponents(Some(&components)) };
            }
            None => unsafe { reminder.setDueDateComponents(None) },
        }
    }
    if let Some(completed) = input.completed {
        unsafe { reminder.setCompleted(completed) };
    }
    if let Some(priority) = input.priority {
        validate_priority(Some(priority))?;
        unsafe { reminder.setPriority(priority_as_usize(priority)?) };
    }
    if let Some(url) = input.url {
        match url {
            Some(value) => {
                let ns = NSURL::URLWithString(&NSString::from_str(&value)).ok_or_else(|| {
                    EventKitError::ValidationFailed("invalid reminder url".into())
                })?;
                unsafe { reminder.setURL(Some(&ns)) };
            }
            None => unsafe { reminder.setURL(None) },
        }
    }
    if let Some(list_hint) = input.list_hint {
        let calendar = resolve_reminder_list(store, &list_hint)?;
        unsafe { reminder.setCalendar(Some(&calendar)) };
    }
    if let Some(location) = input.location {
        apply_location(reminder, location.as_ref())?;
    }
    if let Some(alarms) = input.alarms {
        apply_alarms_to_item(reminder, &alarms)?;
    }
    if let Some(recurrence) = input.recurrence {
        match recurrence {
            Some(rule) => apply_recurrence_to_item(reminder, std::slice::from_ref(&rule))?,
            None => unsafe { reminder.setRecurrenceRules(None) },
        }
    }
    Ok(())
}

fn apply_location(reminder: &EKReminder, location: Option<&LocationInput>) -> EventKitResult<()> {
    match location {
        Some(value) => {
            let text = value.title.clone().unwrap_or_default();
            let ns = NSString::from_str(&text);
            unsafe { reminder.setLocation(Some(&ns)) };
        }
        None => unsafe { reminder.setLocation(None) },
    }
    Ok(())
}

fn validate_priority(priority: Option<i64>) -> EventKitResult<()> {
    if let Some(value) = priority
        && !(0..=9).contains(&value)
    {
        return Err(EventKitError::ValidationFailed(
            "priority must be between 0 and 9".into(),
        ));
    }
    Ok(())
}

fn priority_as_usize(priority: i64) -> EventKitResult<usize> {
    validate_priority(Some(priority))?;
    usize::try_from(priority)
        .map_err(|_| EventKitError::ValidationFailed("priority must be between 0 and 9".into()))
}
