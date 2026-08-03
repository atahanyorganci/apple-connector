use std::collections::HashMap;

use sqlx::SqlitePool;
use tracing::debug;

use super::queries::fetch_entity_name_rows;

#[derive(Debug, Clone, Default)]
pub struct EntityIds {
    pub note: i64,
    pub folder: i64,
    pub attachment: i64,
    #[allow(dead_code)]
    pub account: i64,
    #[allow(dead_code)]
    pub hashtag: i64,
}

pub async fn load_entity_ids(pool: &SqlitePool) -> Result<EntityIds, sqlx::Error> {
    let rows = fetch_entity_name_rows(pool).await?;

    let mut map = HashMap::new();
    for row in rows {
        map.insert(row.name, row.ent);
    }

    let ids = EntityIds {
        note: *map.get("ICNote").unwrap_or(&12),
        folder: *map.get("ICFolder").unwrap_or(&15),
        attachment: *map.get("ICAttachment").unwrap_or(&5),
        account: *map.get("ICAccount").unwrap_or(&14),
        hashtag: *map.get("ICHashtag").unwrap_or(&8),
    };

    debug!(?ids, "resolved Notes entity ids");
    Ok(ids)
}
