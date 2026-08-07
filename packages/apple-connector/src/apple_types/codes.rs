use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Apple Reminders priority code (0 = none, 1–4 high, 5 medium, 6–9 low).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema,
)]
#[serde(transparent)]
#[schema(value_type = i64, example = 5)]
pub struct ReminderPriority(i64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeValidationError {
    pub kind: &'static str,
    pub value: i64,
    pub message: String,
}

impl std::fmt::Display for CodeValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for CodeValidationError {}

impl ReminderPriority {
    #[must_use]
    pub const fn none() -> Self {
        Self(0)
    }

    pub fn try_new(value: i64) -> Result<Self, CodeValidationError> {
        if (0..=9).contains(&value) {
            Ok(Self(value))
        } else {
            Err(CodeValidationError {
                kind: "ReminderPriority",
                value,
                message: format!("priority must be between 0 and 9, got {value}"),
            })
        }
    }

    #[must_use]
    pub const fn raw(self) -> i64 {
        self.0
    }

    #[must_use]
    pub fn category(self) -> ReminderPriorityCategory {
        match self.0 {
            1..=4 => ReminderPriorityCategory::High,
            5 => ReminderPriorityCategory::Medium,
            6..=9 => ReminderPriorityCategory::Low,
            _ => ReminderPriorityCategory::None,
        }
    }
}

impl From<ReminderPriority> for i64 {
    fn from(value: ReminderPriority) -> Self {
        value.raw()
    }
}

impl TryFrom<i64> for ReminderPriority {
    type Error = CodeValidationError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReminderPriorityCategory {
    None,
    Low,
    Medium,
    High,
}

#[cfg(test)]
mod tests {
    use super::ReminderPriority;

    #[test]
    fn reminder_priority_round_trips_zero_through_nine() -> Result<(), Box<dyn std::error::Error>> {
        for value in 0..=9 {
            let priority = ReminderPriority::try_new(value)?;
            assert_eq!(priority.raw(), value);
            let again = ReminderPriority::try_new(priority.raw())?;
            assert_eq!(again, priority);
        }
        Ok(())
    }
}
