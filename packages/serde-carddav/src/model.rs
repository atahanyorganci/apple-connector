use serde::{Deserialize, Serialize};
use serde_vcard::VCard;

/// CardDAV address-object resource wrapping an embedded vCard payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardDavAddressObject {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    pub vcard: VCard,
}

/// CardDAV addressbook collection resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardDavAddressBookResource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub displayname: Option<String>,
}

/// CardDAV multistatus response envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardDavMultistatus {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub responses: Vec<CardDavResponse>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardDavResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_object: Option<CardDavAddressObject>,
}
