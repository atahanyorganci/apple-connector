use sqlx::SqlitePool;

use super::{
    assembly::{
        account_from_row, calendar_detail_from_row, calendar_summary_from_row,
        event_detail_from_row, event_summary_from_row,
    },
    model::{
        CalendarAccount, CalendarDetail, CalendarSummary, EventAttachment, EventDetail,
        EventSummary,
    },
    queries::{
        fetch_alarms_by_event_id, fetch_attachment_by_event_and_id, fetch_attachments_by_owner_id,
        fetch_calendar_by_id, fetch_calendar_resolve_metadata, fetch_calendars_page,
        fetch_direct_events_page, fetch_event_by_id, fetch_event_external_id,
        fetch_exception_dates_by_owner_id, fetch_location_by_id, fetch_occurrence_events_page,
        fetch_participants_by_owner_id, fetch_recurrence_by_owner_id, fetch_stores_ordered,
    },
    row::{EventRow, core_data_secs_from_timestamp},
    search::EventFilters,
};
use crate::api::cursor::{
    CalendarEventCursor, CalendarListCursor, EventSearchCursor, GlobalEventCursor, decode, encode,
};

#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CalendarResolveMetadata {
    pub api_id: String,
    pub external_id: Option<String>,
    pub title: Option<String>,
    pub store_type: i64,
}

pub struct CalendarRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> CalendarRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_accounts(&self) -> Result<Vec<CalendarAccount>, sqlx::Error> {
        crate::db::run_timed_query(|| async {
            let rows = fetch_stores_ordered(self.pool).await?;
            Ok(rows.into_iter().map(account_from_row).collect())
        })
        .await
    }

    pub async fn list_calendars(
        &self,
        limit: u32,
        cursor: Option<CalendarListCursor>,
    ) -> Result<Page<CalendarSummary>, sqlx::Error> {
        crate::db::run_timed_query(|| self.list_calendars_inner(limit, cursor)).await
    }

    async fn list_calendars_inner(
        &self,
        limit: u32,
        cursor: Option<CalendarListCursor>,
    ) -> Result<Page<CalendarSummary>, sqlx::Error> {
        let fetch_limit = i64::from(limit) + 1;
        let rows = fetch_calendars_page(self.pool, cursor.map(|c| c.row_id), fetch_limit).await?;
        let (rows, has_more) = split_page(rows, limit);
        let next_cursor = has_more
            .then(|| {
                rows.last()
                    .map(|row| encode(&CalendarListCursor { row_id: row.row_id }).ok())
            })
            .flatten()
            .flatten();
        Ok(Page {
            items: rows.into_iter().map(calendar_summary_from_row).collect(),
            has_more,
            next_cursor,
        })
    }

    pub async fn get_calendar(
        &self,
        calendar_id: &str,
    ) -> Result<Option<CalendarDetail>, sqlx::Error> {
        crate::db::run_timed_query(|| async {
            let row = fetch_calendar_by_id(self.pool, calendar_id).await?;
            Ok(row.map(calendar_detail_from_row))
        })
        .await
    }

    pub async fn get_calendar_resolve_metadata(
        &self,
        calendar_id: &str,
    ) -> Result<Option<CalendarResolveMetadata>, sqlx::Error> {
        let row = fetch_calendar_resolve_metadata(self.pool, calendar_id).await?;

        Ok(row.map(|row| CalendarResolveMetadata {
            api_id: row.api_id,
            external_id: row.external_id,
            title: row.title,
            store_type: row.store_type.unwrap_or(0),
        }))
    }

    pub async fn get_event_external_id(
        &self,
        event_id: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        fetch_event_external_id(self.pool, event_id).await
    }

    pub async fn list_events(
        &self,
        filters: &EventFilters,
        limit: u32,
        cursor: Option<GlobalEventCursor>,
    ) -> Result<Page<EventSummary>, sqlx::Error> {
        crate::db::run_timed_query(|| self.list_events_inner(filters, limit, cursor)).await
    }

    async fn list_events_inner(
        &self,
        filters: &EventFilters,
        limit: u32,
        cursor: Option<GlobalEventCursor>,
    ) -> Result<Page<EventSummary>, sqlx::Error> {
        let use_occurrence = filters.start_after.is_some() || filters.start_before.is_some();
        if use_occurrence {
            self.list_occurrence_events(filters, limit, cursor).await
        } else {
            self.list_direct_events(filters, limit, cursor).await
        }
    }

    async fn list_direct_events(
        &self,
        filters: &EventFilters,
        limit: u32,
        cursor: Option<GlobalEventCursor>,
    ) -> Result<Page<EventSummary>, sqlx::Error> {
        let fetch_limit = i64::from(limit) + 1;
        let binds = filters.bind_values(
            cursor.as_ref().map(|c| c.modified_at),
            cursor.map(|c| c.row_id),
            fetch_limit,
        );
        let rows = fetch_direct_events_page(self.pool, &binds).await?;
        let (rows, has_more) = split_page(rows, limit);
        let next_cursor = has_more
            .then(|| {
                rows.last().map(|row| {
                    encode(&GlobalEventCursor {
                        modified_at: row.last_modified.unwrap_or(0.0),
                        row_id: row.row_id,
                    })
                    .ok()
                })
            })
            .flatten()
            .flatten();
        Ok(Page {
            items: rows.into_iter().map(event_summary_from_row).collect(),
            has_more,
            next_cursor,
        })
    }

    async fn list_occurrence_events(
        &self,
        filters: &EventFilters,
        limit: u32,
        cursor: Option<GlobalEventCursor>,
    ) -> Result<Page<EventSummary>, sqlx::Error> {
        let fetch_limit = i64::from(limit) + 1;
        let binds = filters.bind_values(
            cursor.as_ref().map(|c| c.modified_at),
            cursor.map(|c| c.row_id),
            fetch_limit,
        );
        let rows = fetch_occurrence_events_page(self.pool, &binds).await?;
        let (rows, has_more) = split_page(rows, limit);
        let next_cursor = has_more
            .then(|| {
                rows.last().map(|row| {
                    encode(&GlobalEventCursor {
                        modified_at: row
                            .occurrence_start
                            .unwrap_or(row.start_date.unwrap_or(0.0)),
                        row_id: row.row_id,
                    })
                    .ok()
                })
            })
            .flatten()
            .flatten();
        Ok(Page {
            items: rows.into_iter().map(event_summary_from_row).collect(),
            has_more,
            next_cursor,
        })
    }

    pub async fn list_calendar_events(
        &self,
        calendar_id: &str,
        filters: &EventFilters,
        limit: u32,
        cursor: Option<CalendarEventCursor>,
    ) -> Result<Page<EventSummary>, sqlx::Error> {
        let mut scoped = filters.clone();
        scoped.calendar_id = Some(calendar_id.to_owned());
        let global_cursor = cursor.map(|c| GlobalEventCursor {
            modified_at: c.start_at,
            row_id: c.row_id,
        });
        let page = self.list_events(&scoped, limit, global_cursor).await?;
        Ok(Page {
            items: page.items,
            has_more: page.has_more,
            next_cursor: reencode_calendar_event_cursor(page.next_cursor),
        })
    }

    pub async fn search_events(
        &self,
        filters: &EventFilters,
        limit: u32,
        cursor: Option<EventSearchCursor>,
    ) -> Result<Page<EventSummary>, sqlx::Error> {
        let global_cursor = cursor.map(|c| GlobalEventCursor {
            modified_at: c.start_at,
            row_id: c.row_id,
        });
        self.list_events(filters, limit, global_cursor).await
    }

    pub async fn get_event(&self, event_id: &str) -> Result<Option<EventDetail>, sqlx::Error> {
        crate::db::run_timed_query(|| self.get_event_inner(event_id)).await
    }

    async fn get_event_inner(&self, event_id: &str) -> Result<Option<EventDetail>, sqlx::Error> {
        let row = fetch_event_by_id(self.pool, event_id).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(self.hydrate_event(row).await?))
    }

    async fn hydrate_event(&self, row: EventRow) -> Result<EventDetail, sqlx::Error> {
        let event_row_id = row.row_id;
        let location = if let Some(location_id) = row.location_id.filter(|id| *id > 0) {
            fetch_location_by_id(self.pool, location_id).await?
        } else {
            None
        };

        let participants = fetch_participants_by_owner_id(self.pool, event_row_id).await?;
        let organizer = row
            .organizer_id
            .and_then(|org_id| participants.iter().find(|p| p.row_id == org_id).cloned());
        let attendees: Vec<_> = participants
            .into_iter()
            .filter(|p| Some(p.row_id) != row.organizer_id)
            .collect();

        let recurrence = fetch_recurrence_by_owner_id(self.pool, event_row_id).await?;
        let exception_rows = fetch_exception_dates_by_owner_id(self.pool, event_row_id).await?;
        let alarms = fetch_alarms_by_event_id(self.pool, event_row_id).await?;
        let attachments = fetch_attachments_by_owner_id(self.pool, event_row_id).await?;

        Ok(event_detail_from_row(
            row,
            location,
            organizer,
            attendees,
            recurrence,
            exception_rows.into_iter().map(|r| r.date).collect(),
            alarms,
            attachments,
        ))
    }

    pub async fn get_attachment(
        &self,
        event_id: &str,
        attachment_id: &str,
    ) -> Result<Option<EventAttachment>, sqlx::Error> {
        crate::db::run_timed_query(|| async {
            let row = fetch_attachment_by_event_and_id(self.pool, event_id, attachment_id).await?;
            Ok(row.map(super::assembly::attachment_from_row))
        })
        .await
    }
}

fn reencode_calendar_event_cursor(cursor: Option<String>) -> Option<String> {
    cursor.and_then(|value| {
        decode::<GlobalEventCursor>(&value).ok().and_then(|global| {
            encode(&CalendarEventCursor {
                start_at: global.modified_at,
                row_id: global.row_id,
            })
            .ok()
        })
    })
}

fn split_page<T>(mut rows: Vec<T>, limit: u32) -> (Vec<T>, bool) {
    let has_more = rows.len() > limit as usize;
    if has_more {
        rows.truncate(limit as usize);
    }
    (rows, has_more)
}

pub fn unix_to_core_data_secs(unix: i64) -> f64 {
    use chrono::TimeZone;
    match chrono::Utc.timestamp_opt(unix, 0).single() {
        Some(dt) => core_data_secs_from_timestamp(dt),
        None => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::{CalendarRepository, unix_to_core_data_secs};
    use crate::{
        api::cursor::decode,
        connect_pool,
        fixtures::{CalendarFixtureDb, SEED_EVENT_ID},
    };

    #[tokio::test]
    async fn lists_seeded_events() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = CalendarFixtureDb::seeded().await?;
        let pool = connect_pool(fixture.path()).await?;
        let repo = CalendarRepository::new(&pool);
        let page = repo.list_events(&Default::default(), 10, None).await?;
        assert!(!page.items.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn calendar_event_cursor_round_trips_through_pagination()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = CalendarFixtureDb::seeded().await?;
        let pool = connect_pool(fixture.path()).await?;
        let repo = CalendarRepository::new(&pool);
        let calendars = repo.list_calendars(1, None).await?;
        let calendar_id = calendars.items[0].id.clone();

        let first = repo
            .list_calendar_events(&calendar_id, &Default::default(), 1, None)
            .await?;
        assert!(first.has_more);
        let cursor = first.next_cursor.ok_or("missing cursor")?;

        let second = repo
            .list_calendar_events(&calendar_id, &Default::default(), 1, Some(decode(&cursor)?))
            .await?;
        assert_ne!(first.items[0].id, second.items[0].id);
        Ok(())
    }

    #[tokio::test]
    async fn gets_event_detail() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = CalendarFixtureDb::seeded().await?;
        let pool = connect_pool(fixture.path()).await?;
        let repo = CalendarRepository::new(&pool);
        let event = repo
            .get_event(SEED_EVENT_ID)
            .await?
            .ok_or("event not found")?;
        assert_eq!(event.summary.id, SEED_EVENT_ID);
        assert!(!event.attendees.is_empty());
        assert!(event.location.is_some());
        Ok(())
    }

    #[test]
    fn unix_to_core_data_conversion() {
        let core_data = unix_to_core_data_secs(1_736_942_400);
        assert!((core_data - 758_635_200.0).abs() < 1.0);
    }
}
