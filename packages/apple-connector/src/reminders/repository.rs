use std::collections::HashMap;

use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use super::{
    assembly::{
        alarm_from_object, attachment_from_row, build_section_map, list_from_row,
        list_summary_from_row, recurrence_from_object, reminder_from_row,
        reminder_summary_from_row,
    },
    entities::{load_entity_ids, EntityIds},
    model::{Reminder, ReminderAttachment, ReminderList, ReminderSummary, Section},
    row::{AttachmentRow, ListRow, ObjectRow, ReminderRow, SectionRow},
    search::{apply_filters, ReminderFilters},
    sections::section_from_row,
    sql::{
        ATTACHMENT_SELECT_CORE, LIST_FROM, LIST_SELECT_CORE, REMINDER_FROM_JOIN,
        REMINDER_SELECT_CORE, SECTION_SELECT_CORE,
    },
};
use crate::api::cursor::{
    encode, GlobalReminderCursor, ListCursor, ListReminderCursor, ReminderSearchCursor,
};

#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListLookupError {
    NotFound,
}

pub struct ReminderRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> ReminderRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_lists(
        &self,
        limit: u32,
        cursor: Option<ListCursor>,
    ) -> Result<Page<ReminderList>, sqlx::Error> {
        crate::db::run_timed_query(|| self.list_lists_inner(limit, cursor)).await
    }

    async fn list_lists_inner(
        &self,
        limit: u32,
        cursor: Option<ListCursor>,
    ) -> Result<Page<ReminderList>, sqlx::Error> {
        let entity_ids = load_entity_ids(self.pool).await?;
        let fetch_limit = i64::from(limit) + 1;

        let mut builder = QueryBuilder::<Sqlite>::new(LIST_SELECT_CORE);
        builder.push(LIST_FROM);
        if let Some(cursor) = cursor {
            builder.push(" AND l.Z_PK < ");
            builder.push_bind(cursor.row_id);
        }
        builder.push(" ORDER BY l.Z_PK DESC LIMIT ");
        builder.push_bind(fetch_limit);

        let rows: Vec<ListRow> = builder.build_query_as().fetch_all(self.pool).await?;
        let (rows, has_more) = split_page(rows, limit);
        let next_cursor = has_more
            .then(|| rows.last().map(|row| crate::api::cursor::encode(&ListCursor { row_id: row.row_id }).ok()))
            .flatten()
            .flatten();

        let items = rows
            .into_iter()
            .map(|row| list_summary_from_row(row, entity_ids.smart_list))
            .collect();

        Ok(Page {
            items,
            has_more,
            next_cursor,
        })
    }

    pub async fn get_list(&self, list_row_id: i64) -> Result<Option<ReminderList>, sqlx::Error> {
        let entity_ids = load_entity_ids(self.pool).await?;
        let mut builder = QueryBuilder::<Sqlite>::new(LIST_SELECT_CORE);
        builder.push(LIST_FROM);
        builder.push(" AND l.Z_PK = ");
        builder.push_bind(list_row_id);

        let row: Option<ListRow> = builder.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        let sections = self.fetch_sections_for_list(list_row_id).await?;
        Ok(Some(list_from_row(row, sections, entity_ids.smart_list)))
    }

    pub async fn get_list_by_key(
        &self,
        key: &crate::api::params::ReminderListKey,
    ) -> Result<Option<ReminderList>, sqlx::Error> {
        match key {
            crate::api::params::ReminderListKey::Row(row_id) => self.get_list(*row_id).await,
            crate::api::params::ReminderListKey::Id(id) => self.get_list_by_uuid(id).await,
        }
    }

    pub async fn get_list_by_uuid(&self, id: &str) -> Result<Option<ReminderList>, sqlx::Error> {
        let entity_ids = load_entity_ids(self.pool).await?;
        let mut builder = QueryBuilder::<Sqlite>::new(LIST_SELECT_CORE);
        builder.push(LIST_FROM);
        builder.push(" AND lower(substr(hex(l.ZIDENTIFIER), 1, 8) || '-' || substr(hex(l.ZIDENTIFIER), 9, 4) || '-' || substr(hex(l.ZIDENTIFIER), 13, 4) || '-' || substr(hex(l.ZIDENTIFIER), 17, 4) || '-' || substr(hex(l.ZIDENTIFIER), 21, 12)) = ");
        builder.push_bind(id.to_lowercase());

        let row: Option<ListRow> = builder.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        let sections = self.fetch_sections_for_list(row.row_id).await?;
        Ok(Some(list_from_row(row, sections, entity_ids.smart_list)))
    }

    pub async fn list_reminders(
        &self,
        filters: &ReminderFilters,
        limit: u32,
        cursor: Option<GlobalReminderCursor>,
        include_subtasks: bool,
        include_tags: bool,
        section_map: Option<&HashMap<i64, HashMap<String, String>>>,
    ) -> Result<Page<ReminderSummary>, sqlx::Error> {
        crate::db::run_timed_query(|| {
            self.list_reminders_inner(filters, limit, cursor, include_subtasks, include_tags, section_map)
        })
        .await
    }

    async fn list_reminders_inner(
        &self,
        filters: &ReminderFilters,
        limit: u32,
        cursor: Option<GlobalReminderCursor>,
        include_subtasks: bool,
        include_tags: bool,
        section_map: Option<&HashMap<i64, HashMap<String, String>>>,
    ) -> Result<Page<ReminderSummary>, sqlx::Error> {
        let entity_ids = load_entity_ids(self.pool).await?;
        let fetch_limit = i64::from(limit) + 1;
        let mut effective_filters = filters.clone();
        if !include_subtasks && effective_filters.top_level_only.is_none() {
            effective_filters.top_level_only = Some(true);
        }

        let mut builder = QueryBuilder::<Sqlite>::new(REMINDER_SELECT_CORE);
        builder.push(REMINDER_FROM_JOIN);
        apply_filters(&mut builder, &effective_filters);
        if let Some(cursor) = cursor {
            builder.push(
                " AND (r.ZLASTMODIFIEDDATE < ? OR (r.ZLASTMODIFIEDDATE = ? AND r.Z_PK < ?))",
            );
            builder.push_bind(cursor.modified_at);
            builder.push_bind(cursor.modified_at);
            builder.push_bind(cursor.row_id);
        }
        builder.push(" ORDER BY r.ZLASTMODIFIEDDATE DESC, r.Z_PK DESC LIMIT ");
        builder.push_bind(fetch_limit);

        let rows: Vec<ReminderRow> = builder.build_query_as().fetch_all(self.pool).await?;
        let (rows, has_more) = split_page(rows, limit);
        let use_search_cursor = effective_filters.is_active();
        let next_cursor = has_more
            .then(|| {
                rows.last().map(|row| {
                    let modified_at = row.last_modified_date.unwrap_or(0.0);
                    if use_search_cursor {
                        encode(&ReminderSearchCursor {
                            modified_at,
                            row_id: row.row_id,
                            filters: effective_filters.snapshot(),
                        })
                        .ok()
                    } else {
                        encode(&GlobalReminderCursor {
                            modified_at,
                            row_id: row.row_id,
                        })
                        .ok()
                    }
                })
            })
            .flatten()
            .flatten();

        let tags_by_reminder = if include_tags {
            self.fetch_tags_for_reminders(rows.iter().map(|row| row.row_id).collect(), &entity_ids)
                .await?
        } else {
            HashMap::new()
        };

        let items = rows
            .into_iter()
            .map(|row| {
                let section_id = section_map
                    .and_then(|maps| maps.get(&row.list_row_id))
                    .and_then(|map| map.get(&row.id.to_lowercase()))
                    .cloned();
                let tags = tags_by_reminder.get(&row.row_id).cloned().unwrap_or_default();
                reminder_summary_from_row(row, section_id, tags)
            })
            .collect();

        Ok(Page {
            items,
            has_more,
            next_cursor,
        })
    }

    pub async fn list_list_reminders(
        &self,
        list_row_id: i64,
        filters: &ReminderFilters,
        limit: u32,
        cursor: Option<ListReminderCursor>,
        include_subtasks: bool,
        include_tags: bool,
    ) -> Result<Result<Page<ReminderSummary>, ListLookupError>, sqlx::Error> {
        if self.get_list(list_row_id).await?.is_none() {
            return Ok(Err(ListLookupError::NotFound));
        }

        let mut scoped_filters = filters.clone();
        scoped_filters.list_id = Some(super::search::ListIdFilter::RowId(list_row_id));
        let global_cursor = cursor.map(|value| GlobalReminderCursor {
            modified_at: value.modified_at,
            row_id: value.row_id,
        });
        let section_map = self.build_section_maps_for_lists(&[list_row_id]).await?;
        let page = self
            .list_reminders_inner(
                &scoped_filters,
                limit,
                global_cursor,
                include_subtasks,
                include_tags,
                Some(&section_map),
            )
            .await?;

        let next_cursor = if scoped_filters.is_active() {
            page.next_cursor
        } else {
            page.next_cursor.and_then(|cursor| {
                crate::api::cursor::decode::<GlobalReminderCursor>(&cursor)
                    .ok()
                    .and_then(|decoded| {
                        encode(&ListReminderCursor {
                            modified_at: decoded.modified_at,
                            row_id: decoded.row_id,
                        })
                        .ok()
                    })
            })
        };

        Ok(Ok(Page {
            items: page.items,
            has_more: page.has_more,
            next_cursor,
        }))
    }

    pub async fn get_reminder(&self, id: &str) -> Result<Option<Reminder>, sqlx::Error> {
        let entity_ids = load_entity_ids(self.pool).await?;
        let mut builder = QueryBuilder::<Sqlite>::new(REMINDER_SELECT_CORE);
        builder.push(REMINDER_FROM_JOIN);
        builder.push(" AND lower(substr(hex(r.ZIDENTIFIER), 1, 8) || '-' || substr(hex(r.ZIDENTIFIER), 9, 4) || '-' || substr(hex(r.ZIDENTIFIER), 13, 4) || '-' || substr(hex(r.ZIDENTIFIER), 17, 4) || '-' || substr(hex(r.ZIDENTIFIER), 21, 12)) = ");
        builder.push_bind(id.to_lowercase());

        let row: Option<ReminderRow> = builder.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(
            self.hydrate_reminder(row, &entity_ids, true, true).await?,
        ))
    }

    pub async fn get_attachment_by_id(
        &self,
        id: &str,
    ) -> Result<Option<ReminderAttachment>, sqlx::Error> {
        let mut builder = QueryBuilder::<Sqlite>::new(ATTACHMENT_SELECT_CORE);
        builder.push(" FROM ZREMCDSAVEDATTACHMENT sa WHERE sa.ZMARKEDFORDELETION = 0 AND lower(substr(hex(sa.ZIDENTIFIER), 1, 8) || '-' || substr(hex(sa.ZIDENTIFIER), 9, 4) || '-' || substr(hex(sa.ZIDENTIFIER), 13, 4) || '-' || substr(hex(sa.ZIDENTIFIER), 17, 4) || '-' || substr(hex(sa.ZIDENTIFIER), 21, 12)) = ");
        builder.push_bind(id.to_lowercase());

        let row: Option<AttachmentRow> = builder.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(attachment_from_row))
    }

    pub async fn get_reminder_id_for_row(&self, row_id: i64) -> Result<Option<String>, sqlx::Error> {
        let id: Option<(String,)> = sqlx::query_as(
            "SELECT lower(substr(hex(r.ZIDENTIFIER), 1, 8) || '-' || substr(hex(r.ZIDENTIFIER), 9, 4) || '-' || substr(hex(r.ZIDENTIFIER), 13, 4) || '-' || substr(hex(r.ZIDENTIFIER), 17, 4) || '-' || substr(hex(r.ZIDENTIFIER), 21, 12)) \
             FROM ZREMCDREMINDER r WHERE r.Z_PK = ?1 AND r.ZMARKEDFORDELETION = 0",
        )
        .bind(row_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(id.map(|row| row.0))
    }

    pub async fn search_reminders(
        &self,
        filters: &ReminderFilters,
        limit: u32,
        cursor: Option<ReminderSearchCursor>,
        include_subtasks: bool,
        include_tags: bool,
    ) -> Result<Page<ReminderSummary>, sqlx::Error> {
        let global = cursor.map(|value| GlobalReminderCursor {
            modified_at: value.modified_at,
            row_id: value.row_id,
        });
        self.list_reminders(filters, limit, global, include_subtasks, include_tags, None)
            .await
    }

    async fn hydrate_reminder(
        &self,
        row: ReminderRow,
        entity_ids: &EntityIds,
        include_subtasks: bool,
        include_tags: bool,
    ) -> Result<Reminder, sqlx::Error> {
        let section_map = self.build_section_maps_for_lists(&[row.list_row_id]).await?;
        let section_id = section_map
            .get(&row.list_row_id)
            .and_then(|map| map.get(&row.id.to_lowercase()))
            .cloned();

        let subtasks = if include_subtasks {
            self.fetch_subtasks(row.row_id, entity_ids).await?
        } else {
            Vec::new()
        };

        let tags = if include_tags {
            self.fetch_tags_for_reminders(vec![row.row_id], entity_ids)
                .await?
                .remove(&row.row_id)
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let alarms = self.fetch_alarms(row.row_id, entity_ids).await?;
        let recurrence = self.fetch_recurrence(row.row_id, entity_ids).await?;
        let attachments = self.fetch_attachments(row.row_id).await?;

        Ok(reminder_from_row(
            row,
            section_id,
            subtasks,
            tags,
            alarms,
            recurrence,
            attachments,
        ))
    }

    async fn fetch_sections_for_list(&self, list_row_id: i64) -> Result<Vec<Section>, sqlx::Error> {
        let mut builder = QueryBuilder::<Sqlite>::new(SECTION_SELECT_CORE);
        builder.push(" FROM ZREMCDBASESECTION s WHERE s.ZMARKEDFORDELETION = 0 AND s.ZLIST = ");
        builder.push_bind(list_row_id);
        builder.push(" ORDER BY s.Z_PK ASC");

        let rows: Vec<SectionRow> = builder.build_query_as().fetch_all(self.pool).await?;
        Ok(rows.into_iter().map(section_from_row).collect())
    }

    async fn build_section_maps_for_lists(
        &self,
        list_row_ids: &[i64],
    ) -> Result<HashMap<i64, HashMap<String, String>>, sqlx::Error> {
        let mut maps = HashMap::new();
        for list_row_id in list_row_ids {
            let data: Option<(Option<Vec<u8>>,) > = sqlx::query_as(
                "SELECT ZMEMBERSHIPSOFREMINDERSINSECTIONSASDATA FROM ZREMCDBASELIST WHERE Z_PK = ?1 AND ZMARKEDFORDELETION = 0",
            )
            .bind(list_row_id)
            .fetch_optional(self.pool)
            .await?;
            let membership = data.and_then(|row| row.0);
            maps.insert(*list_row_id, build_section_map(membership.as_deref()));
        }
        Ok(maps)
    }

    async fn fetch_subtasks(
        &self,
        parent_row_id: i64,
        entity_ids: &EntityIds,
    ) -> Result<Vec<ReminderSummary>, sqlx::Error> {
        let mut builder = QueryBuilder::<Sqlite>::new(REMINDER_SELECT_CORE);
        builder.push(REMINDER_FROM_JOIN);
        builder.push(" AND r.ZPARENTREMINDER = ");
        builder.push_bind(parent_row_id);
        builder.push(" ORDER BY r.ZICSDISPLAYORDER ASC, r.Z_PK ASC");

        let rows: Vec<ReminderRow> = builder.build_query_as().fetch_all(self.pool).await?;
        let tags = self
            .fetch_tags_for_reminders(rows.iter().map(|row| row.row_id).collect(), entity_ids)
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let tags = tags.get(&row.row_id).cloned().unwrap_or_default();
                reminder_summary_from_row(row, None, tags)
            })
            .collect())
    }

    async fn fetch_tags_for_reminders(
        &self,
        reminder_row_ids: Vec<i64>,
        entity_ids: &EntityIds,
    ) -> Result<HashMap<i64, Vec<String>>, sqlx::Error> {
        if reminder_row_ids.is_empty() || entity_ids.hashtag == 0 {
            return Ok(HashMap::new());
        }

        let mut builder = QueryBuilder::<Sqlite>::new(
            "SELECT o.ZREMINDER AS reminder_row_id, label.ZNAME AS tag_name \
             FROM ZREMCDOBJECT o \
             JOIN ZREMCDHASHTAGLABEL label ON o.ZHASHTAGLABEL = label.Z_PK \
             WHERE o.ZMARKEDFORDELETION = 0 AND o.Z_ENT = ",
        );
        builder.push_bind(entity_ids.hashtag);
        builder.push(" AND o.ZREMINDER IN (");
        {
            let mut separated = builder.separated(", ");
            for id in &reminder_row_ids {
                separated.push_bind(id);
            }
        }
        builder.push(")");

        let rows: Vec<(i64, String)> = builder.build_query_as().fetch_all(self.pool).await?;
        let mut map: HashMap<i64, Vec<String>> = HashMap::new();
        for (reminder_row_id, tag_name) in rows {
            map.entry(reminder_row_id).or_default().push(tag_name);
        }
        Ok(map)
    }

    async fn fetch_alarms(
        &self,
        reminder_row_id: i64,
        entity_ids: &EntityIds,
    ) -> Result<Vec<super::model::Alarm>, sqlx::Error> {
        let rows: Vec<ObjectRow> = sqlx::query_as(
            "SELECT o.Z_PK AS row_id, o.Z_ENT AS ent, o.ZREMINDER AS reminder_row_id, o.ZTYPE AS object_type, \
             o.ZTITLE AS title, o.ZLATITUDE AS latitude, o.ZLONGITUDE AS longitude, o.ZRADIUS AS radius, \
             o.ZTIMEINTERVAL AS time_interval, o.ZDATECOMPONENTSDATA AS date_components_data, \
             o.Z16_TRIGGER AS trigger_row_id, o.ZFREQUENCY AS frequency, o.ZINTERVAL AS recurrence_interval, \
             o.ZOCCURRENCECOUNT AS occurrence_count, o.ZHASHTAGLABEL AS hashtag_label_row_id, NULL AS tag_name \
             FROM ZREMCDOBJECT o \
             WHERE o.ZMARKEDFORDELETION = 0 AND o.ZREMINDER = ?1",
        )
        .bind(reminder_row_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .iter()
            .filter_map(|row| alarm_from_object(row, entity_ids))
            .collect())
    }

    async fn fetch_recurrence(
        &self,
        reminder_row_id: i64,
        entity_ids: &EntityIds,
    ) -> Result<Option<super::model::RecurrenceRule>, sqlx::Error> {
        let rows: Vec<ObjectRow> = sqlx::query_as(
            "SELECT o.Z_PK AS row_id, o.Z_ENT AS ent, o.ZREMINDER AS reminder_row_id, o.ZTYPE AS object_type, \
             o.ZTITLE AS title, o.ZLATITUDE AS latitude, o.ZLONGITUDE AS longitude, o.ZRADIUS AS radius, \
             o.ZTIMEINTERVAL AS time_interval, o.ZDATECOMPONENTSDATA AS date_components_data, \
             o.Z16_TRIGGER AS trigger_row_id, o.ZFREQUENCY AS frequency, o.ZINTERVAL AS recurrence_interval, \
             o.ZOCCURRENCECOUNT AS occurrence_count, o.ZHASHTAGLABEL AS hashtag_label_row_id, NULL AS tag_name \
             FROM ZREMCDOBJECT o \
             WHERE o.ZMARKEDFORDELETION = 0 AND o.ZREMINDER = ?1 AND o.Z_ENT = ?2",
        )
        .bind(reminder_row_id)
        .bind(entity_ids.recurrence_rule)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .first()
            .and_then(|row| recurrence_from_object(row, entity_ids.recurrence_rule)))
    }

    async fn fetch_attachments(
        &self,
        reminder_row_id: i64,
    ) -> Result<Vec<ReminderAttachment>, sqlx::Error> {
        let mut builder = QueryBuilder::<Sqlite>::new(ATTACHMENT_SELECT_CORE);
        builder.push(" FROM ZREMCDSAVEDATTACHMENT sa WHERE sa.ZMARKEDFORDELETION = 0 AND sa.ZREMINDER = ");
        builder.push_bind(reminder_row_id);

        let rows: Vec<AttachmentRow> = builder.build_query_as().fetch_all(self.pool).await?;
        Ok(rows.into_iter().map(attachment_from_row).collect())
    }
}

fn split_page<T>(mut rows: Vec<T>, limit: u32) -> (Vec<T>, bool) {
    let has_more = rows.len() > limit as usize;
    if has_more {
        rows.truncate(limit as usize);
    }
    (rows, has_more)
}

#[cfg(test)]
mod tests {
    use super::ReminderRepository;
    use crate::{connect_pool, fixtures::RemindersFixtureDb};

    #[tokio::test]
    async fn repository_reads_seeded_lists_and_reminders() {
        let fixture = RemindersFixtureDb::seeded().await.expect("fixture");
        let pool = connect_pool(fixture.path()).await.expect("pool");
        let repo = ReminderRepository::new(&pool);

        let lists = repo.list_lists(10, None).await.expect("lists");
        assert!(!lists.items.is_empty());

        let reminders = repo
            .list_reminders(&Default::default(), 10, None, false, false, None)
            .await
            .expect("reminders");
        assert!(!reminders.items.is_empty());

        let detail = repo
            .get_reminder("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb")
            .await
            .expect("detail")
            .expect("reminder");
        assert_eq!(detail.title, "Fixture Reminder");
        assert!(!detail.subtasks.is_empty());
    }
}
