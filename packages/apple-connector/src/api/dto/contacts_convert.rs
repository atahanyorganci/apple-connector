use super::{
    contacts::{
        ContactAddressDto, ContactDetailDto, ContactEmailDto, ContactPageDto, ContactPhoneDto,
        ContactSocialProfileDto, ContactSummaryDto, ContactUrlDto, ContainerDetailDto,
        ContainerPageDto, ContainerSummaryDto, GroupDetailDto, GroupPageDto, GroupSummaryDto,
    },
    pagination::PageMetaDto,
};
use crate::{
    apple_types::{ContactId, ContainerId, GroupId},
    contacts::{
        ContactAddress, ContactDetail, ContactEmail, ContactGroup, ContactPhone,
        ContactSocialProfile, ContactSummary, ContactUrl, Container, Page,
    },
};

pub fn container_page_to_dto(items: Vec<Container>) -> ContainerPageDto {
    ContainerPageDto {
        items: items.iter().map(container_summary_to_dto).collect(),
    }
}

pub fn container_summary_to_dto(container: &Container) -> ContainerSummaryDto {
    ContainerSummaryDto {
        id: ContainerId::new(container.id.as_str()),
        source_id: container.source_id.clone(),
        name: container.name.clone(),
        container_type: container.container_type,
        read_only: container.read_only,
    }
}

pub fn container_detail_to_dto(container: &Container) -> ContainerDetailDto {
    container_summary_to_dto(container)
}

pub fn group_page_to_dto(
    items: Vec<ContactGroup>,
    has_more: bool,
    next_cursor: Option<String>,
    limit: u32,
) -> GroupPageDto {
    GroupPageDto {
        items: items.iter().map(group_summary_to_dto).collect(),
        page: PageMetaDto {
            limit,
            has_more,
            next_cursor,
        },
    }
}

pub fn group_summary_to_dto(group: &ContactGroup) -> GroupSummaryDto {
    GroupSummaryDto {
        id: GroupId::new(group.id.as_str()),
        source_id: group.source_id.clone(),
        container_id: ContainerId::new(group.container_id.as_str()),
        name: group.name.clone(),
        is_smart_group: group.is_smart_group,
        is_subscribed: group.is_subscribed,
    }
}

pub fn group_detail_to_dto(group: &ContactGroup) -> GroupDetailDto {
    group_summary_to_dto(group)
}

pub fn contact_page_to_dto(page: Page<ContactSummary>, limit: u32) -> ContactPageDto {
    ContactPageDto {
        items: page.items.iter().map(contact_summary_to_dto).collect(),
        page: PageMetaDto {
            limit,
            has_more: page.has_more,
            next_cursor: page.next_cursor,
        },
    }
}

pub fn contact_summary_to_dto(contact: &ContactSummary) -> ContactSummaryDto {
    ContactSummaryDto {
        id: ContactId::new(contact.id.as_str()),
        source_id: contact.source_id.clone(),
        container_id: ContainerId::new(contact.container_id.as_str()),
        display_name: contact.display_name.clone(),
        first_name: contact.first_name.clone(),
        last_name: contact.last_name.clone(),
        organization: contact.organization.clone(),
        modification_date: contact.modification_date,
    }
}

pub fn contact_detail_to_dto(contact: &ContactDetail) -> ContactDetailDto {
    ContactDetailDto {
        summary: contact_summary_to_dto(&ContactSummary {
            id: contact.id.clone(),
            source_id: contact.source_id.clone(),
            container_id: contact.container_id.clone(),
            display_name: contact.display_name.clone(),
            first_name: contact.first_name.clone(),
            last_name: contact.last_name.clone(),
            organization: contact.organization.clone(),
            modification_date: contact.modification_date,
        }),
        middle_name: contact.middle_name.clone(),
        nickname: contact.nickname.clone(),
        job_title: contact.job_title.clone(),
        department: contact.department.clone(),
        note: contact.note.clone(),
        birthday: contact.birthday,
        creation_date: contact.creation_date,
        phones: contact.phones.iter().map(contact_phone_to_dto).collect(),
        emails: contact.emails.iter().map(contact_email_to_dto).collect(),
        addresses: contact
            .addresses
            .iter()
            .map(contact_address_to_dto)
            .collect(),
        urls: contact.urls.iter().map(contact_url_to_dto).collect(),
        social_profiles: contact
            .social_profiles
            .iter()
            .map(contact_social_profile_to_dto)
            .collect(),
        group_ids: contact.group_ids.clone(),
        has_photo: contact.has_photo,
    }
}

fn contact_phone_to_dto(phone: &ContactPhone) -> ContactPhoneDto {
    ContactPhoneDto {
        id: phone.id.clone(),
        label: phone.label.clone(),
        number: phone.number.clone(),
        is_primary: phone.is_primary,
    }
}

fn contact_email_to_dto(email: &ContactEmail) -> ContactEmailDto {
    ContactEmailDto {
        id: email.id.clone(),
        label: email.label.clone(),
        address: email.address.clone(),
        is_primary: email.is_primary,
    }
}

fn contact_address_to_dto(address: &ContactAddress) -> ContactAddressDto {
    ContactAddressDto {
        id: address.id.clone(),
        label: address.label.clone(),
        street: address.street.clone(),
        city: address.city.clone(),
        state: address.state.clone(),
        postal_code: address.postal_code.clone(),
        country: address.country.clone(),
        is_primary: address.is_primary,
    }
}

fn contact_url_to_dto(url: &ContactUrl) -> ContactUrlDto {
    ContactUrlDto {
        id: url.id.clone(),
        label: url.label.clone(),
        url: url.url.clone(),
        is_primary: url.is_primary,
    }
}

fn contact_social_profile_to_dto(profile: &ContactSocialProfile) -> ContactSocialProfileDto {
    ContactSocialProfileDto {
        id: profile.id.clone(),
        label: profile.label.clone(),
        service: profile.service.clone(),
        username: profile.username.clone(),
        url: profile.url.clone(),
        is_primary: profile.is_primary,
    }
}
