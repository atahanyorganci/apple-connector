//! Contacts framework integration for contact and group mutations on macOS.
//!
//! All Objective-C / `unsafe` code is confined to this crate.

mod auth;
mod contact;
mod container;
mod error;
mod group;
mod store;

pub use auth::AuthStatus;
pub use contact::{
    CreateContactInput, LabeledStringInput, PostalAddressInput, SavedContact, UpdateContactInput,
};
pub use container::{ContainerResolveHint, ContainerResolveMetadata, ContainerStoreType};
pub use error::{ContactsError, ContactsResult};
pub use group::{CreateGroupInput, SavedGroup, UpdateGroupInput};
pub use store::ContactsStore;
