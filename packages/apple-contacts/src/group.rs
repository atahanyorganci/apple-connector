use crate::error::{ContactsError, ContactsResult};
#[cfg(target_os = "macos")]
use crate::{container::ContainerResolveHint, store::ContactsStore};

#[derive(Debug, Clone)]
pub struct CreateGroupInput {
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateGroupInput {
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedGroup {
    pub identifier: String,
}

#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2_contacts::{CNContactStore, CNGroup, CNMutableGroup, CNSaveRequest};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSArray, NSString};

#[cfg(target_os = "macos")]
impl ContactsStore {
    pub async fn create_group(
        &self,
        container_hint: ContainerResolveHint,
        input: CreateGroupInput,
    ) -> ContactsResult<SavedGroup> {
        self.ensure_contacts()?;
        validate_group_name(&input.name)?;
        self.run_on_main(move |store| {
            let (container, _) = crate::container::resolve_container(store, &container_hint)?;
            let container_id = unsafe { container.identifier() };
            let group = unsafe { CNMutableGroup::new() };
            let name = NSString::from_str(&input.name);
            unsafe { group.setName(&name) };
            let request = unsafe { CNSaveRequest::new() };
            unsafe {
                request.addGroup_toContainerWithIdentifier(&group, Some(&container_id));
            }
            crate::contact::execute_save(store, &request)?;
            Ok(SavedGroup {
                identifier: unsafe { group.identifier().to_string() },
            })
        })
        .await
    }

    pub async fn update_group(
        &self,
        group_id: &str,
        input: UpdateGroupInput,
    ) -> ContactsResult<SavedGroup> {
        self.ensure_contacts()?;
        let group_id = group_id.to_owned();
        self.run_on_main(move |store| {
            let group = lookup_group(store, &group_id)?;
            let mutable = mutable_group(&group)?;
            if let Some(name) = input.name {
                validate_group_name(&name)?;
                let ns = NSString::from_str(&name);
                unsafe { mutable.setName(&ns) };
            }
            let request = unsafe { CNSaveRequest::new() };
            unsafe { request.updateGroup(&mutable) };
            crate::contact::execute_save(store, &request)?;
            Ok(SavedGroup {
                identifier: unsafe { mutable.identifier().to_string() },
            })
        })
        .await
    }

    pub async fn delete_group(&self, group_id: &str) -> ContactsResult<()> {
        self.ensure_contacts()?;
        let group_id = group_id.to_owned();
        self.run_on_main(move |store| {
            let group = lookup_group(store, &group_id)?;
            let mutable = mutable_group(&group)?;
            let request = unsafe { CNSaveRequest::new() };
            unsafe { request.deleteGroup(&mutable) };
            crate::contact::execute_save(store, &request)
        })
        .await
    }

    pub async fn add_contact_to_group(
        &self,
        contact_id: &str,
        group_id: &str,
    ) -> ContactsResult<()> {
        self.ensure_contacts()?;
        let contact_id = contact_id.to_owned();
        let group_id = group_id.to_owned();
        self.run_on_main(move |store| {
            let contact = crate::contact::lookup_contact(store, &contact_id)?;
            let group = lookup_group(store, &group_id)?;
            let request = unsafe { CNSaveRequest::new() };
            unsafe { request.addMember_toGroup(&contact, &group) };
            crate::contact::execute_save(store, &request)
        })
        .await
    }

    pub async fn remove_contact_from_group(
        &self,
        contact_id: &str,
        group_id: &str,
    ) -> ContactsResult<()> {
        self.ensure_contacts()?;
        let contact_id = contact_id.to_owned();
        let group_id = group_id.to_owned();
        self.run_on_main(move |store| {
            let contact = crate::contact::lookup_contact(store, &contact_id)?;
            let group = lookup_group(store, &group_id)?;
            let request = unsafe { CNSaveRequest::new() };
            unsafe { request.removeMember_fromGroup(&contact, &group) };
            crate::contact::execute_save(store, &request)
        })
        .await
    }
}

#[cfg(target_os = "macos")]
fn lookup_group(store: &CNContactStore, identifier: &str) -> ContactsResult<Retained<CNGroup>> {
    let ns_id = NSString::from_str(identifier);
    let ids = NSArray::from_slice(&[&*ns_id]);
    let predicate = unsafe { CNGroup::predicateForGroupsWithIdentifiers(&ids) };
    let groups = unsafe { store.groupsMatchingPredicate_error(Some(&predicate)) }
        .map_err(crate::error::map_cn_error)?;
    groups.iter().next().ok_or(ContactsError::NotFound)
}

#[cfg(target_os = "macos")]
fn mutable_group(group: &CNGroup) -> ContactsResult<Retained<CNMutableGroup>> {
    use objc2_foundation::NSMutableCopying;

    Ok(group.mutableCopy())
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn validate_group_name(name: &str) -> ContactsResult<()> {
    if name.trim().is_empty() {
        return Err(ContactsError::ValidationFailed(
            "group name cannot be empty".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_group_name;
    use crate::ContactsError;

    #[test]
    fn empty_group_name_is_invalid() {
        assert_eq!(
            validate_group_name("   ").unwrap_err(),
            ContactsError::ValidationFailed("group name cannot be empty".into())
        );
    }

    #[test]
    fn non_empty_group_name_is_valid() {
        assert!(validate_group_name("Friends").is_ok());
    }
}
