#![no_main]

use libfuzzer_sys::fuzz_target;

// Note bodies are gzip + protobuf, or a legacy binary plist.
fuzz_target!(|data: &[u8]| {
    let _ = apple_notes_protobuf::decode_note_body(data);
});
