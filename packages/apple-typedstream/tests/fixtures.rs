use apple_typedstream::{Value, from_slice};

macro_rules! fixture_test {
    ($name:ident, $file:literal) => {
        #[test]
        fn $name() {
            let data = include_bytes!(concat!("../fixtures/", $file));
            let value: Value = from_slice(data).expect(concat!("decode ", $file));
            insta::assert_debug_snapshot!(value);
        }
    };
}

fixture_test!(plain_short, "attributed-body-01-plain-short.bin");
fixture_test!(spaced, "attributed-body-02-spaced.bin");
fixture_test!(long, "attributed-body-03-long.bin");
fixture_test!(multiline, "attributed-body-04-multiline.bin");
fixture_test!(url, "attributed-body-05-url.bin");
fixture_test!(text_url, "attributed-body-06-text-url.bin");
fixture_test!(phone, "attributed-body-07-phone.bin");
fixture_test!(email, "attributed-body-08-email.bin");
fixture_test!(emoji, "attributed-body-09-emoji.bin");
fixture_test!(emoji_text, "attributed-body-10-emoji-text.bin");
fixture_test!(turkish, "attributed-body-11-turkish.bin");
fixture_test!(mention_name, "attributed-body-12-mention-name.bin");
fixture_test!(mention_someone, "attributed-body-13-mention-someone.bin");
fixture_test!(original, "attributed-body-14-original.bin");
fixture_test!(quoted_reply, "attributed-body-15-quoted-reply.bin");
fixture_test!(edit, "attributed-body-16-edit.bin");
fixture_test!(forward, "attributed-body-17-forward.bin");
fixture_test!(photo_caption, "attributed-body-18-photo-caption.bin");
fixture_test!(sticker, "attributed-body-19-sticker.bin");
fixture_test!(expressive, "attributed-body-20-expressive.bin");
fixture_test!(hello, "attributed-body-21-hello.bin");
