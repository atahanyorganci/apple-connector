use std::collections::HashMap;

use sqlx::SqlitePool;
use thiserror::Error;
use tracing::debug;

use super::queries::fetch_entity_name_rows;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityIds {
    pub contact: i64,
    pub group: i64,
    pub container: i64,
}

#[derive(Debug, Error)]
#[error("missing Z_PRIMARYKEY entity: {name}")]
pub struct EntityIdError {
    pub name: &'static str,
}

const CONTACT_ENTITY_NAMES: &[&str] = &["ABCDContact", "ABCDGroup", "CNCDContainer"];

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

    for name in CONTACT_ENTITY_NAMES {
        require_entity(&map, name)?;
    }

    let ids = EntityIds {
        contact: require_entity(&map, "ABCDContact")?,
        group: require_entity(&map, "ABCDGroup")?,
        container: require_entity(&map, "CNCDContainer")?,
    };

    debug!(?ids, "resolved Contacts entity ids");
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::load_entity_ids;
    use crate::{db::connect_pool, fixtures::ContactsFixtureDb};

    #[tokio::test]
    async fn loads_entity_ids_from_seeded_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = ContactsFixtureDb::seeded().await?;
        let pool = connect_pool(fixture.path()).await?;
        let ids = load_entity_ids(&pool).await?;
        assert_eq!(ids.contact, 22);
        assert_eq!(ids.group, 19);
        assert_eq!(ids.container, 25);
        Ok(())
    }

    #[tokio::test]
    async fn missing_entity_metadata_fails_explicitly() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = ContactsFixtureDb::empty().await?;
        let pool = connect_pool(fixture.path()).await?;
        let error = load_entity_ids(&pool)
            .await
            .err()
            .ok_or("expected missing entity error")?;
        assert!(
            error
                .to_string()
                .contains("missing Z_PRIMARYKEY entity: ABCDContact"),
            "unexpected error: {error}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn loads_entity_ids_from_remapped_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = ContactsFixtureDb::seeded_with_remapped_entities().await?;
        let pool = connect_pool(fixture.path()).await?;
        let ids = load_entity_ids(&pool).await?;
        assert_eq!(ids.contact, 30);
        assert_eq!(ids.group, 28);
        assert_eq!(ids.container, 40);
        Ok(())
    }

    #[tokio::test]
    async fn unsupported_schema_fails_explicitly() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = ContactsFixtureDb::unsupported_schema().await?;
        let pool = connect_pool(fixture.path()).await?;
        let error = load_entity_ids(&pool)
            .await
            .err()
            .ok_or("expected error for unsupported schema")?;
        assert!(
            error.to_string().contains("no such table"),
            "unexpected error: {error}"
        );
        Ok(())
    }
}
