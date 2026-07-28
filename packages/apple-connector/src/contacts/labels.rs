/// Decode Apple AddressBook label constants to human-readable labels.
pub fn decode_label(label: Option<&str>) -> Option<String> {
    let label = label?;
    if label.starts_with("_$!<") && label.ends_with(">!$_") {
        let inner = label
            .trim_start_matches("_$!<")
            .trim_end_matches(">!$_");
        return Some(inner.to_owned());
    }
    Some(label.to_owned())
}

#[cfg(test)]
mod tests {
    use super::decode_label;

    #[test]
    fn decodes_apple_label_constants() {
        assert_eq!(
            decode_label(Some("_$!<Mobile>!$_")).as_deref(),
            Some("Mobile")
        );
        assert_eq!(
            decode_label(Some("_$!<Work>!$_")).as_deref(),
            Some("Work")
        );
    }

    #[test]
    fn passes_through_custom_labels() {
        assert_eq!(decode_label(Some("Custom")).as_deref(), Some("Custom"));
    }
}
