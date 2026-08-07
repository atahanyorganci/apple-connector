use std::collections::HashMap;

use sqlx::SqlitePool;
use thiserror::Error;
use tracing::debug;

use super::queries::fetch_entity_name_rows;

#[derive(Debug, Clone)]
pub struct EntityIds {
    pub note: i64,
    pub folder: i64,
    pub attachment: i64,
    #[allow(dead_code)]
    pub account: i64,
    #[allow(dead_code)]
    pub hashtag: i64,
}

#[derive(Debug, Error)]
#[error("missing Z_PRIMARYKEY entity: {name}")]
pub struct EntityIdError {
    pub name: &'static str,
}

const NOTE_ENTITY_NAMES: &[&str] = &[
    "ICNote",
    "ICFolder",
    "ICAttachment",
    "ICAccount",
    "ICHashtag",
];

fn require_entity(map: &HashMap<String, i64>, name: &'static str) -> Result<i64, sqlx::Error> {
    map.get(name)
        .copied()
        .ok_or_else(|| sqlx::Error::Decode(Box::new(EntityIdError { name })))
}

pub async fn load_entity_ids(pool: &SqlitePool) -> Result<EntityIds, sqlx::Error> {
    let rows = fetch_entity_name_rows(pool).await?;

    let mut map = HashMap::new();
    for row in rows {
        map.insert(row.name, row.ent);
    }

    for name in NOTE_ENTITY_NAMES {
        require_entity(&map, name)?;
    }

    let ids = EntityIds {
        note: require_entity(&map, "ICNote")?,
        folder: require_entity(&map, "ICFolder")?,
        attachment: require_entity(&map, "ICAttachment")?,
        account: require_entity(&map, "ICAccount")?,
        hashtag: require_entity(&map, "ICHashtag")?,
    };

    debug!(?ids, "resolved Notes entity ids");
    Ok(ids)
}
