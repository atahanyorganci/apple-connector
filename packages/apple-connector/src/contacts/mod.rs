//! Apple AddressBook SQLite access layer.

mod assembly;
mod discovery;
mod labels;
mod model;
mod repository;
mod row;
mod search;
mod sources;
mod sql;

pub use discovery::{
    DiscoveryError, default_contacts_sources_dir, discover_contacts_sources, DiscoveredSource,
};
pub use model::{
    Contact, ContactAddress, ContactDetail, ContactEmail, ContactGroup, ContactPhone,
    ContactSocialProfile, ContactSummary, ContactUrl, Container, ContainerDetail,
    ContainerSummary,
};
pub use repository::{
    ContactsRepository, ContainerResolveMetadata, GroupResolveMetadata, Page,
};
pub use row::api_id_from_unique_id;
pub use search::ContactFilters;
pub use sources::ContactsSources;
