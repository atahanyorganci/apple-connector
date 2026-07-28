use sqlx::{QueryBuilder, Sqlite};

#[derive(Debug, Clone, Default)]
pub struct ContactFilters {
    pub q: Option<String>,
    pub container_id: Option<String>,
    pub group_id: Option<String>,
}

pub fn apply_contact_filters(builder: &mut QueryBuilder<Sqlite>, filters: &ContactFilters) {
    if let Some(q) = &filters.q {
        let pattern = format!("%{q}%");
        builder.push(" AND (r.ZFIRSTNAME LIKE ");
        builder.push_bind(pattern.clone());
        builder.push(" OR r.ZLASTNAME LIKE ");
        builder.push_bind(pattern.clone());
        builder.push(" OR r.ZORGANIZATION LIKE ");
        builder.push_bind(pattern.clone());
        builder.push(" OR r.ZNAME LIKE ");
        builder.push_bind(pattern);
        builder.push(")");
    }
    if let Some(container_id) = &filters.container_id {
        builder.push(
            " AND lower(substr(c.ZUNIQUEID, 1, instr(c.ZUNIQUEID, ':') - 1)) = lower(",
        );
        builder.push_bind(container_id.clone());
        builder.push(")");
    }
    if let Some(group_id) = &filters.group_id {
        builder.push(
            " AND r.Z_PK IN (SELECT pg.Z_22CONTACTS FROM Z_22PARENTGROUPS pg \
             JOIN ZABCDRECORD g ON g.Z_PK = pg.Z_19PARENTGROUPS1 \
             WHERE lower(substr(g.ZUNIQUEID, 1, instr(g.ZUNIQUEID, ':') - 1)) = lower(",
        );
        builder.push_bind(group_id.clone());
        builder.push("))");
    }
}
