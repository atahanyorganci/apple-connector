use objc2::rc::Retained;
use objc2_contacts::{CNContactStore, CNContainer, CNContainerType};
use objc2_foundation::{NSArray, NSString};

use crate::error::{ContactsError, ContactsResult, map_cn_error};

/// What the SQLite read path knows about a container, used to find it in the framework.
///
/// The hint deliberately carries no writability flag. The SQLite row cannot know whether the
/// framework will accept a write, and a hint that claims otherwise is stale by construction — the
/// Contacts framework is the only authority, and it answers at save time with
/// `CNErrorCodeParentContainerNotWritable` and friends.
#[derive(Debug, Clone)]
pub struct ContainerResolveHint {
    pub api_id: String,
    pub external_id: Option<String>,
    pub name: Option<String>,
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

pub(crate) fn resolve_container(
    store: &CNContactStore,
    hint: &ContainerResolveHint,
) -> ContactsResult<(Retained<CNContainer>, ContainerResolveMetadata)> {
    let mut identifiers = Vec::new();
    if let Some(external_id) = &hint.external_id {
        identifiers.push(external_id.as_str());
    }
    identifiers.push(hint.api_id.as_str());

    for identifier in identifiers {
        if let Some(container) = container_with_identifier(store, identifier)? {
            // The predicate filters by identifier, so a container that comes back under a
            // different one means the stored row no longer names anything real. Writing to
            // whatever the framework returned instead would silently target another account.
            if unsafe { container.identifier().to_string() } != identifier {
                return Err(ContactsError::NotFound);
            }
            let metadata = container_metadata(&container);
            return Ok((container, metadata));
        }
    }

    if let Some(name) = &hint.name {
        return container_with_name(store, name);
    }

    Err(ContactsError::NotFound)
}

fn container_with_identifier(
    store: &CNContactStore,
    identifier: &str,
) -> ContactsResult<Option<Retained<CNContainer>>> {
    let ns_id = NSString::from_str(identifier);
    let ids = NSArray::from_slice(&[&*ns_id]);
    let predicate = unsafe { CNContainer::predicateForContainersWithIdentifiers(&ids) };

    let containers = match unsafe { store.containersMatchingPredicate_error(Some(&predicate)) } {
        Ok(containers) => containers,
        Err(error) => {
            // "No container with this identifier" is a miss, not a failure — fall through to the
            // next candidate and then to name resolution. Anything else is a real framework
            // error and must not be mistaken for an absent container.
            return match map_cn_error(error) {
                ContactsError::NotFound => Ok(None),
                other => Err(other),
            };
        }
    };

    let mut matches = containers.iter();
    let Some(container) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        return Err(ContactsError::AmbiguousMatch(format!(
            "multiple containers share identifier '{identifier}'"
        )));
    }
    Ok(Some(container))
}

fn container_with_name(
    store: &CNContactStore,
    name: &str,
) -> ContactsResult<(Retained<CNContainer>, ContainerResolveMetadata)> {
    let containers =
        unsafe { store.containersMatchingPredicate_error(None) }.map_err(map_cn_error)?;
    let wanted = name.to_ascii_lowercase();
    let mut matches = containers
        .iter()
        .filter(|container| unsafe { container.name().to_string().to_ascii_lowercase() } == wanted);

    let Some(container) = matches.next() else {
        return Err(ContactsError::NotFound);
    };
    if matches.next().is_some() {
        return Err(ContactsError::AmbiguousMatch(format!(
            "multiple containers named '{name}'"
        )));
    }
    let metadata = container_metadata(&container);
    Ok((container, metadata))
}

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

    /// The hint carries only identity, never a writability claim: everything here is something
    /// the SQLite read path can actually know.
    #[test]
    fn a_hint_carries_only_identity() {
        let hint = ContainerResolveHint {
            api_id: "abc".into(),
            external_id: None,
            name: Some("Work".into()),
        };
        assert_eq!(hint.api_id, "abc");
        assert_eq!(hint.name.as_deref(), Some("Work"));
    }

    #[test]
    fn local_store_type_is_distinct() {
        assert!(matches!(
            ContainerStoreType::Local,
            ContainerStoreType::Local
        ));
    }
}
