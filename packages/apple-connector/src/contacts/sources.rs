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

    /// Stable global ordering of sources for cross-source pagination:
    /// sources are consumed one at a time, ascending by `source_id`.
    fn sorted_source_ids(&self) -> Vec<SourceId> {
        let mut ids: Vec<SourceId> = self.pools.keys().cloned().collect();
        ids.sort();
        ids
    }

    fn pool_for_source_or_err(&self, source_id: &SourceId) -> Result<&SqlitePool, sqlx::Error> {
        self.pools.get(source_id).ok_or_else(|| {
            sqlx::Error::Configuration(format!("unknown Contacts source: {source_id}").into())
        })
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
        cursor: Option<ContactListCursor>,
    ) -> Result<Page<ContactGroup>, sqlx::Error> {
        let source_ids = self.sorted_source_ids();
        if source_ids.is_empty() {
            return Ok(empty_page());
        }
        let (mut idx, mut pending_cursor) = resolve_cursor_start(&source_ids, cursor)?;

        let mut items = Vec::new();
        while idx < source_ids.len() && items.len() < limit as usize {
            let source_id = &source_ids[idx];
            let pool = self.pool_for_source_or_err(source_id)?;
            let repo = self.repository_for(source_id, pool).await?;
            let remaining = limit.saturating_sub(u32::try_from(items.len()).unwrap_or(limit));
            let page = repo.list_groups(remaining, pending_cursor.take()).await?;
            let source_has_more = page.has_more;
            items.extend(page.items);
            if source_has_more {
                if items.len() >= limit as usize {
                    return Ok(Page {
                        items,
                        has_more: true,
                        next_cursor: page.next_cursor,
                    });
                }
                pending_cursor = decode_next_cursor(page.next_cursor)?;
                continue;
            }
            idx += 1;
            pending_cursor = None;
        }

        if items.len() < limit as usize {
            return Ok(Page {
                items,
                has_more: false,
                next_cursor: None,
            });
        }
        match self.first_source_with_groups(&source_ids, idx).await? {
            Some(next_source_id) => Ok(Page {
                items,
                has_more: true,
                next_cursor: encode_resume_cursor(next_source_id)?,
            }),
            None => Ok(Page {
                items,
                has_more: false,
                next_cursor: None,
            }),
        }
    }

    /// Finds the first source (starting at `start_idx`) with at least one
    /// group available, draining exhausted-but-empty pages within a source
    /// before moving to the next one.
    async fn first_source_with_groups(
        &self,
        source_ids: &[SourceId],
        start_idx: usize,
    ) -> Result<Option<SourceId>, sqlx::Error> {
        for source_id in &source_ids[start_idx..] {
            let pool = self.pool_for_source_or_err(source_id)?;
            let repo = self.repository_for(source_id, pool).await?;
            let mut cursor = None;
            loop {
                let page = repo.list_groups(1, cursor.take()).await?;
                if !page.items.is_empty() {
                    return Ok(Some(source_id.clone()));
                }
                if !page.has_more {
                    break;
                }
                cursor = decode_next_cursor(page.next_cursor)?;
            }
        }
        Ok(None)
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
        cursor: Option<ContactListCursor>,
        filters: &ContactFilters,
    ) -> Result<Page<ContactSummary>, sqlx::Error> {
        let source_ids = self.sorted_source_ids();
        if source_ids.is_empty() {
            return Ok(empty_page());
        }
        let (mut idx, mut pending_cursor) = resolve_cursor_start(&source_ids, cursor)?;

        let mut items = Vec::new();
        while idx < source_ids.len() && items.len() < limit as usize {
            let source_id = &source_ids[idx];
            let pool = self.pool_for_source_or_err(source_id)?;
            let repo = self.repository_for(source_id, pool).await?;
            let remaining = limit.saturating_sub(u32::try_from(items.len()).unwrap_or(limit));
            let page = repo
                .list_contacts(remaining, pending_cursor.take(), filters)
                .await?;
            let source_has_more = page.has_more;
            items.extend(page.items);
            if source_has_more {
                if items.len() >= limit as usize {
                    return Ok(Page {
                        items,
                        has_more: true,
                        next_cursor: page.next_cursor,
                    });
                }
                pending_cursor = decode_next_cursor(page.next_cursor)?;
                continue;
            }
            idx += 1;
            pending_cursor = None;
        }

        if items.len() < limit as usize {
            return Ok(Page {
                items,
                has_more: false,
                next_cursor: None,
            });
        }
        match self
            .first_source_with_contacts(&source_ids, idx, filters)
            .await?
        {
            Some(next_source_id) => Ok(Page {
                items,
                has_more: true,
                next_cursor: encode_resume_cursor(next_source_id)?,
            }),
            None => Ok(Page {
                items,
                has_more: false,
                next_cursor: None,
            }),
        }
    }

    /// Finds the first source (starting at `start_idx`) with at least one
    /// matching contact, draining exhausted-but-empty pages within a source
    /// before moving to the next one.
    async fn first_source_with_contacts(
        &self,
        source_ids: &[SourceId],
        start_idx: usize,
        filters: &ContactFilters,
    ) -> Result<Option<SourceId>, sqlx::Error> {
        for source_id in &source_ids[start_idx..] {
            let pool = self.pool_for_source_or_err(source_id)?;
            let repo = self.repository_for(source_id, pool).await?;
            let mut cursor = None;
            loop {
                let page = repo.list_contacts(1, cursor.take(), filters).await?;
                if !page.items.is_empty() {
                    return Ok(Some(source_id.clone()));
                }
                if !page.has_more {
                    break;
                }
                cursor = decode_next_cursor(page.next_cursor)?;
            }
        }
        Ok(None)
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

fn empty_page<T>() -> Page<T> {
    Page {
        items: Vec::new(),
        has_more: false,
        next_cursor: None,
    }
}

/// Resolves where cross-source pagination should resume: the index into
/// `source_ids` to start from, and (if resuming mid-source) the per-source
/// cursor to hand to that source's repository. A missing cursor starts from
/// the first source; `ContactListCursor::row_id == i64::MAX` starts the
/// named source from its own beginning.
fn resolve_cursor_start(
    source_ids: &[SourceId],
    cursor: Option<ContactListCursor>,
) -> Result<(usize, Option<ContactListCursor>), sqlx::Error> {
    let Some(cursor) = cursor else {
        return Ok((0, None));
    };
    let idx = source_ids
        .iter()
        .position(|id| id.as_str() == cursor.source_id)
        .ok_or_else(|| {
            sqlx::Error::Configuration(
                format!("unknown Contacts source in cursor: {}", cursor.source_id).into(),
            )
        })?;
    if cursor.row_id == i64::MAX {
        Ok((idx, None))
    } else {
        Ok((idx, Some(cursor)))
    }
}

/// Decodes a per-source `next_cursor` produced by `ContactsRepository`,
/// which is always a valid `ContactListCursor` for the same source.
fn decode_next_cursor(
    next_cursor: Option<String>,
) -> Result<Option<ContactListCursor>, sqlx::Error> {
    let Some(next_cursor) = next_cursor else {
        return Ok(None);
    };
    crate::api::cursor::decode::<ContactListCursor>(&next_cursor)
        .map(Some)
        .map_err(|_| sqlx::Error::Protocol("invalid Contacts pagination cursor".to_owned()))
}

/// Encodes a cursor that resumes multi-source pagination at the start of
/// `source_id` (see `ContactListCursor` docs for the `row_id::MAX` sentinel).
fn encode_resume_cursor(source_id: SourceId) -> Result<Option<String>, sqlx::Error> {
    let encoded = crate::api::cursor::encode(&ContactListCursor {
        source_id: source_id.into_inner(),
        row_id: i64::MAX,
    })
    .map_err(|_| sqlx::Error::Protocol("failed to encode Contacts pagination cursor".to_owned()))?;
    Ok(Some(encoded))
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
    async fn caches_schema_for_remapped_entity_layout() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = crate::fixtures::ContactsFixtureDb::seeded_with_remapped_entities().await?;
        let pool = connect_pool(fixture.path()).await?;
        let source_id = SourceId::new("remapped-source");
        let sources = ContactsSources::new(HashMap::from([(source_id.clone(), pool)]));

        sources.warm_schemas().await?;

        let schema = sources.cached_schema(&source_id).await?;
        assert_eq!(schema.entity_ids.contact, 30);
        assert_eq!(schema.entity_ids.group, 28);
        assert_eq!(schema.entity_ids.container, 40);
        assert_eq!(schema.parent_groups.table, "Z_30PARENTGROUPS");
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

    /// Builds a multi-source `ContactsSources` where each `tag` becomes an
    /// independent AddressBook source (its own fixture file/pool), seeded
    /// with `groups_per_source` extra groups and `contacts_per_source` extra
    /// contacts on top of the default seed row. Returns the fixtures
    /// alongside the sources so their backing temp files stay alive for the
    /// duration of the test.
    async fn build_multi_source(
        tags: &[&str],
        groups_per_source: u32,
        contacts_per_source: u32,
    ) -> Result<(ContactsSources, Vec<ContactsFixtureDb>), Box<dyn std::error::Error>> {
        let mut pools = HashMap::new();
        let mut fixtures = Vec::new();
        for tag in tags {
            let fixture = ContactsFixtureDb::seeded().await?;
            if groups_per_source > 0 {
                crate::fixtures::seed_extra_groups(fixture.path(), tag, groups_per_source).await?;
            }
            if contacts_per_source > 0 {
                crate::fixtures::seed_tagged_contacts(fixture.path(), tag, contacts_per_source)
                    .await?;
            }
            let pool = connect_pool(fixture.path()).await?;
            pools.insert(SourceId::new(*tag), pool);
            fixtures.push(fixture);
        }
        Ok((ContactsSources::new(pools), fixtures))
    }

    #[tokio::test]
    async fn list_groups_paginates_stably_across_sources() -> Result<(), Box<dyn std::error::Error>>
    {
        use std::collections::HashSet;

        let (sources, _fixtures) = build_multi_source(&["src-a", "src-b"], 3, 0).await?;
        // 1 default seeded group + 3 extra groups per source, across 2 sources.
        const EXPECTED_TOTAL: usize = (1 + 3) * 2;

        let mut seen: HashSet<(String, String)> = HashSet::new();
        let mut cursor = None;
        let mut pages: Vec<HashSet<(String, String)>> = Vec::new();
        loop {
            let page = sources.list_groups(3, cursor.clone()).await?;
            assert!(
                !page.items.is_empty() || pages.is_empty(),
                "page unexpectedly empty"
            );
            let mut page_ids = HashSet::new();
            for item in &page.items {
                let key = (
                    item.source_id.as_str().to_owned(),
                    item.id.as_str().to_owned(),
                );
                assert!(
                    seen.insert(key.clone()),
                    "duplicate group returned across pages: {key:?}"
                );
                page_ids.insert(key);
            }
            pages.push(page_ids);

            if page.has_more {
                let next = page
                    .next_cursor
                    .ok_or("has_more was true but next_cursor was missing")?;
                cursor = Some(crate::api::cursor::decode::<
                    crate::api::cursor::ContactListCursor,
                >(&next)?);
            } else {
                assert!(
                    page.next_cursor.is_none(),
                    "next_cursor should be absent once has_more is false"
                );
                break;
            }
        }

        assert_eq!(seen.len(), EXPECTED_TOTAL);
        assert!(
            pages.len() >= 2,
            "expected pagination across multiple pages"
        );
        assert_ne!(pages[0], pages[1], "page 2 must differ from page 1");
        Ok(())
    }

    #[tokio::test]
    async fn list_contacts_paginates_stably_across_sources()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::collections::HashSet;

        let (sources, _fixtures) = build_multi_source(&["src-a", "src-b"], 0, 3).await?;
        // 1 default seeded contact + 3 extra contacts per source, across 2 sources.
        const EXPECTED_TOTAL: usize = (1 + 3) * 2;

        let mut seen: HashSet<(String, String)> = HashSet::new();
        let mut cursor = None;
        let mut pages: Vec<HashSet<(String, String)>> = Vec::new();
        loop {
            let page = sources
                .list_contacts(3, cursor.clone(), &Default::default())
                .await?;
            let mut page_ids = HashSet::new();
            for item in &page.items {
                let key = (
                    item.source_id.as_str().to_owned(),
                    item.id.as_str().to_owned(),
                );
                assert!(
                    seen.insert(key.clone()),
                    "duplicate contact returned across pages: {key:?}"
                );
                page_ids.insert(key);
            }
            pages.push(page_ids);

            if page.has_more {
                let next = page
                    .next_cursor
                    .ok_or("has_more was true but next_cursor was missing")?;
                cursor = Some(crate::api::cursor::decode::<
                    crate::api::cursor::ContactListCursor,
                >(&next)?);
            } else {
                assert!(
                    page.next_cursor.is_none(),
                    "next_cursor should be absent once has_more is false"
                );
                break;
            }
        }

        assert_eq!(seen.len(), EXPECTED_TOTAL);
        assert!(
            pages.len() >= 2,
            "expected pagination across multiple pages"
        );
        assert_ne!(pages[0], pages[1], "page 2 must differ from page 1");
        Ok(())
    }

    #[tokio::test]
    async fn list_groups_cursor_resumes_at_next_source_boundary()
    -> Result<(), Box<dyn std::error::Error>> {
        // One group per source (the default seed row only): a page that
        // exhausts the first source exactly at `limit` must still report
        // `has_more` with a cursor that resumes at the start of the next
        // source, not stall with a false "no more data".
        let (sources, _fixtures) = build_multi_source(&["src-a", "src-b"], 0, 0).await?;

        let page1 = sources.list_groups(1, None).await?;
        assert_eq!(page1.items.len(), 1);
        assert!(page1.has_more, "expected more groups in the second source");
        let cursor = crate::api::cursor::decode::<crate::api::cursor::ContactListCursor>(
            &page1.next_cursor.ok_or("missing next_cursor")?,
        )?;
        assert_eq!(cursor.source_id, "src-b");

        let page2 = sources.list_groups(1, Some(cursor)).await?;
        assert_eq!(page2.items.len(), 1);
        assert!(!page2.has_more);
        assert!(page2.next_cursor.is_none());
        assert_ne!(page1.items[0].source_id, page2.items[0].source_id);
        Ok(())
    }
}
