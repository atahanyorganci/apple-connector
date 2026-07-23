use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
            /// Wrap any string-like value.
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
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
    /// Wrap a raw chat row id.
    #[must_use]
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    /// Unwrap to the raw chat row id.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl From<i64> for ChatId {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{ChatId, MessageId};

    #[test]
    fn string_id_serializes_transparently() {
        let json = serde_json::to_string(&MessageId::new("guid-1")).expect("serialize");
        assert_eq!(json, "\"guid-1\"");
    }

    #[test]
    fn string_id_round_trips() {
        let value = MessageId::new("guid-2");
        let decoded: MessageId =
            serde_json::from_str("\"guid-2\"").expect("deserialize message id");
        assert_eq!(decoded, value);
        assert_eq!(value.as_str(), "guid-2");
    }

    #[test]
    fn chat_id_serializes_as_integer() {
        let json = serde_json::to_string(&ChatId::new(7)).expect("serialize");
        assert_eq!(json, "7");
    }
}
