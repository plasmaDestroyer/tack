use std::process::Command;

#[test]
fn portable_backups_restore_icons_without_the_source_files() {
    let root = std::env::temp_dir().join(format!(
        "tack-backup-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let svg_bytes = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><title>Custom</title></svg>";
    let svg = root.join("custom.svg");
    std::fs::write(&svg, svg_bytes).unwrap();
    let png = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/default.png");
    let run = |data: &str, args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_tack"))
            .args(args)
            .env("XDG_DATA_HOME", root.join(data))
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("NO_COLOR", "1")
            .env("HTTPS_PROXY", "http://127.0.0.1:1")
            .env("NO_PROXY", "")
            .output()
            .unwrap()
    };
    for (name, icon) in [("Custom", svg.to_str().unwrap()), ("Fetched", png)] {
        let output = run(
            "source",
            &[
                "https://example.com",
                name,
                "--icon",
                icon,
                "--browser",
                "/bin/true",
            ],
        );
        assert!(output.status.success(), "{output:?}");
    }
    let manifest_path = root.join("source/tack/apps.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    manifest[1]["user_supplied_icon"] = false.into();
    std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();

    let exported = run("source", &["export", "--quiet"]);
    assert!(exported.status.success());
    let backup = root.join("backup.json");
    std::fs::write(&backup, &exported.stdout).unwrap();
    let mut entries: serde_json::Value = serde_json::from_slice(&exported.stdout).unwrap();
    assert!(entries[0]["icon_data"].is_array());
    let metadata = run("source", &["export", "--no-icons"]);
    assert!(metadata.status.success());
    let metadata: serde_json::Value = serde_json::from_slice(&metadata.stdout).unwrap();
    assert!(metadata[0].get("icon_data").is_none());

    // A missing custom icon must not overwrite an existing backup with incomplete data.
    std::fs::remove_file(root.join("source/icons/custom.svg")).unwrap();
    assert!(
        !run("source", &["export", backup.to_str().unwrap()])
            .status
            .success()
    );
    assert_eq!(std::fs::read(&backup).unwrap(), exported.stdout);
    assert!(run("source", &["export", "--no-icons"]).status.success());
    std::fs::remove_dir_all(root.join("source")).unwrap();
    std::fs::remove_file(&svg).unwrap();

    let args = ["import", backup.to_str().unwrap()];
    assert!(
        run("destination", &[args[0], args[1], "--dry-run"])
            .status
            .success()
    );
    assert!(!root.join("destination").exists());
    let restored = run("destination", &args);
    assert!(restored.status.success(), "{restored:?}");
    assert_eq!(
        std::fs::read(root.join("destination/icons/custom.svg")).unwrap(),
        svg_bytes
    );
    assert_eq!(
        std::fs::read(root.join("destination/icons/fetched.png")).unwrap(),
        std::fs::read(png).unwrap()
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("destination/tack/apps.json")).unwrap())
            .unwrap();
    assert_eq!(manifest[0]["user_supplied_icon"], true);
    assert_eq!(manifest[1]["user_supplied_icon"], false);
    assert!(
        manifest[0]["icon_path"]
            .as_str()
            .unwrap()
            .contains("destination/icons")
    );
    assert!(run("destination", &["update", "Custom"]).status.success());
    assert_eq!(
        std::fs::read(root.join("destination/icons/custom.svg")).unwrap(),
        svg_bytes
    );

    // Bad embedded data fails its entry while valid entries still restore.
    entries[0]["icon_data"] = serde_json::json!([1, 2, 3]);
    std::fs::write(&backup, serde_json::to_vec(&entries).unwrap()).unwrap();
    let partial = run("partial", &args);
    assert!(!partial.status.success());
    assert!(String::from_utf8_lossy(&partial.stderr).contains("1 of 2 apps failed to import"));
    assert!(!root.join("partial/applications/custom.desktop").exists());
    assert!(root.join("partial/applications/fetched.desktop").exists());
    std::fs::remove_dir_all(root).unwrap();
}
