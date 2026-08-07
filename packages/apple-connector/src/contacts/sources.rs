use std::{collections::HashMap, sync::Arc};

use sqlx::SqlitePool;
use tokio::sync::OnceCell;

use super::{
    model::{ContactDetail, ContactGroup, ContactSummary, Container},
    repository::{ContactsRepository, Page},
    schema::{ContactsSchema, load_contacts_schema},
    search::ContactFilters,
};
use crate::{
    api::cursor::{ContactListCursor, GroupContactCursor},
    apple_types::SourceId,
};

#[derive(Debug, Clone)]
pub struct ContactsSources {
    pools: HashMap<SourceId, SqlitePool>,
    schemas: HashMap<SourceId, Arc<OnceCell<Arc<ContactsSchema>>>>,
}

impl ContactsSources {
    pub fn new(pools: HashMap<SourceId, SqlitePool>) -> Self {
        let schemas = pools
            .keys()
            .map(|source_id| (source_id.clone(), Arc::new(OnceCell::new())))
            .collect();
        Self { pools, schemas }
    }

    pub fn is_empty(&self) -> bool {
        self.pools.is_empty()
    }

    /// Resolve (and cache) the verified Contacts schema for a source. Each
    /// source has its own cache since Core Data entity/join names can differ
    /// per AddressBook store.
    pub async fn cached_schema(
        &self,
        source_id: &SourceId,
    ) -> Result<Arc<ContactsSchema>, sqlx::Error> {
        let pool = self.pools.get(source_id).ok_or_else(|| {
            sqlx::Error::Configuration(format!("unknown Contacts source: {source_id}").into())
        })?;
        let cell = self.schemas.get(source_id).ok_or_else(|| {
            sqlx::Error::Configuration(format!("unknown Contacts source: {source_id}").into())
        })?;
        cell.get_or_try_init(|| async { load_contacts_schema(pool).await.map(Arc::new) })
            .await
            .map(Arc::clone)
    }

    /// Eagerly resolve and cache the schema for every configured source.
    /// Used at startup so misconfigured stores fail fast.
    pub async fn warm_schemas(&self) -> Result<(), sqlx::Error> {
        for source_id in self.pools.keys() {
            self.cached_schema(source_id).await?;
        }
        Ok(())
    }

    async fn repository_for<'a>(
        &self,
        source_id: &SourceId,
        pool: &'a SqlitePool,
    ) -> Result<ContactsRepository<'a>, sqlx::Error> {
        let schema = self.cached_schema(source_id).await?;
        Ok(ContactsRepository::with_schema(
            pool,
            source_id.clone(),
            schema,
        ))
    }

    pub fn source_ids(&self) -> impl Iterator<Item = &SourceId> {
        self.pools.keys()
    }

    pub fn pool_for_contact(&self, contact: &ContactSummary) -> Option<&SqlitePool> {
        self.pools.get(&contact.source_id)
    }

    pub fn pool_for_source(&self, source_id: &SourceId) -> Option<&SqlitePool> {
        self.pools.get(source_id)
    }

    pub fn first_pool(&self) -> Option<(&SourceId, &SqlitePool)> {
        self.pools.iter().next()
    }

    pub fn pools(&self) -> impl Iterator<Item = &SqlitePool> {
        self.pools.values()
    }

    pub async fn list_containers(&self) -> Result<Vec<Container>, sqlx::Error> {
        crate::db::run_timed_query(|| self.list_containers_inner()).await
    }

    async fn list_containers_inner(&self) -> Result<Vec<Container>, sqlx::Error> {
        let mut all = Vec::new();
        for (source_id, pool) in &self.pools {
            let repo = self.repository_for(source_id, pool).await?;
            all.extend(repo.list_containers().await?);
        }
        all.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
        Ok(all)
    }

    pub async fn get_container(
        &self,
        container_id: &str,
    ) -> Result<Option<Container>, sqlx::Error> {
        crate::db::run_timed_query(|| self.get_container_inner(container_id)).await
    }

    async fn get_container_inner(
        &self,
        container_id: &str,
    ) -> Result<Option<Container>, sqlx::Error> {
        for (source_id, pool) in &self.pools {
            let repo = self.repository_for(source_id, pool).await?;
            if let Some(container) = repo.get_container(container_id).await? {
                return Ok(Some(container));
            }
        }
        Ok(None)
    }

    pub async fn list_groups(
        &self,
        limit: u32,
        cursor: Option<ContactListCursor>,
    ) -> Result<Page<ContactGroup>, sqlx::Error> {
        crate::db::run_timed_query(|| self.list_groups_inner(limit, cursor)).await
    }

    async fn list_groups_inner(
        &self,
        limit: u32,
        _cursor: Option<ContactListCursor>,
    ) -> Result<Page<ContactGroup>, sqlx::Error> {
        let mut merged = Vec::new();
        let per_source = limit.saturating_add(1);
        for (source_id, pool) in &self.pools {
            let repo = self.repository_for(source_id, pool).await?;
            let page = repo.list_groups(per_source, None).await?;
            merged.extend(page.items);
        }
        merged.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
        let has_more = merged.len() > limit as usize;
        let items = merged.into_iter().take(limit as usize).collect();
        Ok(Page {
            items,
            has_more,
            next_cursor: None,
        })
    }

    pub async fn get_group(&self, group_id: &str) -> Result<Option<ContactGroup>, sqlx::Error> {
        crate::db::run_timed_query(|| self.get_group_inner(group_id)).await
    }

    async fn get_group_inner(&self, group_id: &str) -> Result<Option<ContactGroup>, sqlx::Error> {
        for (source_id, pool) in &self.pools {
            let repo = self.repository_for(source_id, pool).await?;
            if let Some(group) = repo.get_group(group_id).await? {
                return Ok(Some(group));
            }
        }
        Ok(None)
    }

    pub async fn list_contacts(
        &self,
        limit: u32,
        cursor: Option<ContactListCursor>,
        filters: &ContactFilters,
    ) -> Result<Page<ContactSummary>, sqlx::Error> {
        crate::db::run_timed_query(|| self.list_contacts_inner(limit, cursor, filters)).await
    }

    async fn list_contacts_inner(
        &self,
        limit: u32,
        _cursor: Option<ContactListCursor>,
        filters: &ContactFilters,
    ) -> Result<Page<ContactSummary>, sqlx::Error> {
        let mut merged = Vec::new();
        let per_source = limit.saturating_add(1);
        for (source_id, pool) in &self.pools {
            let repo = self.repository_for(source_id, pool).await?;
            let page = repo.list_contacts(per_source, None, filters).await?;
            merged.extend(page.items);
        }
        merged.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
        let has_more = merged.len() > limit as usize;
        let items = merged.into_iter().take(limit as usize).collect();
        Ok(Page {
            items,
            has_more,
            next_cursor: None,
        })
    }

    pub async fn list_group_contacts(
        &self,
        group_id: &str,
        limit: u32,
        cursor: Option<GroupContactCursor>,
    ) -> Result<Page<ContactSummary>, sqlx::Error> {
        crate::db::run_timed_query(|| self.list_group_contacts_inner(group_id, limit, cursor)).await
    }

    async fn list_group_contacts_inner(
        &self,
        group_id: &str,
        limit: u32,
        cursor: Option<GroupContactCursor>,
    ) -> Result<Page<ContactSummary>, sqlx::Error> {
        for (source_id, pool) in &self.pools {
            let repo = self.repository_for(source_id, pool).await?;
            if repo.get_group(group_id).await?.is_some() {
                return repo.list_group_contacts(group_id, limit, cursor).await;
            }
        }
        Ok(Page {
            items: Vec::new(),
            has_more: false,
            next_cursor: None,
        })
    }

    pub async fn get_contact(
        &self,
        contact_id: &str,
    ) -> Result<Option<ContactDetail>, sqlx::Error> {
        crate::db::run_timed_query(|| self.get_contact_inner(contact_id)).await
    }

    async fn get_contact_inner(
        &self,
        contact_id: &str,
    ) -> Result<Option<ContactDetail>, sqlx::Error> {
        for (source_id, pool) in &self.pools {
            let repo = self.repository_for(source_id, pool).await?;
            if let Some(contact) = repo.get_contact(contact_id).await? {
                return Ok(Some(contact));
            }
        }
        Ok(None)
    }

    pub async fn hydrate_contact_summaries(
        &self,
        summaries: Vec<ContactSummary>,
    ) -> Result<Vec<ContactDetail>, sqlx::Error> {
        crate::db::run_timed_query(|| self.hydrate_contact_summaries_inner(summaries)).await
    }

    async fn hydrate_contact_summaries_inner(
        &self,
        summaries: Vec<ContactSummary>,
    ) -> Result<Vec<ContactDetail>, sqlx::Error> {
        use std::collections::HashMap;

        use super::queries::fetch_contacts_by_api_ids;
        use crate::{contacts::row::api_id_from_unique_id, sqlx_util::json_strings};

        if summaries.is_empty() {
            return Ok(Vec::new());
        }

        let ordered_summaries = summaries;
        let mut by_source: HashMap<SourceId, Vec<ContactSummary>> = HashMap::new();
        for summary in &ordered_summaries {
            by_source
                .entry(summary.source_id.clone())
                .or_default()
                .push(summary.clone());
        }

        let mut details_by_id: HashMap<String, ContactDetail> = HashMap::new();
        for (source_id, group) in by_source {
            let Some(pool) = self.pools.get(&source_id) else {
                continue;
            };
            let schema = self.cached_schema(&source_id).await?;
            let repo =
                ContactsRepository::with_schema(pool, source_id.clone(), Arc::clone(&schema));
            let entity_ids = &schema.entity_ids;
            let api_ids: Vec<&str> = group.iter().map(|summary| summary.id.as_str()).collect();
            let rows = fetch_contacts_by_api_ids(pool, entity_ids.contact, &json_strings(&api_ids))
                .await?;
            let mut rows_by_api_id: HashMap<String, super::row::ContactRow> = HashMap::new();
            for row in rows {
                rows_by_api_id.insert(api_id_from_unique_id(&row.unique_id), row);
            }
            let ordered_rows: Vec<super::row::ContactRow> = api_ids
                .into_iter()
                .filter_map(|id| rows_by_api_id.remove(id))
                .collect();
            for detail in repo.hydrate_contacts_batch(ordered_rows).await? {
                details_by_id.insert(detail.id.as_str().to_owned(), detail);
            }
        }

        ordered_summaries
            .into_iter()
            .map(|summary| {
                details_by_id
                    .remove(summary.id.as_str())
                    .ok_or(sqlx::Error::RowNotFound)
            })
            .collect()
    }

    pub async fn get_contact_photo(
        &self,
        contact_id: &str,
    ) -> Result<Option<(Vec<u8>, Option<String>)>, sqlx::Error> {
        crate::db::run_timed_query(|| self.get_contact_photo_inner(contact_id)).await
    }

    async fn get_contact_photo_inner(
        &self,
        contact_id: &str,
    ) -> Result<Option<(Vec<u8>, Option<String>)>, sqlx::Error> {
        for (source_id, pool) in &self.pools {
            let repo = self.repository_for(source_id, pool).await?;
            if let Some(photo) = repo.get_contact_photo(contact_id).await? {
                return Ok(Some(photo));
            }
        }
        Ok(None)
    }

    pub async fn find_pool_for_contact_id(
        &self,
        contact_id: &str,
    ) -> Result<Option<(&SourceId, &SqlitePool)>, sqlx::Error> {
        for (source_id, pool) in &self.pools {
            let repo = self.repository_for(source_id, pool).await?;
            if repo.get_contact_external_id(contact_id).await?.is_some() {
                return Ok(Some((source_id, pool)));
            }
        }
        Ok(None)
    }

    /// Resolve the CNContactStore identifier (`UUID:ABPerson`) for an API contact id.
    pub async fn get_contact_framework_id(
        &self,
        contact_id: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        for (source_id, pool) in &self.pools {
            let repo = self.repository_for(source_id, pool).await?;
            if let Some(external_id) = repo.get_contact_external_id(contact_id).await? {
                return Ok(Some(external_id));
            }
        }
        Ok(None)
    }

    /// Resolve the CNContactStore identifier (`UUID:ABGroup`) for an API group id.
    pub async fn get_group_framework_id(
        &self,
        group_id: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        for (source_id, pool) in &self.pools {
            let repo = self.repository_for(source_id, pool).await?;
            if let Some(external_id) = repo.get_group_external_id(group_id).await? {
                return Ok(Some(external_id));
            }
        }
        Ok(None)
    }

    pub async fn get_container_resolve_metadata(
        &self,
        container_id: &str,
    ) -> Result<Option<super::repository::ContainerResolveMetadata>, sqlx::Error> {
        for (source_id, pool) in &self.pools {
            let repo = self.repository_for(source_id, pool).await?;
            if let Some(meta) = repo.get_container_resolve_metadata(container_id).await? {
                return Ok(Some(meta));
            }
        }
        Ok(None)
    }

    pub async fn get_group_resolve_metadata(
        &self,
        group_id: &str,
    ) -> Result<Option<super::repository::GroupResolveMetadata>, sqlx::Error> {
        for (source_id, pool) in &self.pools {
            let repo = self.repository_for(source_id, pool).await?;
            if let Some(meta) = repo.get_group_resolve_metadata(group_id).await? {
                return Ok(Some(meta));
            }
        }
        Ok(None)
    }
}

// Specialized sort for ContactSummary
impl ContactsSources {
    pub async fn search_contacts(
        &self,
        q: &str,
        limit: u32,
    ) -> Result<Vec<ContactSummary>, sqlx::Error> {
        crate::db::run_timed_query(|| self.search_contacts_inner(q, limit)).await
    }

    async fn search_contacts_inner(
        &self,
        q: &str,
        limit: u32,
    ) -> Result<Vec<ContactSummary>, sqlx::Error> {
        let filters = ContactFilters {
            q: Some(q.to_owned()),
            ..ContactFilters::default()
        };
        let page = self.list_contacts(limit, None, &filters).await?;
        Ok(page.items)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::ContactsSources;
    use crate::{apple_types::SourceId, db::connect_pool, fixtures::ContactsFixtureDb};

    #[tokio::test]
    async fn caches_schema_per_source_and_warms_all_sources()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = ContactsFixtureDb::seeded().await?;
        let pool = connect_pool(fixture.path()).await?;
        let source_id = SourceId::new("fixture-source");
        let sources = ContactsSources::new(HashMap::from([(source_id.clone(), pool)]));

        sources.warm_schemas().await?;

        let schema = sources.cached_schema(&source_id).await?;
        assert_eq!(schema.entity_ids.contact, 22);
        assert_eq!(schema.parent_groups.table, "Z_22PARENTGROUPS");

        let schema_again = sources.cached_schema(&source_id).await?;
        assert!(std::sync::Arc::ptr_eq(&schema, &schema_again));
        Ok(())
    }

    #[tokio::test]
    async fn warm_schemas_fails_explicitly_for_misconfigured_source()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = ContactsFixtureDb::seeded_without_parent_groups_join().await?;
        let pool = connect_pool(fixture.path()).await?;
        let source_id = SourceId::new("broken-source");
        let sources = ContactsSources::new(HashMap::from([(source_id, pool)]));

        let error = sources
            .warm_schemas()
            .await
            .err()
            .ok_or("expected warm_schemas to fail for misconfigured source")?;
        assert!(
            error
                .to_string()
                .contains("missing Contacts parentGroups join table"),
            "unexpected error: {error}"
        );
        Ok(())
    }
}
