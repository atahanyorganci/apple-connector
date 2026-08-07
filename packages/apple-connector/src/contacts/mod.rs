//! Apple AddressBook SQLite access layer.

mod assembly;
mod discovery;
mod entities;
mod labels;
mod model;
mod queries;
mod repository;
mod row;
mod search;
mod sources;

pub use discovery::{
    DiscoveredSource, DiscoveryError, default_contacts_sources_dir, discover_contacts_sources,
};
pub use entities::{EntityIdError, EntityIds, load_entity_ids};
pub use model::{
    Contact, ContactAddress, ContactDetail, ContactEmail, ContactGroup, ContactPhone,
    ContactSocialProfile, ContactSummary, ContactUrl, Container, ContainerDetail, ContainerSummary,
};
pub use repository::{ContactsRepository, ContainerResolveMetadata, GroupResolveMetadata, Page};
pub use row::api_id_from_unique_id;
pub use search::ContactFilters;
pub use sources::ContactsSources;
