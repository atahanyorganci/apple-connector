//! Contacts framework integration for contact and group mutations on macOS.
//!
//! All Objective-C / `unsafe` code is confined to this crate.

#[cfg(not(target_os = "macos"))]
compile_error!(
    "apple-contacts links against macOS-only Apple frameworks; it cannot be built for other targets"
);

mod auth;
mod contact;
mod container;
mod error;
mod group;
mod store;
mod worker;

pub use auth::{AuthOutcome, AuthStatus};
pub use contact::{
    CreateContactInput, LabeledStringInput, PostalAddressInput, SavedContact, UpdateContactInput,
};
pub use container::{ContainerResolveHint, ContainerResolveMetadata, ContainerStoreType};
pub use error::{ContactsError, ContactsResult};
pub use group::{CreateGroupInput, SavedGroup, UpdateGroupInput};
pub use store::ContactsStore;
