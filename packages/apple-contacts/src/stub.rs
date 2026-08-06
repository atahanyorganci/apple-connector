use crate::{
    contact::{CreateContactInput, SavedContact, UpdateContactInput},
    container::ContainerResolveHint,
    error::{ContactsError, ContactsResult},
    group::{CreateGroupInput, SavedGroup, UpdateGroupInput},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthStatus {
    NotDetermined,
    Restricted,
    Denied,
    Authorized,
    Limited,
    Unavailable,
}

pub struct ContactsStore;

impl ContactsStore {
    pub fn new() -> ContactsResult<Self> {
        Err(ContactsError::UnsupportedPlatform)
    }

    pub async fn auth_status(&self) -> AuthStatus {
        AuthStatus::Unavailable
    }

    pub async fn refresh_auth_status(&self) {}

    pub async fn request_access(&self) -> ContactsResult<()> {
        Ok(())
    }

    pub async fn ensure_contacts_access(&self) -> ContactsResult<()> {
        Ok(())
    }

    pub async fn create_contact(
        &self,
        _container_hint: ContainerResolveHint,
        _input: CreateContactInput,
    ) -> ContactsResult<SavedContact> {
        Err(ContactsError::UnsupportedPlatform)
    }

    pub async fn update_contact(
        &self,
        _contact_id: &str,
        _input: UpdateContactInput,
    ) -> ContactsResult<SavedContact> {
        Err(ContactsError::UnsupportedPlatform)
    }

    pub async fn delete_contact(&self, _contact_id: &str) -> ContactsResult<()> {
        Err(ContactsError::UnsupportedPlatform)
    }

    pub async fn create_group(
        &self,
        _container_hint: ContainerResolveHint,
        _input: CreateGroupInput,
    ) -> ContactsResult<SavedGroup> {
        Err(ContactsError::UnsupportedPlatform)
    }

    pub async fn update_group(
        &self,
        _group_id: &str,
        _input: UpdateGroupInput,
    ) -> ContactsResult<SavedGroup> {
        Err(ContactsError::UnsupportedPlatform)
    }

    pub async fn delete_group(&self, _group_id: &str) -> ContactsResult<()> {
        Err(ContactsError::UnsupportedPlatform)
    }

    pub async fn add_contact_to_group(
        &self,
        _contact_id: &str,
        _group_id: &str,
    ) -> ContactsResult<()> {
        Err(ContactsError::UnsupportedPlatform)
    }

    pub async fn remove_contact_from_group(
        &self,
        _contact_id: &str,
        _group_id: &str,
    ) -> ContactsResult<()> {
        Err(ContactsError::UnsupportedPlatform)
    }
}
