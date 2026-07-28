//! Contacts framework integration for contact and group mutations on macOS.
//!
//! All Objective-C / `unsafe` code is confined to this crate.

mod contact;
mod container;
mod error;
mod group;

#[cfg(target_os = "macos")]
mod auth;
#[cfg(target_os = "macos")]
mod store;

pub use contact::{
    CreateContactInput, LabeledStringInput, PostalAddressInput, SavedContact, UpdateContactInput,
};
pub use container::{ContainerResolveHint, ContainerResolveMetadata, ContainerStoreType};
pub use error::{ContactsError, ContactsResult};
pub use group::{CreateGroupInput, SavedGroup, UpdateGroupInput};
#[cfg(target_os = "macos")]
pub use auth::AuthStatus;
#[cfg(target_os = "macos")]
pub use store::ContactsStore;

#[cfg(not(target_os = "macos"))]
mod stub;

#[cfg(not(target_os = "macos"))]
pub use stub::*;
