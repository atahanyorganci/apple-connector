use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Vendor or unknown vCard properties preserved for round-trip.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionBag {
    #[serde(flatten)]
    pub properties: BTreeMap<String, String>,
}

/// A property this crate does not model, kept verbatim so a round trip is
/// lossless rather than lossy-by-omission.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawProperty {
    /// The `item1` in `item1.TEL`, used by Apple to tie a property to its
    /// `X-ABLabel`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    pub name: String,
    /// Parameter segments exactly as written, e.g. `["TYPE=WORK", "PREF=1"]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<String>,
    pub value: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VCard {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structured_name: Option<StructuredName>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birthday: Option<DateOrDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub photo: Option<Photo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phones: Vec<Telephone>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub emails: Vec<Email>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub addresses: Vec<Address>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub urls: Vec<Url>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub social_profiles: Vec<SocialProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<ExtensionBag>,
    /// Every property outside the modelled set, in document order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unknown: Vec<RawProperty>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredName {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub given: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefixes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suffixes: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Telephone {
    pub number: String,
    /// The TYPE value, or the `X-ABLabel` in the same property group when Apple
    /// supplies a custom one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default)]
    pub preferred: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Email {
    pub address: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default)]
    pub preferred: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub street: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locality: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default)]
    pub preferred: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Url {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default)]
    pub preferred: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DateOrDateTime {
    Date(NaiveDate),
    DateTime(chrono::DateTime<chrono::Utc>),
}

/// RFC 6350 §6.2.4 makes PHOTO a URI. vCard 3.0 instead wrote base64 with
/// `ENCODING=b`; both are accepted on input and the v4 form is written out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Photo {
    /// A reference such as `https://example.com/portrait.jpg`.
    Uri { uri: String },
    /// Image bytes, however they arrived.
    Inline {
        data: Vec<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default)]
    pub preferred: bool,
}
