#![no_main]

use libfuzzer_sys::fuzz_target;

// Table attachments keep their cells in gzip-compressed mergeable data (`ZMERGEABLEDATA1`).
fuzz_target!(|data: &[u8]| {
    let _ = apple_notes_protobuf::decode_table(data);
});
