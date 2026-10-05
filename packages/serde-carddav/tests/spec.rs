//! Claims made in `packages/serde-carddav/docs/SPEC.md` that no other test covered.

use serde_carddav::from_str;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CARD: &str = "BEGIN:VCARD\nVERSION:4.0\nFN:Jane Doe\nEND:VCARD";

/// One `address-data` payload that is not a valid vCard fails the whole document, not just its
/// response.
#[test]
fn an_invalid_payload_fails_the_whole_document() -> TestResult {
    let xml = format!(
        r#"<d:multistatus xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
  <d:response><d:href>/good.vcf</d:href><d:propstat><d:prop>
    <card:address-data>{CARD}</card:address-data></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>
  <d:response><d:href>/bad.vcf</d:href><d:propstat><d:prop>
    <card:address-data>BEGIN:VCARD</card:address-data></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>
</d:multistatus>"#
    );
    assert!(from_str(&xml).is_err());
    Ok(())
}

/// The response's `href` is the response's own `DAV:href`, not an href nested inside a property
/// value such as `current-user-principal`.
#[test]
#[ignore = "bug: a DAV:href nested in a property replaces the response href (SPEC.md, Known bugs)"]
fn a_nested_href_does_not_replace_the_response_href() -> TestResult {
    let xml = format!(
        r#"<d:multistatus xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
  <d:response><d:href>/addressbooks/home/jane.vcf</d:href><d:propstat><d:prop>
    <d:current-user-principal><d:href>/principals/me/</d:href></d:current-user-principal>
    <card:address-data>{CARD}</card:address-data></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>
</d:multistatus>"#
    );
    let parsed = from_str(&xml)?;
    let href = parsed.responses.first().and_then(|r| r.href.as_deref());
    assert_eq!(href, Some("/addressbooks/home/jane.vcf"));
    Ok(())
}
