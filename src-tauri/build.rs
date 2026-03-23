fn main() {
    if let Ok(config_text) = std::fs::read_to_string("tauri.conf.json") {
        if let Ok(config_json) = serde_json::from_str::<serde_json::Value>(&config_text) {
            if let Some(bundle_version) = config_json
                .get("bundle")
                .and_then(|bundle| bundle.get("macOS"))
                .and_then(|macos| macos.get("bundleVersion"))
                .and_then(|value| value.as_str())
            {
                println!("cargo:rustc-env=LOCALPUSH_BUILD_NUMBER={bundle_version}");
            }
        }
    }

    tauri_build::build()
}
