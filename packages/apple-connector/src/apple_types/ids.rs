use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Error returned when an identifier fails validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdValidationError {
    pub kind: &'static str,
    pub message: String,
}

impl std::fmt::Display for IdValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for IdValidationError {}

/// Declares a transparent string-backed identifier newtype.
///
/// The generated type serializes and deserializes exactly like a bare string
/// while remaining a distinct type in the API surface.
macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident, $example:literal) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema,
        )]
        #[serde(transparent)]
        #[schema(value_type = String, example = $example)]
        pub struct $name(pub String);

        impl $name {
            /// Wrap any string-like value without validation.
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Construct from a non-empty string.
            pub fn try_new(value: impl Into<String>) -> Result<Self, IdValidationError> {
                let value = value.into();
                if value.is_empty() {
                    return Err(IdValidationError {
                        kind: stringify!($name),
                        message: "identifier must not be empty".to_owned(),
                    });
                }
                Ok(Self(value))
            }

            /// Borrow the identifier as a string slice.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Unwrap into the owned string.
            #[must_use]
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl std::str::FromStr for $name {
            type Err = IdValidationError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::try_new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id!(
    /// Stable identifier for a message (its GUID).
    MessageId,
    "A1B2C3D4-E5F6-7890-ABCD-EF1234567890"
);
string_id!(
    /// Stable identifier for an attachment (its GUID).
    AttachmentId,
    "at_0_1234567890ABCDEF"
);
string_id!(
    /// Stable identifier for a reminder (its UUID).
    ReminderId,
    "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"
);
string_id!(
    /// Stable identifier for a reminder list (its UUID).
    ReminderListId,
    "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"
);
string_id!(
    /// Stable identifier for a reminder section (its UUID).
    SectionId,
    "cccccccc-cccc-cccc-cccc-cccccccccccc"
);
string_id!(
    /// Stable identifier for a reminder attachment (its UUID).
    ReminderAttachmentId,
    "dddddddd-dddd-dddd-dddd-dddddddddddd"
);
string_id!(
    /// Stable identifier for a note (its UUID).
    NoteId,
    "eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee"
);
string_id!(
    /// Stable identifier for a note folder (its UUID).
    NoteFolderId,
    "ffffffff-ffff-ffff-ffff-ffffffffffff"
);
string_id!(
    /// Stable identifier for a note attachment (its UUID).
    NoteAttachmentId,
    "11111111-1111-1111-1111-111111111111"
);
string_id!(
    /// Stable identifier for a calendar (its UUID).
    CalendarId,
    "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"
);
string_id!(
    /// Stable identifier for a calendar account (store external id).
    CalendarAccountId,
    "store-icloud"
);
string_id!(
    /// Stable identifier for a calendar event (its UUID).
    EventId,
    "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"
);
string_id!(
    /// Stable identifier for a calendar event attachment (its UUID).
    CalendarAttachmentId,
    "dddddddd-dddd-dddd-dddd-dddddddddddd"
);
string_id!(
    /// AddressBook source UUID (directory name under Sources/).
    SourceId,
    "27fd6c1e-5da5-4340-a31a-1d83c25d3b70"
);
string_id!(
    /// Stable identifier for a contact container (its UUID).
    ContainerId,
    "11111111-1111-1111-1111-111111111111"
);
string_id!(
    /// Stable identifier for a contact group (its UUID).
    GroupId,
    "22222222-2222-2222-2222-222222222222"
);
string_id!(
    /// Stable identifier for a contact (its UUID).
    ContactId,
    "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"
);
string_id!(
    /// Stable identifier for a contact phone value row.
    ContactPhoneId,
    "33333333-3333-3333-3333-333333333333"
);
string_id!(
    /// Stable identifier for a contact email value row.
    ContactEmailId,
    "44444444-4444-4444-4444-444444444444"
);
string_id!(
    /// Stable identifier for a contact address value row.
    ContactAddressId,
    "55555555-5555-5555-5555-555555555555"
);
string_id!(
    /// Stable identifier for a contact URL value row.
    ContactUrlId,
    "66666666-6666-6666-6666-666666666666"
);
string_id!(
    /// Stable identifier for a contact social profile value row.
    ContactSocialProfileId,
    "77777777-7777-7777-7777-777777777777"
);
string_id!(
    /// Messages handle identifier (phone/email row id).
    HandleId,
    "88888888-8888-8888-8888-888888888888"
);

/// SQLite primary-key row identifier shared across Apple databases.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema,
)]
#[serde(transparent)]
#[schema(value_type = i64, example = 1)]
pub struct RowId(pub i64);

impl RowId {
    #[must_use]
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }

    pub fn try_new(value: i64) -> Result<Self, IdValidationError> {
        if value <= 0 {
            Err(IdValidationError {
                kind: "RowId",
                message: "row id must be positive".to_owned(),
            })
        } else {
            Ok(Self(value))
        }
    }
}

impl From<i64> for RowId {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for RowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Internal chat row identifier.
///
/// Serialized as a JSON integer, matching the `chat.ROWID` primary key exposed
/// by the Messages routes.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema,
)]
#[serde(transparent)]
#[schema(value_type = i64, example = 42)]
pub struct ChatId(pub i64);

impl ChatId {
    /// Wrap a raw chat row id without validation.
    #[must_use]
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    pub fn try_new(value: i64) -> Result<Self, IdValidationError> {
        if value <= 0 {
            Err(IdValidationError {
                kind: "ChatId",
                message: "chat id must be positive".to_owned(),
            })
        } else {
            Ok(Self(value))
        }
    }

    /// Unwrap to the raw chat row id.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl std::fmt::Display for ChatId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i64> for ChatId {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl std::str::FromStr for ChatId {
    type Err = IdValidationError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parsed = value.parse::<i64>().map_err(|_| IdValidationError {
            kind: "ChatId",
            message: "chat id must be a positive integer".to_owned(),
        })?;
        Self::try_new(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::{ChatId, MessageId};

    #[test]
    fn string_id_serializes_transparently() -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(&MessageId::new("guid-1"))?;
        assert_eq!(json, "\"guid-1\"");
        Ok(())
    }

    #[test]
    fn string_id_round_trips() -> Result<(), Box<dyn std::error::Error>> {
        let value = MessageId::new("guid-2");
        let decoded: MessageId = serde_json::from_str("\"guid-2\"")?;
        assert_eq!(decoded, value);
        assert_eq!(value.as_str(), "guid-2");
        Ok(())
    }

    #[test]
    fn chat_id_serializes_as_integer() -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(&ChatId::new(7))?;
        assert_eq!(json, "7");
        Ok(())
    }
}
