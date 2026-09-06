#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(multistatus) = serde_caldav::parse_multistatus(data) {
        let _ = serde_caldav::multistatus_to_string(&multistatus);
    }
});
