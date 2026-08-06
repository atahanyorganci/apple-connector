use sqlx::SqlitePool;

use super::{
    queries::{
        count_attachments, count_calendars, count_events, count_hidden_events, count_occurrences,
        count_recurring_events, count_stores,
    },
    schema::{CalendarSchemaVariant, detect_schema_variant},
};

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
        CalendarSchemaVariant::CalendarItem => (
            count_stores(pool).await?,
            count_calendars(pool).await?,
            count_events(pool).await?,
            count_recurring_events(pool).await?,
            count_occurrences(pool).await?,
            count_attachments(pool).await?,
            count_hidden_events(pool).await?,
        ),
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
    async fn inventory_counts_seeded_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = CalendarFixtureDb::seeded().await?;
        let pool = connect_pool(fixture.path()).await?;
        let inventory = load_inventory(&pool).await?;
        assert_eq!(inventory.schema_variant, "CalendarItem");
        assert!(inventory.calendars >= 1);
        assert!(inventory.events >= 2);
        Ok(())
    }
}
