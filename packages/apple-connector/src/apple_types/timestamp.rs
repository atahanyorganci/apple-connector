use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Whole seconds since the Unix epoch (`1970-01-01T00:00:00Z`), in UTC.
///
/// Serialized as a JSON integer. This replaces the RFC 3339 timestamp strings
/// emitted by earlier revisions of the API (see the `apple_types` breaking
/// change in the README).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema,
)]
#[serde(transparent)]
#[schema(value_type = i64, example = 1705320000)]
pub struct UnixTimestamp(pub i64);

impl UnixTimestamp {
    /// Wrap a raw Unix-seconds value.
    #[must_use]
    pub const fn from_seconds(seconds: i64) -> Self {
        Self(seconds)
    }

    /// Unwrap to the raw Unix-seconds value.
    #[must_use]
    pub const fn seconds(self) -> i64 {
        self.0
    }
}

impl From<i64> for UnixTimestamp {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl From<chrono::DateTime<chrono::Utc>> for UnixTimestamp {
    fn from(value: chrono::DateTime<chrono::Utc>) -> Self {
        Self(value.timestamp())
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::UnixTimestamp;

    #[test]
    fn serializes_as_bare_integer() {
        let json = serde_json::to_string(&UnixTimestamp::from_seconds(1_705_320_000))
            .expect("serialize timestamp");
        assert_eq!(json, "1705320000");
    }

    #[test]
    fn round_trips_through_json() {
        let value = UnixTimestamp::from_seconds(42);
        let decoded: UnixTimestamp =
            serde_json::from_str(&serde_json::to_string(&value).expect("encode")).expect("decode");
        assert_eq!(decoded, value);
    }

    #[test]
    fn converts_from_utc_datetime_to_seconds() {
        let dt = chrono::Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
        assert_eq!(UnixTimestamp::from(dt).seconds(), dt.timestamp());
    }
}
