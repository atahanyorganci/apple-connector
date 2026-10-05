//! Read-only probe: how the identifiers stored in the Calendar and Reminders SQLite databases map
//! onto EventKit lookups.
//!
//! Nothing is created, changed, or deleted. The databases are opened with `sqlite3 -readonly`, and
//! only EventKit lookup methods are called. Output is counts only — no titles, no identifiers.
//!
//! ```bash
//! cargo test -p apple-eventkit --test identifier_probe -- --ignored --nocapture
//! ```
//!
//! Requires Full Disk Access (for the SQLite files) and Calendars + Reminders access already granted
//! to the terminal; the probe never prompts. See `docs/lessons/` for what it established.

#![allow(unsafe_code)]

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use objc2_event_kit::{EKAuthorizationStatus, EKEntityType, EKEventStore};
use objc2_foundation::NSString;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn home() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(PathBuf::from(std::env::var("HOME")?))
}

fn sqlite_rows(db: &Path, sql: &str) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    let output = Command::new("sqlite3")
        .arg("-readonly")
        .arg("-separator")
        .arg("\t")
        .arg(db)
        .arg(sql)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "sqlite3 failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?
        .lines()
        .map(|line| line.split('\t').map(str::to_owned).collect())
        .collect())
}

/// `hex(ZIDENTIFIER)` → `XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX`, the shape the read API lowercases.
fn dashed_uuid(hex: &str) -> Option<String> {
    if hex.len() != 32 {
        return None;
    }
    Some(format!(
        "{}-{}-{}-{}-{}",
        hex.get(0..8)?,
        hex.get(8..12)?,
        hex.get(12..16)?,
        hex.get(16..20)?,
        hex.get(20..32)?
    ))
}

fn granted(entity: EKEntityType) -> bool {
    let status = unsafe { EKEventStore::authorizationStatusForEntityType(entity) };
    !(status == EKAuthorizationStatus::NotDetermined
        || status == EKAuthorizationStatus::Restricted
        || status == EKAuthorizationStatus::Denied
        || status == EKAuthorizationStatus::WriteOnly)
}

fn column(row: &[String], index: usize) -> &str {
    row.get(index).map(String::as_str).unwrap_or_default()
}

#[derive(Default, Debug)]
struct Tally {
    rows: usize,
    exact_found: usize,
    has_uppercase: usize,
    lowercase_found: usize,
    external_present: usize,
    external_found_same: usize,
}

/// Calendars: `Calendar.UUID` against `calendarWithIdentifier:`, in stored and lowercased case,
/// and `Calendar.external_id` against the same lookup.
fn probe_calendars(store: &EKEventStore, db: &Path) -> Result<Tally, Box<dyn std::error::Error>> {
    let mut tally = Tally::default();
    for row in sqlite_rows(db, "SELECT UUID, ifnull(external_id, '') FROM Calendar")? {
        let uuid = column(&row, 0);
        let external = column(&row, 1);
        tally.rows += 1;
        let exact = unsafe { store.calendarWithIdentifier(&NSString::from_str(uuid)) };
        if exact.is_some() {
            tally.exact_found += 1;
        }
        if uuid != uuid.to_lowercase() {
            tally.has_uppercase += 1;
            let lower =
                unsafe { store.calendarWithIdentifier(&NSString::from_str(&uuid.to_lowercase())) };
            if lower.is_some() {
                tally.lowercase_found += 1;
            }
        }
        if !external.is_empty() {
            tally.external_present += 1;
            if let Some(calendar) =
                unsafe { store.calendarWithIdentifier(&NSString::from_str(external)) }
                && unsafe { calendar.calendarIdentifier().to_string() } == uuid
            {
                tally.external_found_same += 1;
            }
        }
    }
    Ok(tally)
}

/// Events: `CalendarItem.UUID` against `calendarItemWithIdentifier:`, and
/// `CalendarItem.unique_identifier` against `calendarItemsWithExternalIdentifier:`.
fn probe_events(store: &EKEventStore, db: &Path) -> Result<Tally, Box<dyn std::error::Error>> {
    let mut tally = Tally::default();
    let sql = "SELECT UUID, ifnull(unique_identifier, '') FROM CalendarItem \
               WHERE UUID IS NOT NULL ORDER BY ROWID DESC LIMIT 100";
    for row in sqlite_rows(db, sql)? {
        let uuid = column(&row, 0);
        let unique = column(&row, 1);
        tally.rows += 1;
        if unsafe { store.calendarItemWithIdentifier(&NSString::from_str(uuid)) }.is_some() {
            tally.exact_found += 1;
        }
        if uuid != uuid.to_lowercase() {
            tally.has_uppercase += 1;
            if unsafe {
                store.calendarItemWithIdentifier(&NSString::from_str(&uuid.to_lowercase()))
            }
            .is_some()
            {
                tally.lowercase_found += 1;
            }
        }
        if !unique.is_empty() {
            tally.external_present += 1;
            let items =
                unsafe { store.calendarItemsWithExternalIdentifier(&NSString::from_str(unique)) };
            if items
                .iter()
                .any(|item| unsafe { item.calendarItemIdentifier().to_string() } == uuid)
            {
                tally.external_found_same += 1;
            }
        }
    }
    Ok(tally)
}

fn reminder_stores() -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let dir =
        home()?.join("Library/Group Containers/group.com.apple.reminders/Container_v1/Stores");
    let mut stores = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if name.starts_with("Data-") && name.ends_with(".sqlite") {
            stores.push(path);
        }
    }
    stores.sort();
    Ok(stores)
}

/// Reminder lists: `ZREMCDBASELIST.ZIDENTIFIER` (as a dashed UUID) and `ZEXTERNALIDENTIFIER`
/// against `calendarWithIdentifier:`.
fn probe_reminder_lists(
    store: &EKEventStore,
    db: &Path,
) -> Result<Tally, Box<dyn std::error::Error>> {
    let mut tally = Tally::default();
    let sql = "SELECT hex(ZIDENTIFIER), ifnull(ZEXTERNALIDENTIFIER, '') FROM ZREMCDBASELIST \
               WHERE ZMARKEDFORDELETION = 0 AND ZIDENTIFIER IS NOT NULL";
    for row in sqlite_rows(db, sql)? {
        let Some(upper) = dashed_uuid(column(&row, 0)) else {
            continue;
        };
        let external = column(&row, 1);
        tally.rows += 1;
        tally.has_uppercase += 1;
        let exact = unsafe { store.calendarWithIdentifier(&NSString::from_str(&upper)) };
        if exact.is_some() {
            tally.exact_found += 1;
        }
        if unsafe { store.calendarWithIdentifier(&NSString::from_str(&upper.to_lowercase())) }
            .is_some()
        {
            tally.lowercase_found += 1;
        }
        if !external.is_empty() {
            tally.external_present += 1;
            if let Some(calendar) =
                unsafe { store.calendarWithIdentifier(&NSString::from_str(external)) }
                && unsafe { calendar.calendarIdentifier().to_string() } == upper
            {
                tally.external_found_same += 1;
            }
        }
    }
    Ok(tally)
}

#[derive(Default, Debug)]
struct ReminderTally {
    base: Tally,
    /// Items found by the uppercase id whose `calendarItemExternalIdentifier`, lowercased, equals
    /// the read API's `ReminderId` — the condition `hydrate_reminder` relies on.
    external_identifier_is_api_id: usize,
    /// Items found by the uppercase id whose `calendarItemExternalIdentifier` equals
    /// `ZEXTERNALIDENTIFIER`.
    external_identifier_is_stored_external: usize,
}

/// Reminders: `ZREMCDREMINDER.ZIDENTIFIER` and `ZEXTERNALIDENTIFIER` against
/// `calendarItemWithIdentifier:` and `calendarItemsWithExternalIdentifier:`.
fn probe_reminders(
    store: &EKEventStore,
    db: &Path,
) -> Result<ReminderTally, Box<dyn std::error::Error>> {
    let mut tally = ReminderTally::default();
    let sql = "SELECT hex(ZIDENTIFIER), ifnull(ZEXTERNALIDENTIFIER, '') FROM ZREMCDREMINDER \
               WHERE ZMARKEDFORDELETION = 0 AND ZIDENTIFIER IS NOT NULL \
               ORDER BY Z_PK DESC LIMIT 100";
    for row in sqlite_rows(db, sql)? {
        let Some(upper) = dashed_uuid(column(&row, 0)) else {
            continue;
        };
        let external = column(&row, 1);
        tally.base.rows += 1;
        tally.base.has_uppercase += 1;
        let found = unsafe { store.calendarItemWithIdentifier(&NSString::from_str(&upper)) };
        if let Some(item) = &found {
            tally.base.exact_found += 1;
            let item_external = unsafe { item.calendarItemExternalIdentifier() }
                .map(|value| value.to_string())
                .unwrap_or_default();
            if item_external.to_lowercase() == upper.to_lowercase() {
                tally.external_identifier_is_api_id += 1;
            }
            if !external.is_empty() && item_external == external {
                tally.external_identifier_is_stored_external += 1;
            }
        }
        if unsafe { store.calendarItemWithIdentifier(&NSString::from_str(&upper.to_lowercase())) }
            .is_some()
        {
            tally.base.lowercase_found += 1;
        }
        if !external.is_empty() {
            tally.base.external_present += 1;
            let items =
                unsafe { store.calendarItemsWithExternalIdentifier(&NSString::from_str(external)) };
            if items
                .iter()
                .any(|item| unsafe { item.calendarItemIdentifier().to_string() } == upper)
            {
                tally.base.external_found_same += 1;
            }
        }
    }
    Ok(tally)
}

#[test]
#[ignore = "read-only probe of live Calendar/Reminders data; needs Full Disk Access and EventKit access"]
fn identifier_spaces() -> TestResult {
    if !granted(EKEntityType::Event) || !granted(EKEntityType::Reminder) {
        return Err(
            "Calendars and Reminders access must already be granted; the probe never prompts"
                .into(),
        );
    }
    let store = unsafe { EKEventStore::new() };
    let calendar_db =
        home()?.join("Library/Group Containers/group.com.apple.calendar/Calendar.sqlitedb");

    let calendars = probe_calendars(&store, &calendar_db)?;
    println!("calendars: {calendars:?}");
    let events = probe_events(&store, &calendar_db)?;
    println!("events: {events:?}");

    // Event calendars: `Calendar.UUID` is `calendarIdentifier` exactly, and the lookup is
    // case-sensitive, so the read API's lowercased `CalendarId` never resolves. `external_id` is
    // the server's id for the calendar, not an EventKit identifier.
    ensure(
        calendars.exact_found > 0,
        "no calendar resolved by its stored UUID",
    )?;
    ensure(
        calendars.lowercase_found == 0,
        "a lowercased Calendar.UUID resolved; calendar lookups are no longer case-sensitive",
    )?;
    ensure(
        calendars.external_found_same == 0,
        "Calendar.external_id resolved through calendarWithIdentifier:",
    )?;

    // Events: same case-sensitivity for `calendarItemWithIdentifier:`; the iCalendar UID in
    // `CalendarItem.unique_identifier` is what `calendarItemsWithExternalIdentifier:` matches.
    ensure(
        events.exact_found > 0,
        "no event resolved by its stored UUID",
    )?;
    ensure(
        events.lowercase_found == 0,
        "a lowercased CalendarItem.UUID resolved; item lookups are no longer case-sensitive",
    )?;
    ensure(
        events.external_found_same > 0,
        "no event resolved through unique_identifier",
    )?;

    for (index, db) in reminder_stores()?.iter().enumerate() {
        let lists = probe_reminder_lists(&store, db)?;
        println!("reminder store {index} lists: {lists:?}");
        let reminders = probe_reminders(&store, db)?;
        println!("reminder store {index} reminders: {reminders:?}");

        // Reminder lists and reminders resolve by their identifier in either case, and a
        // reminder's external identifier is the read API's `ReminderId`.
        ensure(
            lists.lowercase_found == lists.exact_found,
            "reminder list lookups became case-sensitive",
        )?;
        ensure(
            reminders.base.lowercase_found == reminders.base.exact_found,
            "reminder lookups became case-sensitive",
        )?;
        ensure(
            reminders.external_identifier_is_api_id == reminders.base.exact_found,
            "a reminder's calendarItemExternalIdentifier is not its ReminderId",
        )?;
    }
    Ok(())
}

fn ensure(condition: bool, message: &str) -> TestResult {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
