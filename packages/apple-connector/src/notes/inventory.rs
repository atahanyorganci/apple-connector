use sqlx::SqlitePool;

use super::{
    entities::load_entity_ids,
    queries::{
        count_attachments, count_deleted_notes, count_folders, count_locked_notes, count_notes,
        count_notes_with_checklist, count_pinned_notes,
    },
};

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

    let folders = count_folders(pool, entity_ids.folder).await?;
    let notes = count_notes(pool, entity_ids.note).await?;
    let pinned = count_pinned_notes(pool, entity_ids.note).await?;
    let locked = count_locked_notes(pool, entity_ids.note).await?;
    let with_checklist = count_notes_with_checklist(pool, entity_ids.note).await?;
    let with_attachments = count_attachments(pool, entity_ids.attachment).await?;
    let deleted_notes = count_deleted_notes(pool, entity_ids.note).await?;

    Ok(NoteInventory {
        folders: folders as u64,
        notes: notes as u64,
        pinned: pinned as u64,
        locked: locked as u64,
        with_checklist: with_checklist as u64,
        with_attachments: with_attachments as u64,
        deleted_notes: deleted_notes as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::load_inventory;
    use crate::{connect_pool, fixtures::NotesFixtureDb};

    #[tokio::test]
    async fn seeded_fixture_inventory_counts_notes() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = NotesFixtureDb::seeded().await?;
        let pool = connect_pool(fixture.path()).await?;
        let inventory = load_inventory(&pool).await?;
        assert!(inventory.folders >= 2);
        assert!(inventory.notes >= 3);
        Ok(())
    }
}
