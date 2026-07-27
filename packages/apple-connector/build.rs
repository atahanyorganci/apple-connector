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
</dict>
</plist>
"#;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "macos") {
        let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
        let plist_path = out_dir.join("Info.plist");
        fs::write(&plist_path, INFO_PLIST).expect("write Info.plist");
        println!(
            "cargo:rustc-link-arg=-Wl,-sectcreate,__TEXT,__info_plist,{}",
            plist_path.display()
        );
    }
}
