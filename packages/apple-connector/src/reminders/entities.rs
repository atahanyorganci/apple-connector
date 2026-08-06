use std::collections::HashMap;

use sqlx::SqlitePool;
use tracing::debug;

#[derive(Debug, Clone, Default)]
pub struct EntityIds {
    pub alarm: i64,
    pub alarm_date_trigger: i64,
    pub alarm_time_interval_trigger: i64,
    pub alarm_location_trigger: i64,
    pub recurrence_rule: i64,
    pub hashtag: i64,
    pub smart_list: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct EntityIdRow {
    ent: i64,
    name: String,
}

pub async fn load_entity_ids(pool: &SqlitePool) -> Result<EntityIds, sqlx::Error> {
    let rows = sqlx::query_as!(
        EntityIdRow,
        r#"
        SELECT Z_ENT AS "ent!", Z_NAME AS "name!"
        FROM Z_PRIMARYKEY
        WHERE Z_NAME IN (
            'REMCDAlarm', 'REMCDAlarmDateTrigger', 'REMCDAlarmTimeIntervalTrigger',
            'REMCDAlarmLocationTrigger', 'REMCDRecurrenceRule', 'REMCDHashtag', 'REMCDSmartList'
        )
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut map = HashMap::new();
    for row in rows {
        map.insert(row.name, row.ent);
    }

    let ids = EntityIds {
        alarm: *map.get("REMCDAlarm").unwrap_or(&0),
        alarm_date_trigger: *map.get("REMCDAlarmDateTrigger").unwrap_or(&0),
        alarm_time_interval_trigger: *map.get("REMCDAlarmTimeIntervalTrigger").unwrap_or(&0),
        alarm_location_trigger: *map.get("REMCDAlarmLocationTrigger").unwrap_or(&0),
        recurrence_rule: *map.get("REMCDRecurrenceRule").unwrap_or(&0),
        hashtag: *map.get("REMCDHashtag").unwrap_or(&0),
        smart_list: *map.get("REMCDSmartList").unwrap_or(&0),
    };

    debug!(?ids, "resolved Reminders entity ids");
    Ok(ids)
}

pub fn is_alarm_ent(ent: i64, ids: &EntityIds) -> bool {
    ent == ids.alarm
        || ent == ids.alarm_date_trigger
        || ent == ids.alarm_time_interval_trigger
        || ent == ids.alarm_location_trigger
}
