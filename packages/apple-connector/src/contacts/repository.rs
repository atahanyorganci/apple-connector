use sqlx::SqlitePool;

use super::{
    assembly::{
        ContactRelatedRows, contact_detail_from_row, contact_summary_from_row, container_from_row,
        group_from_row,
    },
    model::{ContactDetail, ContactGroup, ContactSummary, Container},
    queries::{
        fetch_addresses_for_contact, fetch_contact_by_api_id, fetch_contact_external_id,
        fetch_contact_photo, fetch_container_by_api_id, fetch_container_resolve_metadata,
        fetch_containers, fetch_emails_for_contact, fetch_filtered_contacts, fetch_group_by_api_id,
        fetch_group_contacts, fetch_group_external_id, fetch_group_ids_for_contact,
        fetch_group_resolve_metadata, fetch_groups, fetch_phones_for_contact,
        fetch_socials_for_contact, fetch_urls_for_contact,
    },
    row::api_id_from_unique_id,
    search::ContactFilters,
};
use crate::{
    api::cursor::{ContactListCursor, GroupContactCursor, encode},
    apple_types::SourceId,
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
}

impl<'a> ContactsRepository<'a> {
    pub fn new(pool: &'a SqlitePool, source_id: SourceId) -> Self {
        Self { pool, source_id }
    }

    pub fn source_id(&self) -> &SourceId {
        &self.source_id
    }

    pub async fn list_containers(&self) -> Result<Vec<Container>, sqlx::Error> {
        let rows = fetch_containers(self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|row| container_from_row(row, self.source_id.clone()))
            .collect())
    }

    pub async fn get_container(
        &self,
        container_id: &str,
    ) -> Result<Option<Container>, sqlx::Error> {
        let row = fetch_container_by_api_id(self.pool, container_id).await?;
        Ok(row.map(|row| container_from_row(row, self.source_id.clone())))
    }

    pub async fn list_groups(
        &self,
        limit: u32,
        cursor: Option<ContactListCursor>,
    ) -> Result<Page<ContactGroup>, sqlx::Error> {
        let fetch_limit = i64::from(limit) + 1;
        let rows = fetch_groups(self.pool, cursor.map(|value| value.row_id), fetch_limit).await?;
        Ok(split_page(
            rows,
            limit,
            |row| group_from_row(row, self.source_id.clone()),
            |row| row.row_id,
        ))
    }

    pub async fn get_group(&self, group_id: &str) -> Result<Option<ContactGroup>, sqlx::Error> {
        let row = fetch_group_by_api_id(self.pool, group_id).await?;
        Ok(row.map(|row| group_from_row(row, self.source_id.clone())))
    }

    pub async fn list_contacts(
        &self,
        limit: u32,
        cursor: Option<ContactListCursor>,
        filters: &ContactFilters,
    ) -> Result<Page<ContactSummary>, sqlx::Error> {
        let fetch_limit = i64::from(limit) + 1;
        let binds = filters.bind_values(cursor.map(|value| value.row_id), fetch_limit);
        let rows = fetch_filtered_contacts(self.pool, &binds).await?;
        Ok(split_page(
            rows,
            limit,
            |row| contact_summary_from_row(row, self.source_id.clone()),
            |row| row.row_id,
        ))
    }

    pub async fn list_group_contacts(
        &self,
        group_id: &str,
        limit: u32,
        cursor: Option<GroupContactCursor>,
    ) -> Result<Page<ContactSummary>, sqlx::Error> {
        let fetch_limit = i64::from(limit) + 1;
        let rows = fetch_group_contacts(
            self.pool,
            group_id,
            cursor.map(|value| value.row_id),
            fetch_limit,
        )
        .await?;
        Ok(split_page(
            rows,
            limit,
            |row| contact_summary_from_row(row, self.source_id.clone()),
            |row| row.row_id,
        ))
    }

    pub async fn get_contact(
        &self,
        contact_id: &str,
    ) -> Result<Option<ContactDetail>, sqlx::Error> {
        let row = fetch_contact_by_api_id(self.pool, contact_id).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(self.hydrate_contact(row).await?))
    }

    pub async fn get_contact_photo(
        &self,
        contact_id: &str,
    ) -> Result<Option<(Vec<u8>, Option<String>)>, sqlx::Error> {
        let row = fetch_contact_photo(self.pool, &format!("{contact_id}:")).await?;
        Ok(row.and_then(|row| row.photo_data.map(|data| (data, row.image_type))))
    }

    pub async fn get_container_resolve_metadata(
        &self,
        container_id: &str,
    ) -> Result<Option<ContainerResolveMetadata>, sqlx::Error> {
        let api_id = api_id_from_unique_id(container_id);
        let row = fetch_container_resolve_metadata(self.pool, &api_id).await?;
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
        let api_id = api_id_from_unique_id(group_id);
        let row = fetch_group_resolve_metadata(self.pool, &api_id).await?;
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
        let api_id = api_id_from_unique_id(contact_id);
        fetch_contact_external_id(self.pool, &api_id).await
    }

    pub async fn get_group_external_id(
        &self,
        group_id: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let api_id = api_id_from_unique_id(group_id);
        fetch_group_external_id(self.pool, &api_id).await
    }

    async fn hydrate_contact(
        &self,
        row: super::row::ContactRow,
    ) -> Result<ContactDetail, sqlx::Error> {
        let row_id = row.row_id;
        let phones = fetch_phones_for_contact(self.pool, row_id).await?;
        let emails = fetch_emails_for_contact(self.pool, row_id).await?;
        let addresses = fetch_addresses_for_contact(self.pool, row_id).await?;
        let urls = fetch_urls_for_contact(self.pool, row_id).await?;
        let socials = fetch_socials_for_contact(self.pool, row_id).await?;
        let groups = fetch_group_ids_for_contact(self.pool, row_id).await?;
        Ok(contact_detail_from_row(
            row,
            self.source_id.clone(),
            ContactRelatedRows {
                phones,
                emails,
                addresses,
                urls,
                socials,
            },
            groups.into_iter().map(|group| group.unique_id).collect(),
        ))
    }
}

fn split_page<T, R, F, K>(rows: Vec<R>, limit: u32, map: F, row_id: K) -> Page<T>
where
    F: Fn(R) -> T,
    K: Fn(&R) -> i64,
{
    let has_more = rows.len() > limit as usize;
    let last_row_id = rows.get(limit.saturating_sub(1) as usize).map(row_id);
    let items: Vec<T> = rows.into_iter().take(limit as usize).map(map).collect();
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
}
