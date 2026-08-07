use std::{collections::HashMap, sync::Arc};

use sqlx::SqlitePool;

use super::{
    assembly::{
        ContactRelatedRows, contact_detail_from_row, contact_summary_from_row, container_from_row,
        group_from_row,
    },
    entities::{EntityIds, load_entity_ids},
    model::{ContactDetail, ContactGroup, ContactSummary, Container},
    queries::{
        AddressOwnedRow, EmailOwnedRow, GroupOwnedRow, PhoneOwnedRow, SocialOwnedRow, UrlOwnedRow,
        fetch_addresses_for_contact_ids, fetch_contact_by_api_id, fetch_contact_external_id,
        fetch_contact_photo, fetch_contacts_by_row_ids, fetch_container_by_api_id,
        fetch_container_resolve_metadata, fetch_containers, fetch_emails_for_contact_ids,
        fetch_filtered_contacts, fetch_group_by_api_id, fetch_group_contacts,
        fetch_group_external_id, fetch_group_ids_for_contact_ids, fetch_group_resolve_metadata,
        fetch_groups, fetch_phones_for_contact_ids, fetch_socials_for_contact_ids,
        fetch_urls_for_contact_ids,
    },
    row::api_id_from_unique_id,
    schema::{ContactsSchema, ParentGroupsJoin, discover_parent_groups_join},
    search::ContactFilters,
};
use crate::{
    api::cursor::{ContactListCursor, GroupContactCursor, encode},
    apple_types::SourceId,
    sqlx_util::json_ids,
};

#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ContainerResolveMetadata {
    pub api_id: String,
    /// Full AddressBook unique id (`UUID:ABContainer`) for CNContactStore.
    pub external_id: String,
    pub name: Option<String>,
    pub container_type: i64,
}

#[derive(Debug, Clone)]
pub struct GroupResolveMetadata {
    pub api_id: String,
    pub name: Option<String>,
    pub container_id: Option<i64>,
    pub group_type: i64,
}

pub struct ContactsRepository<'a> {
    pool: &'a SqlitePool,
    source_id: SourceId,
    entity_ids: Option<Arc<EntityIds>>,
    parent_groups: Option<Arc<ParentGroupsJoin>>,
}

impl<'a> ContactsRepository<'a> {
    pub fn new(pool: &'a SqlitePool, source_id: SourceId) -> Self {
        Self {
            pool,
            source_id,
            entity_ids: None,
            parent_groups: None,
        }
    }

    pub fn with_entity_ids(
        pool: &'a SqlitePool,
        source_id: SourceId,
        entity_ids: Arc<EntityIds>,
    ) -> Self {
        Self {
            pool,
            source_id,
            entity_ids: Some(entity_ids),
            parent_groups: None,
        }
    }

    pub fn with_schema(
        pool: &'a SqlitePool,
        source_id: SourceId,
        schema: Arc<ContactsSchema>,
    ) -> Self {
        Self {
            pool,
            source_id,
            entity_ids: Some(Arc::new(schema.entity_ids.clone())),
            parent_groups: Some(Arc::new(schema.parent_groups.clone())),
        }
    }

    async fn entity_ids(&self) -> Result<Arc<EntityIds>, sqlx::Error> {
        if let Some(entity_ids) = &self.entity_ids {
            return Ok(Arc::clone(entity_ids));
        }
        load_entity_ids(self.pool).await.map(Arc::new)
    }

    async fn parent_groups(&self) -> Result<Arc<ParentGroupsJoin>, sqlx::Error> {
        if let Some(parent_groups) = &self.parent_groups {
            return Ok(Arc::clone(parent_groups));
        }
        let entity_ids = self.entity_ids().await?;
        discover_parent_groups_join(self.pool, &entity_ids)
            .await
            .map(Arc::new)
    }

    pub fn source_id(&self) -> &SourceId {
        &self.source_id
    }

    pub async fn list_containers(&self) -> Result<Vec<Container>, sqlx::Error> {
        let entity_ids = self.entity_ids().await?;
        let rows = fetch_containers(self.pool, entity_ids.container).await?;
        Ok(rows
            .into_iter()
            .map(|row| container_from_row(row, self.source_id.clone()))
            .collect())
    }

    pub async fn get_container(
        &self,
        container_id: &str,
    ) -> Result<Option<Container>, sqlx::Error> {
        let entity_ids = self.entity_ids().await?;
        let row = fetch_container_by_api_id(self.pool, entity_ids.container, container_id).await?;
        Ok(row.map(|row| container_from_row(row, self.source_id.clone())))
    }

    pub async fn list_groups(
        &self,
        limit: u32,
        cursor: Option<ContactListCursor>,
    ) -> Result<Page<ContactGroup>, sqlx::Error> {
        let entity_ids = self.entity_ids().await?;
        let fetch_limit = i64::from(limit) + 1;
        let rows = fetch_groups(
            self.pool,
            entity_ids.group,
            cursor.map(|value| value.row_id),
            fetch_limit,
        )
        .await?;
        Ok(split_page_skipping(
            rows,
            limit,
            |row| {
                let group = group_from_row(row, self.source_id.clone());
                group.container_id.is_some().then_some(group)
            },
            |row| row.row_id,
        ))
    }

    pub async fn get_group(&self, group_id: &str) -> Result<Option<ContactGroup>, sqlx::Error> {
        let entity_ids = self.entity_ids().await?;
        let row = fetch_group_by_api_id(self.pool, entity_ids.group, group_id).await?;
        Ok(row.map(|row| group_from_row(row, self.source_id.clone())))
    }

    pub async fn list_contacts(
        &self,
        limit: u32,
        cursor: Option<ContactListCursor>,
        filters: &ContactFilters,
    ) -> Result<Page<ContactSummary>, sqlx::Error> {
        let entity_ids = self.entity_ids().await?;
        let parent_groups = self.parent_groups().await?;
        let fetch_limit = i64::from(limit) + 1;
        let binds = filters.bind_values(cursor.map(|value| value.row_id), fetch_limit);
        let rows =
            fetch_filtered_contacts(self.pool, entity_ids.contact, &parent_groups, &binds).await?;
        Ok(split_page_skipping(
            rows,
            limit,
            |row| {
                let summary = contact_summary_from_row(row, self.source_id.clone());
                summary.container_id.is_some().then_some(summary)
            },
            |row| row.row_id,
        ))
    }

    pub async fn list_group_contacts(
        &self,
        group_id: &str,
        limit: u32,
        cursor: Option<GroupContactCursor>,
    ) -> Result<Page<ContactSummary>, sqlx::Error> {
        let entity_ids = self.entity_ids().await?;
        let parent_groups = self.parent_groups().await?;
        let fetch_limit = i64::from(limit) + 1;
        let rows = fetch_group_contacts(
            self.pool,
            entity_ids.contact,
            &parent_groups,
            group_id,
            cursor.map(|value| value.row_id),
            fetch_limit,
        )
        .await?;
        Ok(split_page_skipping(
            rows,
            limit,
            |row| {
                let summary = contact_summary_from_row(row, self.source_id.clone());
                summary.container_id.is_some().then_some(summary)
            },
            |row| row.row_id,
        ))
    }

    pub async fn get_contact(
        &self,
        contact_id: &str,
    ) -> Result<Option<ContactDetail>, sqlx::Error> {
        let entity_ids = self.entity_ids().await?;
        let row = fetch_contact_by_api_id(self.pool, entity_ids.contact, contact_id).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(self.hydrate_contact(row).await?))
    }

    pub async fn get_contact_photo(
        &self,
        contact_id: &str,
    ) -> Result<Option<(Vec<u8>, Option<String>)>, sqlx::Error> {
        let entity_ids = self.entity_ids().await?;
        let row =
            fetch_contact_photo(self.pool, entity_ids.contact, &format!("{contact_id}:")).await?;
        Ok(row.and_then(|row| row.photo_data.map(|data| (data, row.image_type))))
    }

    pub async fn get_container_resolve_metadata(
        &self,
        container_id: &str,
    ) -> Result<Option<ContainerResolveMetadata>, sqlx::Error> {
        let entity_ids = self.entity_ids().await?;
        let api_id = api_id_from_unique_id(container_id);
        let row =
            fetch_container_resolve_metadata(self.pool, entity_ids.container, &api_id).await?;
        Ok(row.map(|row| ContainerResolveMetadata {
            api_id: row.api_id.unwrap_or(api_id),
            external_id: row.external_id,
            name: row.name,
            container_type: row.container_type.unwrap_or(0),
        }))
    }

    pub async fn get_group_resolve_metadata(
        &self,
        group_id: &str,
    ) -> Result<Option<GroupResolveMetadata>, sqlx::Error> {
        let entity_ids = self.entity_ids().await?;
        let api_id = api_id_from_unique_id(group_id);
        let row = fetch_group_resolve_metadata(self.pool, entity_ids.group, &api_id).await?;
        Ok(row.map(|row| GroupResolveMetadata {
            api_id: row.api_id.unwrap_or(api_id),
            name: row.name,
            group_type: row.group_type.unwrap_or(0),
            container_id: row.container_id,
        }))
    }

    pub async fn get_contact_external_id(
        &self,
        contact_id: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let entity_ids = self.entity_ids().await?;
        let api_id = api_id_from_unique_id(contact_id);
        fetch_contact_external_id(self.pool, entity_ids.contact, &api_id).await
    }

    pub async fn get_group_external_id(
        &self,
        group_id: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let entity_ids = self.entity_ids().await?;
        let api_id = api_id_from_unique_id(group_id);
        fetch_group_external_id(self.pool, entity_ids.group, &api_id).await
    }

    pub async fn hydrate_contacts_batch(
        &self,
        rows: Vec<super::row::ContactRow>,
    ) -> Result<Vec<ContactDetail>, sqlx::Error> {
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let entity_ids = self.entity_ids().await?;
        let parent_groups = self.parent_groups().await?;

        let row_ids: Vec<i64> = rows.iter().map(|row| row.row_id).collect();
        let fetched =
            fetch_contacts_by_row_ids(self.pool, entity_ids.contact, &json_ids(&row_ids)).await?;
        let mut by_row_id: std::collections::HashMap<i64, super::row::ContactRow> =
            fetched.into_iter().map(|row| (row.row_id, row)).collect();
        let rows: Vec<super::row::ContactRow> = row_ids
            .into_iter()
            .filter_map(|row_id| by_row_id.remove(&row_id))
            .collect();
        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let row_ids: Vec<i64> = rows.iter().map(|row| row.row_id).collect();
        let ids_json = json_ids(&row_ids);

        let phones = fetch_phones_for_contact_ids(self.pool, &ids_json).await?;
        let emails = fetch_emails_for_contact_ids(self.pool, &ids_json).await?;
        let addresses = fetch_addresses_for_contact_ids(self.pool, &ids_json).await?;
        let urls = fetch_urls_for_contact_ids(self.pool, &ids_json).await?;
        let socials = fetch_socials_for_contact_ids(self.pool, &ids_json).await?;
        let groups = fetch_group_ids_for_contact_ids(self.pool, &parent_groups, &ids_json).await?;

        let phones_by_owner = group_phones(phones);
        let emails_by_owner = group_emails(emails);
        let addresses_by_owner = group_addresses(addresses);
        let urls_by_owner = group_urls(urls);
        let socials_by_owner = group_socials(socials);
        let groups_by_owner = group_group_ids(groups);

        Ok(rows
            .into_iter()
            .map(|row| {
                let row_id = row.row_id;
                contact_detail_from_row(
                    row,
                    self.source_id.clone(),
                    ContactRelatedRows {
                        phones: phones_by_owner.get(&row_id).cloned().unwrap_or_default(),
                        emails: emails_by_owner.get(&row_id).cloned().unwrap_or_default(),
                        addresses: addresses_by_owner.get(&row_id).cloned().unwrap_or_default(),
                        urls: urls_by_owner.get(&row_id).cloned().unwrap_or_default(),
                        socials: socials_by_owner.get(&row_id).cloned().unwrap_or_default(),
                    },
                    groups_by_owner.get(&row_id).cloned().unwrap_or_default(),
                )
            })
            .collect())
    }

    async fn hydrate_contact(
        &self,
        row: super::row::ContactRow,
    ) -> Result<ContactDetail, sqlx::Error> {
        self.hydrate_contacts_batch(vec![row])
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }
}

fn group_phones(rows: Vec<PhoneOwnedRow>) -> HashMap<i64, Vec<super::row::PhoneRow>> {
    let mut map: HashMap<i64, Vec<super::row::PhoneRow>> = HashMap::new();
    for row in rows {
        map.entry(row.owner)
            .or_default()
            .push(super::row::PhoneRow {
                unique_id: row.unique_id,
                number: row.number,
                label: row.label,
                is_primary: row.is_primary,
                ordering_index: row.ordering_index,
            });
    }
    map
}

fn group_emails(rows: Vec<EmailOwnedRow>) -> HashMap<i64, Vec<super::row::EmailRow>> {
    let mut map: HashMap<i64, Vec<super::row::EmailRow>> = HashMap::new();
    for row in rows {
        map.entry(row.owner)
            .or_default()
            .push(super::row::EmailRow {
                unique_id: row.unique_id,
                address: row.address,
                label: row.label,
                is_primary: row.is_primary,
                ordering_index: row.ordering_index,
            });
    }
    map
}

fn group_addresses(rows: Vec<AddressOwnedRow>) -> HashMap<i64, Vec<super::row::AddressRow>> {
    let mut map: HashMap<i64, Vec<super::row::AddressRow>> = HashMap::new();
    for row in rows {
        map.entry(row.owner)
            .or_default()
            .push(super::row::AddressRow {
                unique_id: row.unique_id,
                street: row.street,
                city: row.city,
                state: row.state,
                postal_code: row.postal_code,
                country: row.country,
                label: row.label,
                is_primary: row.is_primary,
                ordering_index: row.ordering_index,
            });
    }
    map
}

fn group_urls(rows: Vec<UrlOwnedRow>) -> HashMap<i64, Vec<super::row::UrlRow>> {
    let mut map: HashMap<i64, Vec<super::row::UrlRow>> = HashMap::new();
    for row in rows {
        map.entry(row.owner).or_default().push(super::row::UrlRow {
            unique_id: row.unique_id,
            url: row.url,
            label: row.label,
            is_primary: row.is_primary,
            ordering_index: row.ordering_index,
        });
    }
    map
}

fn group_socials(rows: Vec<SocialOwnedRow>) -> HashMap<i64, Vec<super::row::SocialRow>> {
    let mut map: HashMap<i64, Vec<super::row::SocialRow>> = HashMap::new();
    for row in rows {
        map.entry(row.owner)
            .or_default()
            .push(super::row::SocialRow {
                unique_id: row.unique_id,
                service: row.service,
                username: row.username,
                url: row.url,
                label: row.label,
                is_primary: row.is_primary,
                ordering_index: row.ordering_index,
            });
    }
    map
}

fn group_group_ids(rows: Vec<GroupOwnedRow>) -> HashMap<i64, Vec<String>> {
    let mut map: HashMap<i64, Vec<String>> = HashMap::new();
    for row in rows {
        map.entry(row.owner).or_default().push(row.unique_id);
    }
    map
}

fn split_page_skipping<T, R, F, K>(rows: Vec<R>, limit: u32, map: F, row_id: K) -> Page<T>
where
    F: Fn(R) -> Option<T>,
    K: Fn(&R) -> i64,
{
    let has_more = rows.len() > limit as usize;
    let last_row_id = rows.get(limit.saturating_sub(1) as usize).map(row_id);
    let items: Vec<T> = rows
        .into_iter()
        .take(limit as usize)
        .filter_map(map)
        .collect();
    let next_cursor = if has_more {
        last_row_id.and_then(|id| encode(&ContactListCursor { row_id: id }).ok())
    } else {
        None
    };
    Page {
        items,
        has_more,
        next_cursor,
    }
}

#[cfg(test)]
mod tests {
    use super::ContactsRepository;
    use crate::{
        apple_types::SourceId,
        db::connect_pool,
        fixtures::{ContactsFixtureDb, SEED_CONTACT_ID, SEED_CONTAINER_ID, SEED_GROUP_ID},
    };

    #[tokio::test]
    async fn fixture_lists_containers_groups_and_contacts() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = ContactsFixtureDb::seeded().await?;
        let pool = connect_pool(fixture.path()).await?;
        let repo = ContactsRepository::new(&pool, SourceId::new("fixture-source"));

        let containers = repo.list_containers().await?;
        assert!(!containers.is_empty());
        assert!(
            containers
                .iter()
                .any(|c| c.id.as_str() == SEED_CONTAINER_ID)
        );

        let groups = repo.list_groups(50, None).await?;
        assert!(groups.items.iter().any(|g| g.id.as_str() == SEED_GROUP_ID));

        let contacts = repo.list_contacts(50, None, &Default::default()).await?;
        assert!(
            contacts
                .items
                .iter()
                .any(|c| c.id.as_str() == SEED_CONTACT_ID)
        );

        let detail = repo
            .get_contact(SEED_CONTACT_ID)
            .await?
            .ok_or("contact not found")?;
        assert_eq!(detail.first_name.as_deref(), Some("Jane"));
        assert_eq!(detail.phones.len(), 1);
        assert_eq!(detail.emails.len(), 1);
        Ok(())
    }
    #[tokio::test]
    async fn remapped_entity_fixture_lists_containers_groups_and_contacts()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = crate::fixtures::ContactsFixtureDb::seeded_with_remapped_entities().await?;
        let pool = connect_pool(fixture.path()).await?;
        let repo = ContactsRepository::new(&pool, SourceId::new("remapped-source"));

        let containers = repo.list_containers().await?;
        assert!(
            containers
                .iter()
                .any(|c| c.id.as_str() == SEED_CONTAINER_ID)
        );

        let groups = repo.list_groups(50, None).await?;
        assert!(groups.items.iter().any(|g| g.id.as_str() == SEED_GROUP_ID));

        let contacts = repo.list_contacts(50, None, &Default::default()).await?;
        assert!(
            contacts
                .items
                .iter()
                .any(|c| c.id.as_str() == SEED_CONTACT_ID)
        );

        let filtered = repo
            .list_contacts(
                50,
                None,
                &crate::contacts::ContactFilters {
                    group_id: Some(SEED_GROUP_ID.to_owned()),
                    ..Default::default()
                },
            )
            .await?;
        assert!(
            filtered
                .items
                .iter()
                .any(|c| c.id.as_str() == SEED_CONTACT_ID)
        );

        let detail = repo
            .get_contact(SEED_CONTACT_ID)
            .await?
            .ok_or("contact not found")?;
        assert_eq!(detail.first_name.as_deref(), Some("Jane"));
        assert!(detail.group_ids.iter().any(|g| g.as_str() == SEED_GROUP_ID));
        Ok(())
    }

    #[tokio::test]
    async fn hydrate_batch_uses_bounded_queries_for_large_page()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::db::query_budget;

        let fixture = ContactsFixtureDb::seeded_with_batch_contacts(200).await?;
        let pool = connect_pool(fixture.path()).await?;
        let repo = ContactsRepository::new(&pool, SourceId::new("fixture-source"));
        let entity_ids = crate::contacts::load_entity_ids(&pool).await?;
        let rows = crate::contacts::queries::fetch_contacts_by_row_ids(
            &pool,
            entity_ids.contact,
            &crate::sqlx_util::json_ids(&(100..300).collect::<Vec<_>>()),
        )
        .await?;
        assert_eq!(rows.len(), 200);

        query_budget::reset();
        let details = repo.hydrate_contacts_batch(rows).await?;
        assert_eq!(details.len(), 200);
        assert!(
            query_budget::get() == 6,
            "expected exactly 6 batch hydration queries, got {}",
            query_budget::get()
        );
        Ok(())
    }
}
