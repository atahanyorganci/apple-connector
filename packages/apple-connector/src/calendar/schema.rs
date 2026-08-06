use sqlx::SqlitePool;

use super::queries::{count_calendar_item_table, count_zcalendaritem_table};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarSchemaVariant {
    /// Modern schema using `CalendarItem`, `Calendar`, etc.
    CalendarItem,
    /// Legacy Core Data schema using `ZCALENDARITEM`, etc.
    ZCalendarItem,
}

impl CalendarSchemaVariant {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CalendarItem => "CalendarItem",
            Self::ZCalendarItem => "ZCALENDARITEM",
        }
    }
}

pub async fn detect_schema_variant(
    pool: &SqlitePool,
) -> Result<CalendarSchemaVariant, sqlx::Error> {
    if count_calendar_item_table(pool).await? > 0 {
        return Ok(CalendarSchemaVariant::CalendarItem);
    }

    if count_zcalendaritem_table(pool).await? > 0 {
        return Ok(CalendarSchemaVariant::ZCalendarItem);
    }

    Ok(CalendarSchemaVariant::CalendarItem)
}

#[allow(dead_code)]
pub fn event_table(variant: CalendarSchemaVariant) -> &'static str {
    match variant {
        CalendarSchemaVariant::CalendarItem => "CalendarItem",
        CalendarSchemaVariant::ZCalendarItem => "ZCALENDARITEM",
    }
}

#[allow(dead_code)]
pub fn calendar_table(variant: CalendarSchemaVariant) -> &'static str {
    match variant {
        CalendarSchemaVariant::CalendarItem => "Calendar",
        CalendarSchemaVariant::ZCalendarItem => "ZCALENDAR",
    }
}

#[allow(dead_code)]
pub fn store_table(variant: CalendarSchemaVariant) -> &'static str {
    match variant {
        CalendarSchemaVariant::CalendarItem => "Store",
        CalendarSchemaVariant::ZCalendarItem => "ZSTORE",
    }
}

#[cfg(test)]
mod tests {
    use super::{CalendarSchemaVariant, detect_schema_variant};
    use crate::{connect_pool, fixtures::CalendarFixtureDb};

    #[tokio::test]
    async fn fixture_uses_modern_schema() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = CalendarFixtureDb::empty().await?;
        let pool = connect_pool(fixture.path()).await?;
        let variant = detect_schema_variant(&pool).await?;
        assert_eq!(variant, CalendarSchemaVariant::CalendarItem);
        Ok(())
    }
}
