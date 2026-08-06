use std::{env, fs, path::PathBuf};

const INFO_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key>
  <string>dev.apple-connector</string>
  <key>CFBundleName</key>
  <string>apple-connector</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>NSRemindersUsageDescription</key>
  <string>apple-connector creates, updates, and deletes reminders through the HTTP API.</string>
  <key>NSCalendarsUsageDescription</key>
  <string>apple-connector creates, updates, and deletes calendar events through the HTTP API.</string>
  <key>NSContactsUsageDescription</key>
  <string>apple-connector creates, updates, and deletes contacts and groups through the HTTP API.</string>
</dict>
</plist>
"#;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "macos") {
        let Ok(out_dir) = env::var("OUT_DIR") else {
            return;
        };
        let plist_path = PathBuf::from(out_dir).join("Info.plist");
        if fs::write(&plist_path, INFO_PLIST).is_err() {
            return;
        }
        println!(
            "cargo:rustc-link-arg=-Wl,-sectcreate,__TEXT,__info_plist,{}",
            plist_path.display()
        );
    }
}
