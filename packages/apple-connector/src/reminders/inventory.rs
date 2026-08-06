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
    let lists = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!: i64" FROM ZREMCDBASELIST WHERE ZMARKEDFORDELETION = 0"#
    )
    .fetch_one(pool)
    .await?;

    let reminders = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!: i64" FROM ZREMCDREMINDER WHERE ZMARKEDFORDELETION = 0"#
    )
    .fetch_one(pool)
    .await?;

    let completed = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!: i64" FROM ZREMCDREMINDER WHERE ZMARKEDFORDELETION = 0 AND ZCOMPLETED = 1"#
    )
    .fetch_one(pool)
    .await?;

    let with_due_date = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!: i64" FROM ZREMCDREMINDER WHERE ZMARKEDFORDELETION = 0 AND ZDUEDATE IS NOT NULL"#
    )
    .fetch_one(pool)
    .await?;

    let with_subtasks = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!: i64" FROM ZREMCDREMINDER WHERE ZMARKEDFORDELETION = 0 AND ZPARENTREMINDER IS NOT NULL"#
    )
    .fetch_one(pool)
    .await?;

    let with_sections = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!: i64" FROM ZREMCDBASESECTION WHERE ZMARKEDFORDELETION = 0"#
    )
    .fetch_one(pool)
    .await?;

    let with_attachments = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!: i64" FROM ZREMCDSAVEDATTACHMENT WHERE ZMARKEDFORDELETION = 0"#
    )
    .fetch_one(pool)
    .await?;

    Ok(ReminderInventory {
        lists: lists as u64,
        reminders: reminders as u64,
        completed: completed as u64,
        with_due_date: with_due_date as u64,
        with_subtasks: with_subtasks as u64,
        with_sections: with_sections as u64,
        with_attachments: with_attachments as u64,
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
