use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use super::{
    assembly::{
        account_from_row, calendar_detail_from_row, calendar_summary_from_row,
        event_detail_from_row, event_summary_from_row,
    },
    model::{
        CalendarAccount, CalendarDetail, CalendarSummary, EventAttachment, EventDetail,
        EventSummary,
    },
    row::{
        AlarmRow, AttachmentRow, CalendarRow, EventRow, ExceptionDateRow, LocationRow,
        ParticipantRow, RecurrenceRow, StoreRow, core_data_secs_from_timestamp,
    },
    search::{
        EventFilters, apply_direct_date_range, apply_event_filters, apply_occurrence_date_range,
    },
    sql::{
        ALARM_SELECT, ATTACHMENT_SELECT, CALENDAR_SELECT, EVENT_SELECT, EXCEPTION_DATE_SELECT,
        LOCATION_SELECT, OCCURRENCE_EVENT_SELECT, PARTICIPANT_SELECT, RECURRENCE_SELECT,
        STORE_SELECT,
    },
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

pub struct CalendarRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> CalendarRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_accounts(&self) -> Result<Vec<CalendarAccount>, sqlx::Error> {
        crate::db::run_timed_query(|| async {
            let mut builder = QueryBuilder::<Sqlite>::new(STORE_SELECT);
            builder.push(" ORDER BY s.display_order, s.ROWID");
            let rows: Vec<StoreRow> = builder.build_query_as().fetch_all(self.pool).await?;
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
        let mut builder = QueryBuilder::<Sqlite>::new(CALENDAR_SELECT);
        builder.push(" WHERE 1=1");
        if let Some(cursor) = cursor {
            builder.push(" AND c.ROWID < ");
            builder.push_bind(cursor.row_id);
        }
        builder.push(" ORDER BY c.ROWID DESC LIMIT ");
        builder.push_bind(fetch_limit);

        let rows: Vec<CalendarRow> = builder.build_query_as().fetch_all(self.pool).await?;
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
            let mut builder = QueryBuilder::<Sqlite>::new(CALENDAR_SELECT);
            builder.push(" WHERE lower(c.UUID) = lower(");
            builder.push_bind(calendar_id.to_owned());
            builder.push(")");
            let row: Option<CalendarRow> =
                builder.build_query_as().fetch_optional(self.pool).await?;
            Ok(row.map(calendar_detail_from_row))
        })
        .await
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
        let mut builder = QueryBuilder::<Sqlite>::new(EVENT_SELECT);
        builder.push(" JOIN Store s ON s.ROWID = c.store_id WHERE 1=1");
        apply_event_filters(&mut builder, filters, "ci");
        apply_direct_date_range(
            &mut builder,
            filters.start_after,
            filters.start_before,
            "ci",
        );
        if let Some(cursor) = cursor {
            builder.push(" AND (ci.last_modified < ");
            builder.push_bind(cursor.modified_at);
            builder.push(" OR (ci.last_modified = ");
            builder.push_bind(cursor.modified_at);
            builder.push(" AND ci.ROWID < ");
            builder.push_bind(cursor.row_id);
            builder.push("))");
        }
        builder.push(" ORDER BY ci.last_modified DESC, ci.ROWID DESC LIMIT ");
        builder.push_bind(fetch_limit);

        let rows: Vec<EventRow> = builder.build_query_as().fetch_all(self.pool).await?;
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
        let mut builder = QueryBuilder::<Sqlite>::new(OCCURRENCE_EVENT_SELECT);
        builder.push(" JOIN Store s ON s.ROWID = c.store_id WHERE 1=1");
        apply_event_filters(&mut builder, filters, "ci");
        apply_occurrence_date_range(&mut builder, filters.start_after, filters.start_before);
        if let Some(cursor) = cursor {
            builder.push(" AND (oc.occurrence_start_date < ");
            builder.push_bind(cursor.modified_at);
            builder.push(" OR (oc.occurrence_start_date = ");
            builder.push_bind(cursor.modified_at);
            builder.push(" AND ci.ROWID < ");
            builder.push_bind(cursor.row_id);
            builder.push("))");
        }
        builder.push(" ORDER BY oc.occurrence_start_date DESC, ci.ROWID DESC LIMIT ");
        builder.push_bind(fetch_limit);

        let rows: Vec<EventRow> = builder.build_query_as().fetch_all(self.pool).await?;
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
        let mut builder = QueryBuilder::<Sqlite>::new(EVENT_SELECT);
        builder.push(" WHERE lower(ci.UUID) = lower(");
        builder.push_bind(event_id.to_owned());
        builder.push(")");
        let row: Option<EventRow> = builder.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(self.hydrate_event(row).await?))
    }

    async fn hydrate_event(&self, row: EventRow) -> Result<EventDetail, sqlx::Error> {
        let event_row_id = row.row_id;
        let location = if let Some(location_id) = row.location_id.filter(|id| *id > 0) {
            let mut builder = QueryBuilder::<Sqlite>::new(LOCATION_SELECT);
            builder.push(" WHERE ROWID = ");
            builder.push_bind(location_id);
            builder
                .build_query_as::<LocationRow>()
                .fetch_optional(self.pool)
                .await?
        } else {
            None
        };

        let mut participant_builder = QueryBuilder::<Sqlite>::new(PARTICIPANT_SELECT);
        participant_builder.push(" WHERE owner_id = ");
        participant_builder.push_bind(event_row_id);
        let participants: Vec<ParticipantRow> = participant_builder
            .build_query_as()
            .fetch_all(self.pool)
            .await?;
        let organizer = row
            .organizer_id
            .and_then(|org_id| participants.iter().find(|p| p.row_id == org_id).cloned());
        let attendees: Vec<ParticipantRow> = participants
            .into_iter()
            .filter(|p| Some(p.row_id) != row.organizer_id)
            .collect();

        let mut recurrence_builder = QueryBuilder::<Sqlite>::new(RECURRENCE_SELECT);
        recurrence_builder.push(" WHERE owner_id = ");
        recurrence_builder.push_bind(event_row_id);
        let recurrence: Option<RecurrenceRow> = recurrence_builder
            .build_query_as()
            .fetch_optional(self.pool)
            .await?;

        let mut exception_builder = QueryBuilder::<Sqlite>::new(EXCEPTION_DATE_SELECT);
        exception_builder.push(" WHERE owner_id = ");
        exception_builder.push_bind(event_row_id);
        let exception_rows: Vec<ExceptionDateRow> = exception_builder
            .build_query_as()
            .fetch_all(self.pool)
            .await?;

        let mut alarm_builder = QueryBuilder::<Sqlite>::new(ALARM_SELECT);
        alarm_builder.push(" WHERE calendaritem_owner_id = ");
        alarm_builder.push_bind(event_row_id);
        let alarms: Vec<AlarmRow> = alarm_builder.build_query_as().fetch_all(self.pool).await?;

        let mut attachment_builder = QueryBuilder::<Sqlite>::new(ATTACHMENT_SELECT);
        attachment_builder.push(" WHERE a.owner_id = ");
        attachment_builder.push_bind(event_row_id);
        let attachments: Vec<AttachmentRow> = attachment_builder
            .build_query_as()
            .fetch_all(self.pool)
            .await?;

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
            let mut builder = QueryBuilder::<Sqlite>::new(ATTACHMENT_SELECT);
            builder.push(" JOIN CalendarItem ci ON ci.ROWID = a.owner_id");
            builder.push(" WHERE lower(ci.UUID) = lower(");
            builder.push_bind(event_id.to_owned());
            builder.push(") AND lower(af.UUID) = lower(");
            builder.push_bind(attachment_id.to_owned());
            builder.push(")");
            let row: Option<AttachmentRow> =
                builder.build_query_as().fetch_optional(self.pool).await?;
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
    core_data_secs_from_timestamp(chrono::Utc.timestamp_opt(unix, 0).unwrap())
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
    async fn lists_seeded_events() {
        let fixture = CalendarFixtureDb::seeded().await.expect("fixture");
        let pool = connect_pool(fixture.path()).await.expect("pool");
        let repo = CalendarRepository::new(&pool);
        let page = repo
            .list_events(&Default::default(), 10, None)
            .await
            .expect("list");
        assert!(!page.items.is_empty());
    }

    #[tokio::test]
    async fn calendar_event_cursor_round_trips_through_pagination() {
        let fixture = CalendarFixtureDb::seeded().await.expect("fixture");
        let pool = connect_pool(fixture.path()).await.expect("pool");
        let repo = CalendarRepository::new(&pool);
        let calendars = repo.list_calendars(1, None).await.expect("calendars");
        let calendar_id = calendars.items[0].id.clone();

        let first = repo
            .list_calendar_events(&calendar_id, &Default::default(), 1, None)
            .await
            .expect("first page");
        assert!(first.has_more);
        let cursor = first.next_cursor.expect("next cursor");

        let second = repo
            .list_calendar_events(
                &calendar_id,
                &Default::default(),
                1,
                Some(decode(&cursor).expect("decode calendar cursor")),
            )
            .await
            .expect("second page");
        assert_ne!(first.items[0].id, second.items[0].id);
    }

    #[tokio::test]
    async fn gets_event_detail() {
        let fixture = CalendarFixtureDb::seeded().await.expect("fixture");
        let pool = connect_pool(fixture.path()).await.expect("pool");
        let repo = CalendarRepository::new(&pool);
        let event = repo
            .get_event(SEED_EVENT_ID)
            .await
            .expect("get")
            .expect("event");
        assert_eq!(event.summary.id, SEED_EVENT_ID);
        assert!(!event.attendees.is_empty());
        assert!(event.location.is_some());
    }

    #[test]
    fn unix_to_core_data_conversion() {
        let core_data = unix_to_core_data_secs(1_736_942_400);
        assert!((core_data - 758_635_200.0).abs() < 1.0);
    }
}
