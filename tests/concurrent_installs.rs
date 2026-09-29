use std::process::{Command, Stdio};

#[test]
fn concurrent_installs_keep_every_manifest_entry() {
    let root = std::env::temp_dir().join(format!(
        "tack-concurrent-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let icon = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/default.png");
    let mut children = Vec::new();
    for index in 0..12 {
        children.push(
            Command::new(env!("CARGO_BIN_EXE_tack"))
                .args([
                    "https://example.com",
                    &format!("App {index}"),
                    "--icon",
                    icon,
                    "--browser",
                    "/bin/true",
                ])
                .env("XDG_DATA_HOME", root.join("data"))
                .env("XDG_CONFIG_HOME", root.join("config"))
                .env("NO_COLOR", "1")
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        );
    }
    for child in children {
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let manifest = std::fs::read(root.join("data/tack/apps.json")).unwrap();
    let apps: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    assert_eq!(apps.as_array().unwrap().len(), 12);
    assert_eq!(
        std::fs::read_dir(root.join("data/applications"))
            .unwrap()
            .count(),
        12
    );
    std::fs::remove_dir_all(root).unwrap();
}
