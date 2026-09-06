//! RFC 6350 coverage: modelled fields, unknown-property preservation, PHOTO
//! interoperability between vCard 3 input and vCard 4 output.

use serde_vcard::{Photo, SocialProfile, Url, VCard, from_str, parse_vcards, to_string};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn card(body: &str) -> String {
    format!("BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Test Person\r\n{body}\r\nEND:VCARD\r\n")
}

fn unfolded(vcf: &str) -> String {
    vcf.replace("\r\n ", "").replace("\n ", "")
}

#[test]
fn urls_parse_and_serialize() -> TestResult {
    // `urls` was declared on the model but never read or written, so every
    // URL silently vanished.
    let parsed = from_str::<VCard>(&card("URL;TYPE=work:https://example.com/team"))?;
    assert_eq!(parsed.urls.len(), 1);
    assert_eq!(parsed.urls[0].url, "https://example.com/team");
    assert_eq!(parsed.urls[0].label.as_deref(), Some("work"));

    let vcf = unfolded(&to_string(&parsed)?);
    assert!(vcf.contains("URL"), "{vcf}");
    assert!(vcf.contains("https://example.com/team"), "{vcf}");

    let reparsed = from_str::<VCard>(&vcf)?;
    assert_eq!(reparsed.urls, parsed.urls);
    Ok(())
}

#[test]
fn social_profiles_parse_and_serialize() -> TestResult {
    let parsed = from_str::<VCard>(&card(
        "X-SOCIALPROFILE;TYPE=twitter;x-user=ada:https://twitter.com/ada",
    ))?;
    assert_eq!(parsed.social_profiles.len(), 1);
    let profile = &parsed.social_profiles[0];
    assert_eq!(profile.service.as_deref(), Some("twitter"));
    assert_eq!(profile.username.as_deref(), Some("ada"));

    let vcf = unfolded(&to_string(&parsed)?);
    assert!(vcf.contains("X-SOCIALPROFILE"), "{vcf}");

    let reparsed = from_str::<VCard>(&vcf)?;
    assert_eq!(reparsed.social_profiles, parsed.social_profiles);
    Ok(())
}

#[test]
fn impp_is_read_as_a_social_profile() -> TestResult {
    let parsed = from_str::<VCard>(&card("IMPP;X-SERVICE-TYPE=Skype:skype:ada.lovelace"))?;
    let profile = parsed
        .social_profiles
        .first()
        .ok_or("expected a social profile")?;
    assert_eq!(profile.service.as_deref(), Some("Skype"));
    Ok(())
}

#[test]
fn unknown_properties_are_preserved() -> TestResult {
    // These all fell through the `_ => {}` arm and were dropped.
    let parsed = from_str::<VCard>(&card(
        "CATEGORIES:friends,colleagues\r\nROLE:Engineer\r\nGEO:geo:37.4,-122.1",
    ))?;
    assert_eq!(parsed.unknown.len(), 3);

    let names: Vec<&str> = parsed
        .unknown
        .iter()
        .map(|property| property.name.as_str())
        .collect();
    assert_eq!(names, vec!["CATEGORIES", "ROLE", "GEO"]);

    let vcf = unfolded(&to_string(&parsed)?);
    assert!(vcf.contains("CATEGORIES:friends,colleagues"), "{vcf}");
    assert!(vcf.contains("ROLE:Engineer"), "{vcf}");
    assert!(vcf.contains("GEO:geo:37.4,-122.1"), "{vcf}");

    let reparsed = from_str::<VCard>(&vcf)?;
    assert_eq!(reparsed.unknown, parsed.unknown);
    Ok(())
}

#[test]
fn v3_photo_input_becomes_v4_output() -> TestResult {
    // "hello" as base64.
    let v3 = "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Photo Person\r\n\
              PHOTO;ENCODING=b;TYPE=JPEG:aGVsbG8=\r\nEND:VCARD\r\n";
    let parsed = from_str::<VCard>(v3)?;
    let photo = parsed.photo.clone().ok_or("expected a photo")?;
    assert_eq!(
        photo,
        Photo::Inline {
            data: b"hello".to_vec(),
            // vCard 3 wrote a bare format name; v4 wants a media type.
            media_type: Some("image/jpeg".to_owned()),
        }
    );

    let vcf = unfolded(&to_string(&parsed)?);
    assert!(
        vcf.contains("PHOTO:data:image/jpeg;base64,aGVsbG8="),
        "{vcf}"
    );
    assert!(!vcf.contains("ENCODING=b"), "{vcf}");

    let reparsed = from_str::<VCard>(&vcf)?;
    assert_eq!(reparsed.photo, parsed.photo);
    Ok(())
}

#[test]
fn v4_data_uri_photo_parses_instead_of_failing_the_whole_card() -> TestResult {
    // Base64-decoding the raw value made this error, and the error propagated
    // out of apply_line, so a valid vCard 4 card failed to parse entirely.
    let parsed = from_str::<VCard>(&card("PHOTO:data:image/png;base64,aGVsbG8="))?;
    assert_eq!(parsed.formatted_name.as_deref(), Some("Test Person"));
    assert_eq!(
        parsed.photo,
        Some(Photo::Inline {
            data: b"hello".to_vec(),
            media_type: Some("image/png".to_owned()),
        })
    );
    Ok(())
}

#[test]
fn photo_uris_stay_uris() -> TestResult {
    let parsed = from_str::<VCard>(&card("PHOTO:https://example.com/ada.jpg"))?;
    assert_eq!(
        parsed.photo,
        Some(Photo::Uri {
            uri: "https://example.com/ada.jpg".to_owned(),
        })
    );

    let vcf = unfolded(&to_string(&parsed)?);
    assert!(vcf.contains("PHOTO:https://example.com/ada.jpg"), "{vcf}");
    Ok(())
}

#[test]
fn unterminated_cards_are_an_error() -> TestResult {
    let error = from_str::<VCard>("BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Truncated\r\n")
        .err()
        .ok_or("expected an unterminated-card error")?;
    assert!(
        error.to_string().contains("END:VCARD"),
        "unexpected error: {error}"
    );
    Ok(())
}

#[test]
fn quoted_parameter_values_may_contain_a_colon() -> TestResult {
    // Splitting at the first colon regardless of quoting cut this line inside
    // the parameter and produced a nonsense property.
    let parsed = from_str::<VCard>(&card("TEL;TYPE=\"work:main\":+15551234567"))?;
    assert_eq!(parsed.phones.len(), 1);
    assert_eq!(parsed.phones[0].number, "+15551234567");
    assert_eq!(parsed.phones[0].label.as_deref(), Some("work:main"));
    Ok(())
}

#[test]
fn apple_group_labels_become_the_property_label() -> TestResult {
    let apple = "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Apple Person\r\n\
                 item1.TEL:+15551230000\r\n\
                 item1.X-ABLabel:_$!<School>!$_\r\n\
                 END:VCARD\r\n";
    let parsed = from_str::<VCard>(apple)?;
    assert_eq!(parsed.phones.len(), 1);
    assert_eq!(parsed.phones[0].label.as_deref(), Some("School"));
    Ok(())
}

#[test]
fn type_is_written_once_per_property() -> TestResult {
    // label and phone_type were both filled from TYPE, so the serializer
    // emitted TYPE= twice on every phone.
    let parsed = from_str::<VCard>(&card("TEL;TYPE=CELL:+15551234567"))?;
    let vcf = unfolded(&to_string(&parsed)?);
    let tel_line = vcf
        .lines()
        .find(|line| line.starts_with("TEL"))
        .ok_or("expected a TEL line")?;
    assert_eq!(tel_line.matches("TYPE=").count(), 1, "{tel_line}");
    Ok(())
}

#[test]
fn bare_type_parameters_are_understood() -> TestResult {
    let parsed = from_str::<VCard>(&card("TEL;WORK;VOICE:+15551234567"))?;
    assert_eq!(parsed.phones[0].label.as_deref(), Some("WORK"));
    Ok(())
}

#[test]
fn folded_values_keep_significant_spaces() -> TestResult {
    // Continuations were trimmed, which silently ate spaces inside a value.
    let folded =
        "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Fold Person\r\nNOTE:first\r\n  second\r\nEND:VCARD\r\n";
    let parsed = from_str::<VCard>(folded)?;
    assert_eq!(parsed.note.as_deref(), Some("first second"));
    Ok(())
}

#[test]
fn multiple_cards_still_parse() -> TestResult {
    let input = "BEGIN:VCARD\r\nFN:One\r\nEND:VCARD\r\nBEGIN:VCARD\r\nFN:Two\r\nEND:VCARD\r\n";
    let cards = parse_vcards(input)?;
    assert_eq!(cards.len(), 2);
    Ok(())
}

#[test]
fn a_full_card_round_trips_without_loss() -> TestResult {
    let original = VCard {
        formatted_name: Some("Ada Lovelace".to_owned()),
        urls: vec![Url {
            url: "https://example.com".to_owned(),
            label: Some("work".to_owned()),
            preferred: true,
        }],
        social_profiles: vec![SocialProfile {
            service: Some("github".to_owned()),
            username: Some("ada".to_owned()),
            url: Some("https://github.com/ada".to_owned()),
            label: None,
            preferred: false,
        }],
        photo: Some(Photo::Inline {
            data: b"hello".to_vec(),
            media_type: Some("image/png".to_owned()),
        }),
        ..VCard::default()
    };

    let reparsed = from_str::<VCard>(&to_string(&original)?)?;
    assert_eq!(reparsed.formatted_name, original.formatted_name);
    assert_eq!(reparsed.urls, original.urls);
    assert_eq!(reparsed.photo, original.photo);
    assert_eq!(
        reparsed.social_profiles[0].username,
        original.social_profiles[0].username
    );
    Ok(())
}
