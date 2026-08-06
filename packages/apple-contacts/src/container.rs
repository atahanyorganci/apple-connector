use crate::error::{ContactsError, ContactsResult};

#[derive(Debug, Clone)]
pub struct ContainerResolveHint {
    pub api_id: String,
    pub external_id: Option<String>,
    pub name: Option<String>,
    pub read_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerStoreType {
    Unassigned,
    Local,
    Exchange,
    CardDav,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerResolveMetadata {
    pub identifier: String,
    pub name: String,
    pub container_type: ContainerStoreType,
}

#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2_contacts::{CNContactStore, CNContainer, CNContainerType};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSArray, NSString};

#[cfg(target_os = "macos")]
pub(crate) fn resolve_container(
    store: &CNContactStore,
    hint: &ContainerResolveHint,
) -> ContactsResult<(Retained<CNContainer>, ContainerResolveMetadata)> {
    if hint.read_only {
        return Err(ContactsError::ReadOnlyContainer);
    }

    if let Some(container) = lookup_container(store, &hint.external_id, &hint.api_id) {
        return Ok((container.clone(), container_metadata(&container)));
    }

    if let Some(title) = &hint.name {
        let containers = unsafe { store.containersMatchingPredicate_error(None) }
            .map_err(crate::error::map_cn_error)?;
        let lowered = title.to_ascii_lowercase();
        let mut matches = containers
            .iter()
            .filter(|container| unsafe {
                container.name().to_string().to_ascii_lowercase() == lowered
            })
            .collect::<Vec<_>>();

        if matches.len() == 1 {
            let container = matches.remove(0).clone();
            return Ok((container.clone(), container_metadata(&container)));
        }
        if matches.len() > 1 {
            return Err(ContactsError::ValidationFailed(format!(
                "multiple containers named '{title}'"
            )));
        }
    }

    Err(ContactsError::NotFound)
}

#[cfg(target_os = "macos")]
fn lookup_container(
    store: &CNContactStore,
    external_id: &Option<String>,
    api_id: &str,
) -> Option<Retained<CNContainer>> {
    let mut candidates = Vec::new();
    if let Some(external_id) = external_id {
        candidates.push(external_id.as_str());
    }
    candidates.push(api_id);

    for candidate in candidates {
        let ns_id = NSString::from_str(candidate);
        let ids = NSArray::from_slice(&[&*ns_id]);
        let predicate = unsafe { CNContainer::predicateForContainersWithIdentifiers(&ids) };
        let containers =
            unsafe { store.containersMatchingPredicate_error(Some(&predicate)) }.ok()?;
        if let Some(container) = containers.iter().next() {
            return Some(container.clone());
        }
    }

    None
}

#[cfg(target_os = "macos")]
fn container_metadata(container: &CNContainer) -> ContainerResolveMetadata {
    let container_type = match unsafe { container.r#type() } {
        CNContainerType::Local => ContainerStoreType::Local,
        CNContainerType::Exchange => ContainerStoreType::Exchange,
        CNContainerType::CardDAV => ContainerStoreType::CardDav,
        _ => ContainerStoreType::Unassigned,
    };

    ContainerResolveMetadata {
        identifier: unsafe { container.identifier().to_string() },
        name: unsafe { container.name().to_string() },
        container_type,
    }
}

#[cfg(test)]
mod tests {
    use super::{ContainerResolveHint, ContainerStoreType};

    #[test]
    fn read_only_hint_is_detectable() {
        let hint = ContainerResolveHint {
            api_id: "abc".into(),
            external_id: None,
            name: Some("Work".into()),
            read_only: true,
        };
        assert!(hint.read_only);
    }

    #[test]
    fn local_store_type_is_distinct() {
        assert!(matches!(
            ContainerStoreType::Local,
            ContainerStoreType::Local
        ));
    }
}
