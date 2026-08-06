use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Seconds between the Unix epoch and Apple's Core Data reference date (2001-01-01 UTC).
pub const CORE_DATA_EPOCH_UNIX_SECS: i64 = 978_307_200;

fn f64_to_i64_secs(value: f64) -> i64 {
    if !value.is_finite() {
        return 0;
    }
    if value >= i64::MAX as f64 {
        return i64::MAX;
    }
    if value <= i64::MIN as f64 {
        return i64::MIN;
    }
    #[expect(clippy::cast_possible_truncation)]
    {
        value.trunc() as i64
    }
}

fn f64_fraction_to_subsec_nanos(fraction: f64) -> u32 {
    let nanos = (fraction * 1_000_000_000.0).round();
    if nanos <= 0.0 {
        return 0;
    }
    if nanos >= f64::from(u32::MAX) {
        return u32::MAX;
    }
    #[expect(clippy::cast_possible_truncation)]
    {
        nanos as u32
    }
}

/// Parse a Core Data timestamp (seconds since 2001-01-01 UTC). Zero means unset.
#[must_use]
pub fn parse_core_data_timestamp(secs: Option<f64>) -> Option<DateTime<Utc>> {
    let secs = secs?;
    if secs <= 0.0 {
        return None;
    }
    let whole_secs = f64_to_i64_secs(secs) + CORE_DATA_EPOCH_UNIX_SECS;
    let nanos = f64_fraction_to_subsec_nanos(secs.fract());
    DateTime::from_timestamp(whole_secs, nanos)
}

/// Encode a UTC datetime as Core Data seconds since 2001-01-01.
#[must_use]
pub fn core_data_secs_from_timestamp(dt: DateTime<Utc>) -> f64 {
    (dt.timestamp() - CORE_DATA_EPOCH_UNIX_SECS) as f64
        + f64::from(dt.timestamp_subsec_nanos()) / 1_000_000_000.0
}

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
