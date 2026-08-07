use sqlx::SqlitePool;

use super::queries::count_calendar_item_table;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarSchemaVariant {
    /// Modern schema using `CalendarItem`, `Calendar`, etc.
    CalendarItem,
}

impl CalendarSchemaVariant {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CalendarItem => "CalendarItem",
        }
    }
}

/// Require the modern `CalendarItem` table. Legacy `ZCALENDARITEM` databases are unsupported.
pub async fn detect_schema_variant(
    pool: &SqlitePool,
) -> Result<CalendarSchemaVariant, sqlx::Error> {
    if count_calendar_item_table(pool).await? == 0 {
        return Err(sqlx::Error::Configuration(
            "Calendar database is missing the modern CalendarItem table; legacy ZCALENDARITEM schemas are unsupported".into(),
        ));
    }
    Ok(CalendarSchemaVariant::CalendarItem)
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

    #[tokio::test]
    async fn rejects_database_without_calendar_item_table() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = CalendarFixtureDb::legacy_unsupported().await?;
        let pool = connect_pool(fixture.path()).await?;
        match detect_schema_variant(&pool).await {
            Ok(variant) => Err(format!("expected schema rejection, got {variant:?}").into()),
            Err(err) => {
                assert!(
                    err.to_string().contains("CalendarItem"),
                    "unexpected error: {err}"
                );
                Ok(())
            }
        }
    }
}
