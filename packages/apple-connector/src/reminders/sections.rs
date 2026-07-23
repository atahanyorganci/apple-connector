use std::collections::HashMap;

use serde::Deserialize;

use super::{
    model::{Section, SmartFilter},
    row::SectionRow,
};

#[derive(Debug, Deserialize)]
struct MembershipDocument {
    #[serde(default)]
    memberships: Vec<MembershipEntry>,
}

#[derive(Debug, Deserialize)]
struct MembershipEntry {
    #[serde(rename = "memberID")]
    member_id: String,
    #[serde(rename = "groupID")]
    group_id: String,
}

pub fn section_from_row(row: SectionRow) -> Section {
    Section {
        row_id: row.row_id,
        id: row.id,
        display_name: row.display_name.unwrap_or_else(|| "Untitled".to_owned()),
        canonical_name: row.canonical_name,
        list_row_id: row.list_row_id,
    }
}

pub fn parse_section_memberships(data: Option<&[u8]>) -> HashMap<String, String> {
    let Some(data) = data else {
        return HashMap::new();
    };

    let Ok(document) = serde_json::from_slice::<MembershipDocument>(data) else {
        return HashMap::new();
    };

    document
        .memberships
        .into_iter()
        .map(|entry| {
            (
                entry.member_id.to_lowercase(),
                entry.group_id.to_lowercase(),
            )
        })
        .collect()
}

pub fn decode_smart_filter(data: Option<&[u8]>) -> SmartFilter {
    let Some(data) = data else {
        return SmartFilter::default();
    };

    match serde_json::from_slice::<serde_json::Value>(data) {
        Ok(value) => SmartFilter {
            decoded: true,
            raw: Some(value),
        },
        Err(_) => SmartFilter {
            decoded: false,
            raw: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::parse_section_memberships;

    #[test]
    fn parses_membership_json() {
        let json = br#"{"minimumSupportedVersion":20230430,"memberships":[{"memberID":"bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb","groupID":"dddddddd-dddd-dddd-dddd-dddddddddddd","modifiedOn":796647639.739}]}"#;
        let map = parse_section_memberships(Some(json));
        assert_eq!(
            map.get("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"),
            Some(&"dddddddd-dddd-dddd-dddd-dddddddddddd".to_owned())
        );
    }
}
