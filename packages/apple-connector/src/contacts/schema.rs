//! Runtime discovery of the Core Data `parentGroups` many-to-many join table.
//!
//! AddressBook names this join table after the numeric entity ids assigned by
//! Core Data (e.g. `Z_22PARENTGROUPS`), which differ across macOS versions and
//! stores. We resolve the concrete names once per pool and verify them against
//! `sqlite_master` / `PRAGMA table_info` before using them in dynamic SQL.

use std::collections::HashSet;

use sqlx::SqlitePool;
use thiserror::Error;
use tracing::debug;

use super::entities::{EntityIds, load_entity_ids};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentGroupsJoin {
    pub table: String,
    pub contact_col: String,
    pub group_col: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactsSchema {
    pub entity_ids: EntityIds,
    pub parent_groups: ParentGroupsJoin,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParentGroupsSchemaError {
    #[error("invalid identifier derived for Contacts parentGroups join: {name}")]
    InvalidIdentifier { name: String },
    #[error("missing Contacts parentGroups join table: {table}")]
    MissingTable { table: String },
    #[error("missing column `{column}` on Contacts parentGroups join table `{table}`")]
    MissingColumn { table: String, column: String },
}

/// Only allow the characters Core Data uses for generated table/column names
/// before interpolating them into dynamic SQL.
fn is_sql_identifier(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

fn validated_identifier(name: String) -> Result<String, sqlx::Error> {
    if is_sql_identifier(&name) {
        Ok(name)
    } else {
        Err(sqlx::Error::Decode(Box::new(
            ParentGroupsSchemaError::InvalidIdentifier { name },
        )))
    }
}

pub async fn load_contacts_schema(pool: &SqlitePool) -> Result<ContactsSchema, sqlx::Error> {
    let entity_ids = load_entity_ids(pool).await?;
    let parent_groups = discover_parent_groups_join(pool, &entity_ids).await?;
    Ok(ContactsSchema {
        entity_ids,
        parent_groups,
    })
}

pub async fn discover_parent_groups_join(
    pool: &SqlitePool,
    entity_ids: &EntityIds,
) -> Result<ParentGroupsJoin, sqlx::Error> {
    let table = validated_identifier(format!("Z_{}PARENTGROUPS", entity_ids.contact))?;
    let contact_col = validated_identifier(format!("Z_{}CONTACTS", entity_ids.contact))?;
    let group_col = validated_identifier(format!("Z_{}PARENTGROUPS1", entity_ids.group))?;

    let table_count = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!: i64" FROM sqlite_master WHERE type = 'table' AND name = ?1"#,
        table,
    )
    .fetch_one(pool)
    .await?;
    if table_count == 0 {
        return Err(sqlx::Error::Decode(Box::new(
            ParentGroupsSchemaError::MissingTable { table },
        )));
    }

    // `PRAGMA table_info` cannot take a bound parameter for the table name, and
    // SQLite cannot resolve `pragma_table_info(?)`'s columns without a literal
    // argument either, so the (already-validated) identifier is interpolated here.
    let pragma_sql = format!("SELECT name FROM pragma_table_info('{table}')");
    let column_names: HashSet<String> =
        sqlx::query_as::<_, (String,)>(sqlx::AssertSqlSafe(pragma_sql))
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|(name,)| name)
            .collect();

    for column in [&contact_col, &group_col] {
        if !column_names.contains(column) {
            return Err(sqlx::Error::Decode(Box::new(
                ParentGroupsSchemaError::MissingColumn {
                    table: table.clone(),
                    column: column.clone(),
                },
            )));
        }
    }

    debug!(
        table = %table,
        contact_col = %contact_col,
        group_col = %group_col,
        "resolved Contacts parentGroups join"
    );

    Ok(ParentGroupsJoin {
        table,
        contact_col,
        group_col,
    })
}

#[cfg(test)]
mod tests {
    use super::load_contacts_schema;
    use crate::{db::connect_pool, fixtures::ContactsFixtureDb};

    #[tokio::test]
    async fn discovers_parent_groups_join_from_seeded_fixture()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = ContactsFixtureDb::seeded().await?;
        let pool = connect_pool(fixture.path()).await?;

        let schema = load_contacts_schema(&pool).await?;

        assert_eq!(schema.entity_ids.contact, 22);
        assert_eq!(schema.entity_ids.group, 19);
        assert_eq!(schema.parent_groups.table, "Z_22PARENTGROUPS");
        assert_eq!(schema.parent_groups.contact_col, "Z_22CONTACTS");
        assert_eq!(schema.parent_groups.group_col, "Z_19PARENTGROUPS1");
        Ok(())
    }

    #[tokio::test]
    async fn missing_parent_groups_table_fails_explicitly() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = ContactsFixtureDb::seeded_without_parent_groups_join().await?;
        let pool = connect_pool(fixture.path()).await?;

        let error = load_contacts_schema(&pool)
            .await
            .err()
            .ok_or("expected missing join table error")?;
        assert!(
            error
                .to_string()
                .contains("missing Contacts parentGroups join table: Z_22PARENTGROUPS"),
            "unexpected error: {error}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn resolves_remapped_parent_groups_join() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = ContactsFixtureDb::seeded_with_remapped_entities().await?;
        let pool = connect_pool(fixture.path()).await?;

        let schema = load_contacts_schema(&pool).await?;

        assert_eq!(schema.entity_ids.contact, 30);
        assert_eq!(schema.entity_ids.group, 28);
        assert_eq!(schema.entity_ids.container, 40);
        assert_eq!(schema.parent_groups.table, "Z_30PARENTGROUPS");
        assert_eq!(schema.parent_groups.contact_col, "Z_30CONTACTS");
        assert_eq!(schema.parent_groups.group_col, "Z_28PARENTGROUPS1");
        Ok(())
    }
}
