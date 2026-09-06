#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(event) = serde_icalendar::from_slice(data) {
        let _ = serde_icalendar::to_string(&event);
    }
});
