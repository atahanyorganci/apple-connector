use sqlx::SqlitePool;

use super::entities::load_entity_ids;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NoteInventory {
    pub folders: u64,
    pub notes: u64,
    pub pinned: u64,
    pub locked: u64,
    pub with_checklist: u64,
    pub with_attachments: u64,
    pub deleted_notes: u64,
}

pub async fn load_inventory(pool: &SqlitePool) -> Result<NoteInventory, sqlx::Error> {
    let entity_ids = load_entity_ids(pool).await?;

    let folders: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM ZICCLOUDSYNCINGOBJECT \
         WHERE Z_ENT = ?1 AND ZMARKEDFORDELETION = 0 AND ZFOLDERTYPE != 1",
    )
    .bind(entity_ids.folder)
    .fetch_one(pool)
    .await?;

    let notes: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM ZICCLOUDSYNCINGOBJECT n \
         LEFT JOIN ZICCLOUDSYNCINGOBJECT f ON n.ZFOLDER = f.Z_PK \
         WHERE n.Z_ENT = ?1 AND n.ZMARKEDFORDELETION = 0 \
         AND (f.Z_PK IS NULL OR (f.ZMARKEDFORDELETION = 0 AND f.ZFOLDERTYPE != 1))",
    )
    .bind(entity_ids.note)
    .fetch_one(pool)
    .await?;

    let pinned: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM ZICCLOUDSYNCINGOBJECT \
         WHERE Z_ENT = ?1 AND ZMARKEDFORDELETION = 0 AND ZISPINNED = 1",
    )
    .bind(entity_ids.note)
    .fetch_one(pool)
    .await?;

    let locked: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM ZICCLOUDSYNCINGOBJECT \
         WHERE Z_ENT = ?1 AND ZMARKEDFORDELETION = 0 AND ZISPASSWORDPROTECTED = 1",
    )
    .bind(entity_ids.note)
    .fetch_one(pool)
    .await?;

    let with_checklist: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM ZICCLOUDSYNCINGOBJECT \
         WHERE Z_ENT = ?1 AND ZMARKEDFORDELETION = 0 AND ZHASCHECKLIST = 1",
    )
    .bind(entity_ids.note)
    .fetch_one(pool)
    .await?;

    let with_attachments: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM ZICCLOUDSYNCINGOBJECT \
         WHERE Z_ENT = ?1 AND ZMARKEDFORDELETION = 0",
    )
    .bind(entity_ids.attachment)
    .fetch_one(pool)
    .await?;

    let deleted_notes: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM ZICCLOUDSYNCINGOBJECT n \
         LEFT JOIN ZICCLOUDSYNCINGOBJECT f ON n.ZFOLDER = f.Z_PK \
         WHERE n.Z_ENT = ?1 AND (n.ZMARKEDFORDELETION = 1 OR f.ZFOLDERTYPE = 1)",
    )
    .bind(entity_ids.note)
    .fetch_one(pool)
    .await?;

    Ok(NoteInventory {
        folders: folders.0 as u64,
        notes: notes.0 as u64,
        pinned: pinned.0 as u64,
        locked: locked.0 as u64,
        with_checklist: with_checklist.0 as u64,
        with_attachments: with_attachments.0 as u64,
        deleted_notes: deleted_notes.0 as u64,
    })
}

#[cfg(test)]
mod tests {
    use crate::fixtures::NotesFixtureDb;
    use crate::connect_pool;

    use super::load_inventory;

    #[tokio::test]
    async fn seeded_fixture_inventory_counts_notes() {
        let fixture = NotesFixtureDb::seeded().await.expect("fixture");
        let pool = connect_pool(fixture.path()).await.expect("pool");
        let inventory = load_inventory(&pool).await.expect("inventory");
        assert!(inventory.folders >= 2);
        assert!(inventory.notes >= 3);
    }
}
