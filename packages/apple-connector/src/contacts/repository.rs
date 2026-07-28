use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use super::{
    assembly::{
        contact_detail_from_row, contact_summary_from_row, container_from_row, group_from_row,
        ContactRelatedRows,
    },
    model::{ContactDetail, ContactGroup, ContactSummary, Container},
    row::{
        AddressRow, ContactRow, ContainerRow, EmailRow, GroupIdRow, GroupRow, PhoneRow,
        PhotoRow, SocialRow, UrlRow,
    },
    search::{ContactFilters, apply_contact_filters},
    sql::{
        ADDRESS_SELECT, CONTACT_EXTERNAL_ID_SELECT, CONTACT_SELECT, CONTAINER_RESOLVE_SELECT,
        CONTAINER_SELECT, EMAIL_SELECT, GROUP_IDS_FOR_CONTACT, GROUP_RESOLVE_SELECT, GROUP_SELECT,
        PHONE_SELECT, PHOTO_SELECT, SOCIAL_SELECT, URL_SELECT,
    },
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
        let rows: Vec<ContainerRow> = sqlx::query_as(CONTAINER_SELECT)
            .fetch_all(self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| container_from_row(row, self.source_id.clone()))
            .collect())
    }

    pub async fn get_container(
        &self,
        container_id: &str,
    ) -> Result<Option<Container>, sqlx::Error> {
        let mut builder = QueryBuilder::<Sqlite>::new(CONTAINER_SELECT);
        builder.push(" AND lower(substr(r.ZUNIQUEID, 1, instr(r.ZUNIQUEID, ':') - 1)) = lower(");
        builder.push_bind(container_id.to_owned());
        builder.push(")");
        let row: Option<ContainerRow> = builder.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| container_from_row(row, self.source_id.clone())))
    }

    pub async fn list_groups(
        &self,
        limit: u32,
        cursor: Option<ContactListCursor>,
    ) -> Result<Page<ContactGroup>, sqlx::Error> {
        let fetch_limit = i64::from(limit) + 1;
        let mut builder = QueryBuilder::<Sqlite>::new(GROUP_SELECT);
        builder.push(" AND 1=1");
        if let Some(cursor) = cursor {
            builder.push(" AND r.Z_PK < ");
            builder.push_bind(cursor.row_id);
        }
        builder.push(" ORDER BY r.Z_PK DESC LIMIT ");
        builder.push_bind(fetch_limit);
        let rows: Vec<GroupRow> = builder.build_query_as().fetch_all(self.pool).await?;
        Ok(split_page(rows, limit, |row| {
            group_from_row(row, self.source_id.clone())
        }, |row| row.row_id))
    }

    pub async fn get_group(&self, group_id: &str) -> Result<Option<ContactGroup>, sqlx::Error> {
        let mut builder = QueryBuilder::<Sqlite>::new(GROUP_SELECT);
        builder.push(" AND lower(substr(r.ZUNIQUEID, 1, instr(r.ZUNIQUEID, ':') - 1)) = lower(");
        builder.push_bind(group_id.to_owned());
        builder.push(")");
        let row: Option<GroupRow> = builder.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| group_from_row(row, self.source_id.clone())))
    }

    pub async fn list_contacts(
        &self,
        limit: u32,
        cursor: Option<ContactListCursor>,
        filters: &ContactFilters,
    ) -> Result<Page<ContactSummary>, sqlx::Error> {
        let fetch_limit = i64::from(limit) + 1;
        let mut builder = QueryBuilder::<Sqlite>::new(CONTACT_SELECT);
        apply_contact_filters(&mut builder, filters);
        if let Some(cursor) = cursor {
            builder.push(" AND r.Z_PK < ");
            builder.push_bind(cursor.row_id);
        }
        builder.push(" ORDER BY r.Z_PK DESC LIMIT ");
        builder.push_bind(fetch_limit);
        let rows: Vec<ContactRow> = builder.build_query_as().fetch_all(self.pool).await?;
        Ok(split_page(rows, limit, |row| {
            contact_summary_from_row(row, self.source_id.clone())
        }, |row| row.row_id))
    }

    pub async fn list_group_contacts(
        &self,
        group_id: &str,
        limit: u32,
        cursor: Option<GroupContactCursor>,
    ) -> Result<Page<ContactSummary>, sqlx::Error> {
        let fetch_limit = i64::from(limit) + 1;
        let mut builder = QueryBuilder::<Sqlite>::new(CONTACT_SELECT);
        builder.push(
            " AND r.Z_PK IN (SELECT pg.Z_22CONTACTS FROM Z_22PARENTGROUPS pg \
             JOIN ZABCDRECORD g ON g.Z_PK = pg.Z_19PARENTGROUPS1 \
             WHERE lower(substr(g.ZUNIQUEID, 1, instr(g.ZUNIQUEID, ':') - 1)) = lower(",
        );
        builder.push_bind(group_id.to_owned());
        builder.push("))");
        if let Some(cursor) = cursor {
            builder.push(" AND r.Z_PK < ");
            builder.push_bind(cursor.row_id);
        }
        builder.push(" ORDER BY r.Z_PK DESC LIMIT ");
        builder.push_bind(fetch_limit);
        let rows: Vec<ContactRow> = builder.build_query_as().fetch_all(self.pool).await?;
        Ok(split_page(rows, limit, |row| {
            contact_summary_from_row(row, self.source_id.clone())
        }, |row| row.row_id))
    }

    pub async fn get_contact(&self, contact_id: &str) -> Result<Option<ContactDetail>, sqlx::Error> {
        let mut builder = QueryBuilder::<Sqlite>::new(CONTACT_SELECT);
        builder.push(" AND lower(substr(r.ZUNIQUEID, 1, instr(r.ZUNIQUEID, ':') - 1)) = lower(");
        builder.push_bind(contact_id.to_owned());
        builder.push(")");
        let row: Option<ContactRow> = builder.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(self.hydrate_contact(row).await?))
    }

    pub async fn get_contact_photo(
        &self,
        contact_id: &str,
    ) -> Result<Option<(Vec<u8>, Option<String>)>, sqlx::Error> {
        let row: Option<PhotoRow> = sqlx::query_as(PHOTO_SELECT)
            .bind(format!("{contact_id}:"))
            .fetch_optional(self.pool)
            .await?;
        Ok(row.and_then(|row| {
            row.photo_data
                .map(|data| (data, row.image_type))
        }))
    }

    pub async fn get_container_resolve_metadata(
        &self,
        container_id: &str,
    ) -> Result<Option<ContainerResolveMetadata>, sqlx::Error> {
        let row: Option<(String, Option<String>, Option<i64>)> =
            sqlx::query_as(CONTAINER_RESOLVE_SELECT)
                .bind(container_id.to_owned())
                .fetch_optional(self.pool)
                .await?;
        Ok(row.map(|(api_id, name, container_type)| ContainerResolveMetadata {
            api_id,
            name,
            container_type: container_type.unwrap_or(0),
        }))
    }

    pub async fn get_group_resolve_metadata(
        &self,
        group_id: &str,
    ) -> Result<Option<GroupResolveMetadata>, sqlx::Error> {
        type Row = (String, Option<String>, Option<i64>, Option<i64>);
        let row: Option<Row> = sqlx::query_as(GROUP_RESOLVE_SELECT)
                .bind(group_id.to_owned())
                .fetch_optional(self.pool)
                .await?;
        Ok(row.map(
            |(api_id, name, group_type, container_id)| GroupResolveMetadata {
                api_id,
                name,
                group_type: group_type.unwrap_or(0),
                container_id,
            },
        ))
    }

    pub async fn get_contact_external_id(
        &self,
        contact_id: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let row: Option<(String,)> = sqlx::query_as(CONTACT_EXTERNAL_ID_SELECT)
            .bind(contact_id.to_owned())
            .fetch_optional(self.pool)
            .await?;
        Ok(row.map(|(id,)| id))
    }

    async fn hydrate_contact(&self, row: ContactRow) -> Result<ContactDetail, sqlx::Error> {
        let row_id = row.row_id;
        let phones: Vec<PhoneRow> = sqlx::query_as(PHONE_SELECT)
            .bind(row_id)
            .fetch_all(self.pool)
            .await?;
        let emails: Vec<EmailRow> = sqlx::query_as(EMAIL_SELECT)
            .bind(row_id)
            .fetch_all(self.pool)
            .await?;
        let addresses: Vec<AddressRow> = sqlx::query_as(ADDRESS_SELECT)
            .bind(row_id)
            .fetch_all(self.pool)
            .await?;
        let urls: Vec<UrlRow> = sqlx::query_as(URL_SELECT)
            .bind(row_id)
            .fetch_all(self.pool)
            .await?;
        let socials: Vec<SocialRow> = sqlx::query_as(SOCIAL_SELECT)
            .bind(row_id)
            .fetch_all(self.pool)
            .await?;
        let groups: Vec<GroupIdRow> = sqlx::query_as(GROUP_IDS_FOR_CONTACT)
            .bind(row_id)
            .fetch_all(self.pool)
            .await?;
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
            groups.into_iter().map(|g| g.unique_id).collect(),
        ))
    }
}

fn split_page<T, R, F, K>(rows: Vec<R>, limit: u32, map: F, row_id: K) -> Page<T>
where
    F: Fn(R) -> T,
    K: Fn(&R) -> i64,
{
    let has_more = rows.len() > limit as usize;
    let last_row_id = rows
        .get(limit.saturating_sub(1) as usize)
        .map(row_id);
    let items: Vec<T> = rows
        .into_iter()
        .take(limit as usize)
        .map(map)
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
    async fn fixture_lists_containers_groups_and_contacts() {
        let fixture = ContactsFixtureDb::seeded().await.expect("fixture");
        let pool = connect_pool(fixture.path()).await.expect("pool");
        let repo = ContactsRepository::new(&pool, SourceId::new("fixture-source"));

        let containers = repo.list_containers().await.expect("containers");
        assert!(!containers.is_empty());
        assert!(containers.iter().any(|c| c.id.as_str() == SEED_CONTAINER_ID));

        let groups = repo.list_groups(50, None).await.expect("groups");
        assert!(groups.items.iter().any(|g| g.id.as_str() == SEED_GROUP_ID));

        let contacts = repo.list_contacts(50, None, &Default::default()).await.expect("contacts");
        assert!(contacts.items.iter().any(|c| c.id.as_str() == SEED_CONTACT_ID));

        let detail = repo
            .get_contact(SEED_CONTACT_ID)
            .await
            .expect("detail")
            .expect("contact");
        assert_eq!(detail.first_name.as_deref(), Some("Jane"));
        assert_eq!(detail.phones.len(), 1);
        assert_eq!(detail.emails.len(), 1);
    }
}
