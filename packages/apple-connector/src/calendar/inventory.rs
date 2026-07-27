use sqlx::SqlitePool;

use super::schema::{CalendarSchemaVariant, detect_schema_variant};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CalendarInventory {
    pub schema_variant: String,
    pub stores: u64,
    pub calendars: u64,
    pub events: u64,
    pub recurring_events: u64,
    pub occurrences: u64,
    pub attachments: u64,
    pub hidden_events: u64,
}

pub async fn load_inventory(pool: &SqlitePool) -> Result<CalendarInventory, sqlx::Error> {
    let variant = detect_schema_variant(pool).await?;
    let (stores, calendars, events, recurring, occurrences, attachments, hidden) = match variant {
        CalendarSchemaVariant::CalendarItem => {
            let stores: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM Store").fetch_one(pool).await?;
            let calendars: (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM Calendar").fetch_one(pool).await?;
            let events: (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM CalendarItem").fetch_one(pool).await?;
            let recurring: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM CalendarItem WHERE has_recurrences = 1",
            )
            .fetch_one(pool)
            .await?;
            let occurrences: (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM OccurrenceCache").fetch_one(pool).await?;
            let attachments: (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM Attachment").fetch_one(pool).await?;
            let hidden: (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM CalendarItem WHERE hidden = 1").fetch_one(pool).await?;
            (
                stores.0,
                calendars.0,
                events.0,
                recurring.0,
                occurrences.0,
                attachments.0,
                hidden.0,
            )
        }
        CalendarSchemaVariant::ZCalendarItem => (0, 0, 0, 0, 0, 0, 0),
    };

    Ok(CalendarInventory {
        schema_variant: variant.as_str().to_owned(),
        stores: stores as u64,
        calendars: calendars as u64,
        events: events as u64,
        recurring_events: recurring as u64,
        occurrences: occurrences as u64,
        attachments: attachments as u64,
        hidden_events: hidden as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::load_inventory;
    use crate::{connect_pool, fixtures::CalendarFixtureDb};

    #[tokio::test]
    async fn inventory_counts_seeded_fixture() {
        let fixture = CalendarFixtureDb::seeded().await.expect("fixture");
        let pool = connect_pool(fixture.path()).await.expect("pool");
        let inventory = load_inventory(&pool).await.expect("inventory");
        assert_eq!(inventory.schema_variant, "CalendarItem");
        assert!(inventory.calendars >= 1);
        assert!(inventory.events >= 2);
    }
}
