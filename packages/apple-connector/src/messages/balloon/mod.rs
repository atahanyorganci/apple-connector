mod nskeyed;

use base64::{Engine as _, engine::general_purpose};

use self::nskeyed::{as_dictionary, decode_ns_keyed_archiver, dict_string};
use super::model::{
    AppBalloon, AppBalloonKind, PhotosBalloon, PollBalloon, PollOption, UrlBalloon,
};

/// Build a typed [`AppBalloon`] from chat.db `balloon_bundle_id` + `payload_data`.
pub fn decode(bundle_id: String, payload_data: Option<&[u8]>, text: Option<String>) -> AppBalloon {
    let kind = match balloon_app_id(&bundle_id) {
        "com.apple.DigitalTouchBalloonProvider" => AppBalloonKind::DigitalTouch,
        "com.apple.messages.URLBalloonProvider" => payload_data
            .and_then(parse_url_balloon)
            .map(AppBalloonKind::Url)
            .unwrap_or_else(|| unknown_kind(payload_data)),
        "com.apple.mobileslideshow.PhotosMessagesApp" => payload_data
            .and_then(parse_photos_balloon)
            .map(AppBalloonKind::Photos)
            .unwrap_or_else(|| unknown_kind(payload_data)),
        "com.apple.messages.Polls" => payload_data
            .and_then(parse_poll_balloon)
            .map(AppBalloonKind::Poll)
            .unwrap_or_else(|| unknown_kind(payload_data)),
        _ => unknown_kind(payload_data),
    };

    AppBalloon {
        bundle_id,
        text,
        kind,
    }
}

fn balloon_app_id(bundle_id: &str) -> &str {
    bundle_id.rsplit(':').next().unwrap_or(bundle_id)
}

fn unknown_kind(payload_data: Option<&[u8]>) -> AppBalloonKind {
    AppBalloonKind::Unknown {
        payload_data: payload_data.map(ToOwned::to_owned),
    }
}

fn parse_url_balloon(bytes: &[u8]) -> Option<UrlBalloon> {
    let root = decode_ns_keyed_archiver(bytes).ok()?;
    let root = as_dictionary(&root)?;
    let metadata = root
        .get("richLinkMetadata")
        .or_else(|| root.get("metadata"))
        .and_then(as_dictionary)?;

    Some(UrlBalloon {
        url: dict_string(metadata, "URL"),
        original_url: dict_string(metadata, "originalURL"),
        title: dict_string(metadata, "title"),
        summary: dict_string(metadata, "summary"),
        site_name: dict_string(metadata, "siteName"),
    })
}

fn parse_photos_balloon(bytes: &[u8]) -> Option<PhotosBalloon> {
    let root = decode_ns_keyed_archiver(bytes).ok()?;
    let root = as_dictionary(&root)?;
    let user_info = root.get("userInfo").and_then(as_dictionary);

    Some(PhotosBalloon {
        url: dict_string(root, "URL"),
        app_name: dict_string(root, "an"),
        ldtext: dict_string(root, "ldtext"),
        caption: user_info.and_then(|info| dict_string(info, "caption")),
        subcaption: user_info.and_then(|info| dict_string(info, "subcaption")),
    })
}

fn parse_poll_balloon(bytes: &[u8]) -> Option<PollBalloon> {
    let root = decode_ns_keyed_archiver(bytes).ok()?;
    let root = as_dictionary(&root)?;
    let url = dict_string(root, "URL")?;
    let json = decode_data_url_json(&url)?;
    let item = json.get("item")?;

    let options = item
        .get("orderedPollOptions")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|option| {
            Some(PollOption {
                text: option.get("text")?.as_str()?.to_owned(),
                option_id: option
                    .get("optionIdentifier")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_owned(),
                creator_handle: option
                    .get("creatorHandle")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned),
            })
        })
        .collect();

    Some(PollBalloon {
        title: item
            .get("title")
            .and_then(|value| value.as_str())
            .filter(|title| !title.is_empty())
            .map(str::to_owned),
        creator_handle: item
            .get("creatorHandle")
            .and_then(|value| value.as_str())
            .map(str::to_owned),
        options,
        ldtext: dict_string(root, "ldtext"),
    })
}

fn decode_data_url_json(url: &str) -> Option<serde_json::Value> {
    let encoded = url.strip_prefix("data:,")?;
    let encoded = encoded
        .split_once('?')
        .map(|(body, _)| body)
        .unwrap_or(encoded);
    let bytes = general_purpose::STANDARD
        .decode(encoded)
        .or_else(|_| general_purpose::URL_SAFE.decode(encoded))
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::{balloon_app_id, decode};
    use crate::messages::model::AppBalloonKind;

    #[test]
    fn strips_extension_plugin_prefix() {
        assert_eq!(
            balloon_app_id(
                "com.apple.messages.MSMessageExtensionBalloonPlugin:0000000000:com.apple.messages.Polls"
            ),
            "com.apple.messages.Polls"
        );
        assert_eq!(
            balloon_app_id("com.apple.messages.URLBalloonProvider"),
            "com.apple.messages.URLBalloonProvider"
        );
    }

    #[test]
    fn decodes_url_balloon_fixture() {
        let bytes = include_bytes!("../../../fixtures/messages/balloons/url.plist");
        let balloon = decode(
            "com.apple.messages.URLBalloonProvider".to_owned(),
            Some(bytes),
            None,
        );

        match balloon.kind {
            AppBalloonKind::Url(url) => {
                assert_eq!(url.url.as_deref(), Some("https://google.com/"));
                assert_eq!(
                    url.original_url.as_deref(),
                    Some("https://share.google/ZpgCQPccKACrHSLm1")
                );
                assert!(url.title.as_deref().unwrap().contains("Michelin"));
                assert_eq!(url.site_name.as_deref(), Some("www.google.com"));
            }
            other => panic!("expected Url, got {other:?}"),
        }
    }

    #[test]
    fn decodes_simple_url_balloon_fixture() {
        let bytes = include_bytes!("../../../fixtures/messages/balloons/url-simple.plist");
        let balloon = decode(
            "com.apple.messages.URLBalloonProvider".to_owned(),
            Some(bytes),
            None,
        );

        match balloon.kind {
            AppBalloonKind::Url(url) => {
                assert_eq!(url.url.as_deref(), Some("https://example.com/"));
                assert_eq!(url.title.as_deref(), Some("Example Domain"));
            }
            other => panic!("expected Url, got {other:?}"),
        }
    }

    #[test]
    fn decodes_photos_balloon_fixture() {
        let bytes = include_bytes!("../../../fixtures/messages/balloons/photos.plist");
        let balloon = decode(
            "com.apple.messages.MSMessageExtensionBalloonPlugin:0000000000:com.apple.mobileslideshow.PhotosMessagesApp"
                .to_owned(),
            Some(bytes),
            None,
        );

        match balloon.kind {
            AppBalloonKind::Photos(photos) => {
                assert_eq!(photos.app_name.as_deref(), Some("Photos"));
                assert_eq!(photos.ldtext.as_deref(), Some("Today - 14 Items"));
                assert_eq!(photos.caption.as_deref(), Some("Today"));
                assert_eq!(photos.subcaption.as_deref(), Some("14 Items"));
                assert!(
                    photos
                        .url
                        .as_deref()
                        .unwrap()
                        .starts_with("https://share.icloud.com/photos/")
                );
            }
            other => panic!("expected Photos, got {other:?}"),
        }
    }

    #[test]
    fn decodes_poll_balloon_fixture() {
        let bytes = include_bytes!("../../../fixtures/messages/balloons/polls.plist");
        let balloon = decode(
            "com.apple.messages.MSMessageExtensionBalloonPlugin:0000000000:com.apple.messages.Polls"
                .to_owned(),
            Some(bytes),
            None,
        );

        match balloon.kind {
            AppBalloonKind::Poll(poll) => {
                assert_eq!(poll.ldtext.as_deref(), Some("Sent a poll"));
                assert_eq!(poll.creator_handle.as_deref(), Some("+905056704480"));
                assert_eq!(poll.options.len(), 3);
                assert_eq!(poll.options[0].text, "Choice 1");
                assert_eq!(poll.options[1].text, "Choice 2");
                assert_eq!(poll.options[2].text, "Choice 3");
            }
            other => panic!("expected Poll, got {other:?}"),
        }
    }

    #[test]
    fn digital_touch_has_no_payload() {
        let balloon = decode(
            "com.apple.DigitalTouchBalloonProvider".to_owned(),
            None,
            None,
        );
        assert!(matches!(balloon.kind, AppBalloonKind::DigitalTouch));
    }

    #[test]
    fn unknown_bundle_keeps_opaque_payload() {
        let balloon = decode(
            "com.example.UnknownBalloon".to_owned(),
            Some(b"not-a-plist"),
            Some("hi".to_owned()),
        );
        assert_eq!(balloon.text.as_deref(), Some("hi"));
        match balloon.kind {
            AppBalloonKind::Unknown { payload_data } => {
                assert_eq!(payload_data.as_deref(), Some(b"not-a-plist".as_slice()));
            }
            other => panic!("expected Unknown, got {other:?}"),
        }
    }
}
