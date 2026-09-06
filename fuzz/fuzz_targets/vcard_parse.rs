#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        // Parse every card, then re-serialize what parsed, so serializer
        // panics are in scope too.
        if let Ok(cards) = serde_vcard::parse_vcards(text) {
            for card in &cards {
                let _ = serde_vcard::to_string(card);
            }
        }
    }
});
