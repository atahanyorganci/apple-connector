use sqlx::FromRow;

pub use crate::apple_types::parse_core_data_timestamp;

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct ContainerRow {
    pub row_id: i64,
    pub unique_id: String,
    pub name: Option<String>,
    pub container_type: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct GroupRow {
    pub row_id: i64,
    pub unique_id: String,
    pub name: Option<String>,
    pub container_row_id: Option<i64>,
    pub container_unique_id: Option<String>,
    pub group_type: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct ContactRow {
    pub row_id: i64,
    pub unique_id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub middle_name: Option<String>,
    pub nickname: Option<String>,
    pub organization: Option<String>,
    pub job_title: Option<String>,
    pub department: Option<String>,
    pub display_name: Option<String>,
    pub container_row_id: Option<i64>,
    pub container_unique_id: Option<String>,
    pub creation_date: Option<f64>,
    pub modification_date: Option<f64>,
    pub birthday: Option<f64>,
    pub note_text: Option<String>,
    pub has_photo: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct PhoneRow {
    pub unique_id: String,
    pub number: Option<String>,
    pub label: Option<String>,
    pub is_primary: Option<i64>,
    pub ordering_index: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct EmailRow {
    pub unique_id: String,
    pub address: Option<String>,
    pub label: Option<String>,
    pub is_primary: Option<i64>,
    pub ordering_index: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct AddressRow {
    pub unique_id: String,
    pub street: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub label: Option<String>,
    pub is_primary: Option<i64>,
    pub ordering_index: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct UrlRow {
    pub unique_id: String,
    pub url: Option<String>,
    pub label: Option<String>,
    pub is_primary: Option<i64>,
    pub ordering_index: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct SocialRow {
    pub unique_id: String,
    pub service: Option<String>,
    pub username: Option<String>,
    pub url: Option<String>,
    pub label: Option<String>,
    pub is_primary: Option<i64>,
    pub ordering_index: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
pub struct GroupIdRow {
    pub unique_id: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct PhotoRow {
    pub photo_data: Option<Vec<u8>>,
    pub image_type: Option<String>,
}

pub fn api_id_from_unique_id(unique_id: &str) -> String {
    unique_id
        .split_once(':')
        .map(|(id, _)| id.to_owned())
        .unwrap_or_else(|| unique_id.to_owned())
}
