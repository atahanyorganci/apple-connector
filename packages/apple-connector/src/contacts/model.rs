use crate::apple_types::{
    ContactAddressId, ContactEmailId, ContactId, ContactPhoneId, ContactSocialProfileId,
    ContactUrlId, ContainerId, GroupId, SourceId, UnixTimestamp,
};

/// AddressBook account container (CNCDContainer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container {
    pub id: ContainerId,
    pub source_id: SourceId,
    pub name: Option<String>,
    pub container_type: i64,
    pub read_only: bool,
}

pub type ContainerSummary = Container;
pub type ContainerDetail = Container;

/// Contact group (ABCDGroup).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactGroup {
    pub id: GroupId,
    pub source_id: SourceId,
    pub container_id: Option<ContainerId>,
    pub name: Option<String>,
    pub is_smart_group: bool,
    pub is_subscribed: bool,
}

/// Summary row for contact list endpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactSummary {
    pub id: ContactId,
    pub source_id: SourceId,
    pub container_id: Option<ContainerId>,
    pub display_name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub organization: Option<String>,
    pub modification_date: Option<UnixTimestamp>,
}

/// Full contact with multi-value fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactDetail {
    pub id: ContactId,
    pub source_id: SourceId,
    pub container_id: Option<ContainerId>,
    pub display_name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub middle_name: Option<String>,
    pub nickname: Option<String>,
    pub organization: Option<String>,
    pub job_title: Option<String>,
    pub department: Option<String>,
    pub note: Option<String>,
    pub birthday: Option<UnixTimestamp>,
    pub creation_date: Option<UnixTimestamp>,
    pub modification_date: Option<UnixTimestamp>,
    pub phones: Vec<ContactPhone>,
    pub emails: Vec<ContactEmail>,
    pub addresses: Vec<ContactAddress>,
    pub urls: Vec<ContactUrl>,
    pub social_profiles: Vec<ContactSocialProfile>,
    pub group_ids: Vec<GroupId>,
    pub has_photo: bool,
}

pub type Contact = ContactDetail;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactPhone {
    pub id: ContactPhoneId,
    pub label: Option<String>,
    pub number: String,
    pub is_primary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactEmail {
    pub id: ContactEmailId,
    pub label: Option<String>,
    pub address: String,
    pub is_primary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactAddress {
    pub id: ContactAddressId,
    pub label: Option<String>,
    pub street: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub is_primary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactUrl {
    pub id: ContactUrlId,
    pub label: Option<String>,
    pub url: String,
    pub is_primary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactSocialProfile {
    pub id: ContactSocialProfileId,
    pub label: Option<String>,
    pub service: Option<String>,
    pub username: Option<String>,
    pub url: Option<String>,
    pub is_primary: bool,
}

impl ContactDetail {
    /// Convert to interchange vCard model for wire-format serialization.
    pub fn to_vcard(&self) -> serde_vcard::VCard {
        use serde_vcard::{Address, DateOrDateTime, Email, StructuredName, Telephone, VCard};

        VCard {
            uid: Some(self.id.as_str().to_owned()),
            formatted_name: self.display_name.clone().or_else(|| {
                Some(
                    [self.first_name.as_deref(), self.last_name.as_deref()]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                        .join(" "),
                )
            }),
            structured_name: Some(StructuredName {
                given: self.first_name.clone(),
                family: self.last_name.clone(),
                additional: self.middle_name.clone(),
                prefixes: None,
                suffixes: None,
            }),
            nickname: self.nickname.clone(),
            organization: self.organization.clone(),
            title: self.job_title.clone(),
            note: self.note.clone(),
            birthday: self.birthday.and_then(|birthday| {
                chrono::DateTime::from_timestamp(birthday.seconds(), 0)
                    .map(|dt| DateOrDateTime::Date(dt.date_naive()))
            }),
            phones: self
                .phones
                .iter()
                .map(|phone| Telephone {
                    number: phone.number.clone(),
                    label: phone.label.clone(),
                    preferred: phone.is_primary,
                    phone_type: None,
                })
                .collect(),
            emails: self
                .emails
                .iter()
                .map(|email| Email {
                    address: email.address.clone(),
                    label: email.label.clone(),
                    preferred: email.is_primary,
                })
                .collect(),
            addresses: self
                .addresses
                .iter()
                .map(|addr| Address {
                    street: addr.street.clone(),
                    locality: addr.city.clone(),
                    region: addr.state.clone(),
                    postal_code: addr.postal_code.clone(),
                    country: addr.country.clone(),
                    label: addr.label.clone(),
                    preferred: addr.is_primary,
                })
                .collect(),
            ..VCard::default()
        }
    }
}
