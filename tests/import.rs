use std::process::Command;

#[test]
fn import_reports_partial_failure() {
    let root = std::env::temp_dir().join(format!(
        "tack-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let icons = root.join("data/icons");
    std::fs::create_dir_all(&icons).unwrap();
    std::fs::copy(
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/default.png"),
        icons.join("good.png"),
    )
    .unwrap();
    let backup = root.join("backup.json");
    let entries = serde_json::json!([
        {"name":"Bad URL","slug":"bad-url","url":"https://invalid","browser":"/bin/true","icon_path":"","installed_at":0},
        {"name":"Bad Browser","slug":"bad-browser","url":"https://example.com","browser":"/nonexistent/tack-browser","icon_path":"","installed_at":0},
        {"name":"Good","slug":"good","url":"https://example.com","browser":"/bin/true","icon_path":"","installed_at":0}
    ]);
    std::fs::write(&backup, serde_json::to_vec(&entries).unwrap()).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_tack"))
        .args(["import", backup.to_str().unwrap()])
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to import Bad URL"));
    assert!(stderr.contains("Failed to import Bad Browser"));
    assert!(stderr.contains("2 of 3 apps failed to import"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Import completed successfully"));
    assert!(root.join("data/applications/good.desktop").exists());
    assert!(!root.join("data/applications/bad-url.desktop").exists());
    assert!(!root.join("data/applications/bad-browser.desktop").exists());

    std::fs::remove_dir_all(root).unwrap();
}
