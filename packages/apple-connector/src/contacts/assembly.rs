use crate::{
    apple_types::{ContactId, ContainerId, GroupId, SourceId, UnixTimestamp},
    contacts::{
        labels::decode_label,
        model::{
            ContactAddress, ContactDetail, ContactEmail, ContactGroup, ContactPhone,
            ContactSocialProfile, ContactSummary, ContactUrl, Container,
        },
        row::{
            AddressRow, ContactRow, ContainerRow, EmailRow, GroupRow, PhoneRow, SocialRow,
            UrlRow, api_id_from_unique_id, parse_core_data_secs,
        },
    },
};

pub struct ContactRelatedRows {
    pub phones: Vec<PhoneRow>,
    pub emails: Vec<EmailRow>,
    pub addresses: Vec<AddressRow>,
    pub urls: Vec<UrlRow>,
    pub socials: Vec<SocialRow>,
}

pub fn container_from_row(row: ContainerRow, source_id: SourceId) -> Container {
    Container {
        id: ContainerId::new(api_id_from_unique_id(&row.unique_id)),
        source_id,
        name: row.name,
        container_type: row.container_type.unwrap_or(0),
        read_only: false,
    }
}

pub fn group_from_row(row: GroupRow, source_id: SourceId) -> ContactGroup {
    ContactGroup {
        id: GroupId::new(api_id_from_unique_id(&row.unique_id)),
        source_id,
        container_id: row
            .container_unique_id
            .map(|id| ContainerId::new(api_id_from_unique_id(&id)))
            .unwrap_or_else(|| ContainerId::new("unknown")),
        name: row.name,
        is_smart_group: row.group_type.is_some_and(|t| t != 0),
        is_subscribed: false,
    }
}

pub fn contact_summary_from_row(row: ContactRow, source_id: SourceId) -> ContactSummary {
    ContactSummary {
        id: ContactId::new(api_id_from_unique_id(&row.unique_id)),
        source_id,
        container_id: row
            .container_unique_id
            .map(|id| ContainerId::new(api_id_from_unique_id(&id)))
            .unwrap_or_else(|| ContainerId::new("unknown")),
        display_name: row.display_name.or_else(|| {
            Some(
                [row.first_name.as_deref(), row.last_name.as_deref()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        }),
        first_name: row.first_name,
        last_name: row.last_name,
        organization: row.organization,
        modification_date: parse_core_data_secs(row.modification_date).map(UnixTimestamp::from),
    }
}

pub fn contact_detail_from_row(
    row: ContactRow,
    source_id: SourceId,
    related: ContactRelatedRows,
    group_ids: Vec<String>,
) -> ContactDetail {
    let ContactRelatedRows {
        phones,
        emails,
        addresses,
        urls,
        socials,
    } = related;
    ContactDetail {
        id: ContactId::new(api_id_from_unique_id(&row.unique_id)),
        source_id,
        container_id: row
            .container_unique_id
            .map(|id| ContainerId::new(api_id_from_unique_id(&id)))
            .unwrap_or_else(|| ContainerId::new("unknown")),
        display_name: row.display_name,
        first_name: row.first_name,
        last_name: row.last_name,
        middle_name: row.middle_name,
        nickname: row.nickname,
        organization: row.organization,
        job_title: row.job_title,
        department: row.department,
        note: row.note_text,
        birthday: parse_core_data_secs(row.birthday).map(UnixTimestamp::from),
        creation_date: parse_core_data_secs(row.creation_date).map(UnixTimestamp::from),
        modification_date: parse_core_data_secs(row.modification_date).map(UnixTimestamp::from),
        phones: phones
            .into_iter()
            .filter_map(|phone| {
                Some(ContactPhone {
                    id: api_id_from_unique_id(&phone.unique_id),
                    label: decode_label(phone.label.as_deref()),
                    number: phone.number?,
                    is_primary: phone.is_primary.is_some_and(|v| v != 0),
                })
            })
            .collect(),
        emails: emails
            .into_iter()
            .filter_map(|email| {
                Some(ContactEmail {
                    id: api_id_from_unique_id(&email.unique_id),
                    label: decode_label(email.label.as_deref()),
                    address: email.address?,
                    is_primary: email.is_primary.is_some_and(|v| v != 0),
                })
            })
            .collect(),
        addresses: addresses
            .into_iter()
            .map(|addr| ContactAddress {
                id: api_id_from_unique_id(&addr.unique_id),
                label: decode_label(addr.label.as_deref()),
                street: addr.street,
                city: addr.city,
                state: addr.state,
                postal_code: addr.postal_code,
                country: addr.country,
                is_primary: addr.is_primary.is_some_and(|v| v != 0),
            })
            .collect(),
        urls: urls
            .into_iter()
            .filter_map(|url| {
                Some(ContactUrl {
                    id: api_id_from_unique_id(&url.unique_id),
                    label: decode_label(url.label.as_deref()),
                    url: url.url?,
                    is_primary: url.is_primary.is_some_and(|v| v != 0),
                })
            })
            .collect(),
        social_profiles: socials
            .into_iter()
            .map(|profile| ContactSocialProfile {
                id: api_id_from_unique_id(&profile.unique_id),
                label: decode_label(profile.label.as_deref()),
                service: profile.service,
                username: profile.username,
                url: profile.url,
                is_primary: profile.is_primary.is_some_and(|v| v != 0),
            })
            .collect(),
        group_ids: group_ids
            .into_iter()
            .map(|id| GroupId::new(api_id_from_unique_id(&id)))
            .collect(),
        has_photo: row.has_photo.is_some_and(|v| v != 0),
    }
}
