use apple_contacts::{
    ContactsError, ContainerResolveHint, CreateContactInput, CreateGroupInput, LabeledStringInput,
    PostalAddressInput, UpdateContactInput, UpdateGroupInput,
};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    api::{
        dto::contacts::{
            CreateContactRequest, CreateGroupRequest, LabeledStringDto, PostalAddressDto,
            UpdateContactRequest, UpdateGroupRequest,
        },
        error::ApiError,
    },
    contacts::{Container, ContainerResolveMetadata},
};

pub const VCARD_CONTENT_TYPE: &str = "text/vcard; charset=utf-8";
pub const CARDDAV_CONTENT_TYPE: &str = "application/carddav+xml; charset=utf-8";

pub fn map_contacts_error(error: ContactsError) -> ApiError {
    match error {
        ContactsError::NotFound => ApiError::not_found("Contacts item not found"),
        ContactsError::AccessDenied => ApiError::forbidden("Contacts access denied"),
        ContactsError::ReadOnlyContainer => ApiError::forbidden("target container is read-only"),
        ContactsError::ValidationFailed(message) => ApiError::unprocessable(message),
        ContactsError::UnsupportedPlatform => ApiError::contacts_unavailable(),
        ContactsError::Framework(message) => ApiError::internal(message),
        ContactsError::Timeout => ApiError::internal("Contacts operation timed out"),
    }
}

pub fn container_hint(
    container: &Container,
    metadata: ContainerResolveMetadata,
) -> ContainerResolveHint {
    ContainerResolveHint {
        api_id: metadata.api_id,
        external_id: Some(metadata.external_id),
        name: metadata.name.or(container.name.clone()),
        read_only: container.read_only,
    }
}

pub fn create_contact_input(request: CreateContactRequest) -> CreateContactInput {
    CreateContactInput {
        given_name: request.given_name,
        family_name: request.family_name,
        middle_name: request.middle_name,
        nickname: request.nickname,
        organization_name: request.organization_name,
        job_title: request.job_title,
        department_name: request.department_name,
        note: request.note,
        phone_numbers: request
            .phone_numbers
            .into_iter()
            .map(labeled_string_input)
            .collect(),
        email_addresses: request
            .email_addresses
            .into_iter()
            .map(labeled_string_input)
            .collect(),
        postal_addresses: request
            .postal_addresses
            .into_iter()
            .map(postal_address_input)
            .collect(),
        url_addresses: request
            .url_addresses
            .into_iter()
            .map(labeled_string_input)
            .collect(),
    }
}

pub fn update_contact_input(request: UpdateContactRequest) -> UpdateContactInput {
    UpdateContactInput {
        given_name: request.given_name,
        family_name: request.family_name,
        middle_name: request.middle_name,
        nickname: request.nickname,
        organization_name: request.organization_name,
        job_title: request.job_title,
        department_name: request.department_name,
        note: request.note,
        phone_numbers: request
            .phone_numbers
            .map(|values| values.into_iter().map(labeled_string_input).collect()),
        email_addresses: request
            .email_addresses
            .map(|values| values.into_iter().map(labeled_string_input).collect()),
        postal_addresses: request
            .postal_addresses
            .map(|values| values.into_iter().map(postal_address_input).collect()),
        url_addresses: request
            .url_addresses
            .map(|values| values.into_iter().map(labeled_string_input).collect()),
    }
}

pub fn create_group_input(request: CreateGroupRequest) -> CreateGroupInput {
    CreateGroupInput { name: request.name }
}

pub fn update_group_input(request: UpdateGroupRequest) -> UpdateGroupInput {
    UpdateGroupInput { name: request.name }
}

fn labeled_string_input(value: LabeledStringDto) -> LabeledStringInput {
    LabeledStringInput {
        label: value.label,
        value: value.value,
    }
}

fn postal_address_input(value: PostalAddressDto) -> PostalAddressInput {
    PostalAddressInput {
        label: value.label,
        street: value.street,
        city: value.city,
        state: value.state,
        postal_code: value.postal_code,
        country: value.country,
    }
}

pub fn contact_detail_vcard(
    contact: &crate::contacts::ContactDetail,
) -> Result<Response, ApiError> {
    let body = serde_vcard::to_string(&contact.to_vcard())
        .map_err(|error| ApiError::internal(error.to_string()))?;
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, VCARD_CONTENT_TYPE)],
        body,
    )
        .into_response())
}

pub fn contact_detail_carddav(
    contact: &crate::contacts::ContactDetail,
) -> Result<Response, ApiError> {
    let object = serde_carddav::CardDavAddressObject {
        href: Some(format!("/v1/contacts/{}/carddav", contact.id)),
        etag: None,
        content_type: Some(VCARD_CONTENT_TYPE.to_owned()),
        vcard: contact.to_vcard(),
    };
    let body =
        serde_carddav::to_string(&object).map_err(|error| ApiError::internal(error.to_string()))?;
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, CARDDAV_CONTENT_TYPE)],
        body,
    )
        .into_response())
}

pub fn contact_page_vcard(
    contacts: &[crate::contacts::ContactDetail],
) -> Result<Response, ApiError> {
    let mut body = String::new();
    for contact in contacts {
        body.push_str(
            &serde_vcard::to_string(&contact.to_vcard())
                .map_err(|error| ApiError::internal(error.to_string()))?,
        );
        body.push('\n');
    }
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, VCARD_CONTENT_TYPE)],
        body,
    )
        .into_response())
}

pub fn contact_page_carddav(
    contacts: &[crate::contacts::ContactDetail],
) -> Result<Response, ApiError> {
    let mut xml_parts = Vec::new();
    for contact in contacts {
        let object = serde_carddav::CardDavAddressObject {
            href: Some(format!("/v1/contacts/{}/carddav", contact.id)),
            etag: None,
            content_type: Some(VCARD_CONTENT_TYPE.to_owned()),
            vcard: contact.to_vcard(),
        };
        xml_parts.push(
            serde_carddav::to_string(&object)
                .map_err(|error| ApiError::internal(error.to_string()))?,
        );
    }
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, CARDDAV_CONTENT_TYPE)],
        xml_parts.join("\n"),
    )
        .into_response())
}
