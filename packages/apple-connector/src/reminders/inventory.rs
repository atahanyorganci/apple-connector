use sqlx::SqlitePool;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReminderInventory {
    pub lists: u64,
    pub reminders: u64,
    pub completed: u64,
    pub with_due_date: u64,
    pub with_subtasks: u64,
    pub with_sections: u64,
    pub with_attachments: u64,
}

pub async fn load_inventory(pool: &SqlitePool) -> Result<ReminderInventory, sqlx::Error> {
    let lists: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ZREMCDBASELIST WHERE ZMARKEDFORDELETION = 0")
            .fetch_one(pool)
            .await?;

    let reminders: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ZREMCDREMINDER WHERE ZMARKEDFORDELETION = 0")
            .fetch_one(pool)
            .await?;

    let completed: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM ZREMCDREMINDER WHERE ZMARKEDFORDELETION = 0 AND ZCOMPLETED = 1",
    )
    .fetch_one(pool)
    .await?;

    let with_due_date: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM ZREMCDREMINDER WHERE ZMARKEDFORDELETION = 0 AND ZDUEDATE IS NOT NULL",
    )
    .fetch_one(pool)
    .await?;

    let with_subtasks: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM ZREMCDREMINDER WHERE ZMARKEDFORDELETION = 0 AND ZPARENTREMINDER IS NOT NULL",
    )
    .fetch_one(pool)
    .await?;

    let with_sections: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ZREMCDBASESECTION WHERE ZMARKEDFORDELETION = 0")
            .fetch_one(pool)
            .await?;

    let with_attachments: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ZREMCDSAVEDATTACHMENT WHERE ZMARKEDFORDELETION = 0")
            .fetch_one(pool)
            .await?;

    Ok(ReminderInventory {
        lists: lists.0 as u64,
        reminders: reminders.0 as u64,
        completed: completed.0 as u64,
        with_due_date: with_due_date.0 as u64,
        with_subtasks: with_subtasks.0 as u64,
        with_sections: with_sections.0 as u64,
        with_attachments: with_attachments.0 as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::load_inventory;
    use crate::{connect_pool, fixtures::RemindersFixtureDb};

    #[tokio::test]
    async fn inventory_counts_seeded_fixture() {
        let fixture = RemindersFixtureDb::seeded().await.expect("fixture");
        let pool = connect_pool(fixture.path()).await.expect("pool");
        let inventory = load_inventory(&pool).await.expect("inventory");
        assert!(inventory.lists >= 2);
        assert!(inventory.reminders >= 2);
    }
}
