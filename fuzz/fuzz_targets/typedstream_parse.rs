#![no_main]

use libfuzzer_sys::fuzz_target;

// Messages `attributedBody` blobs come straight out of chat.db.
fuzz_target!(|data: &[u8]| {
    let _ = apple_typedstream::value_from_slice(data);
});
