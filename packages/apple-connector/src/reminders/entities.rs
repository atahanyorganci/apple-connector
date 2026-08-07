use std::collections::HashMap;

use sqlx::SqlitePool;
use thiserror::Error;
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

#[derive(Debug, Error)]
#[error("missing Z_PRIMARYKEY entity: {name}")]
pub struct EntityIdError {
    pub name: &'static str,
}

#[derive(Debug, sqlx::FromRow)]
struct EntityIdRow {
    ent: i64,
    name: String,
}

fn require_entity(map: &HashMap<String, i64>, name: &'static str) -> Result<i64, sqlx::Error> {
    map.get(name)
        .copied()
        .ok_or_else(|| sqlx::Error::Decode(Box::new(EntityIdError { name })))
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
        alarm: require_entity(&map, "REMCDAlarm")?,
        alarm_date_trigger: require_entity(&map, "REMCDAlarmDateTrigger")?,
        alarm_time_interval_trigger: require_entity(&map, "REMCDAlarmTimeIntervalTrigger")?,
        alarm_location_trigger: require_entity(&map, "REMCDAlarmLocationTrigger")?,
        recurrence_rule: require_entity(&map, "REMCDRecurrenceRule")?,
        hashtag: require_entity(&map, "REMCDHashtag")?,
        smart_list: require_entity(&map, "REMCDSmartList")?,
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
