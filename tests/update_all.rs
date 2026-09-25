use std::process::Command;

#[test]
fn update_all_reports_partial_failure() {
    let root = std::env::temp_dir().join(format!(
        "tack-update-all-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let icon = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/default.png");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_tack"))
            .args(args)
            .env("XDG_DATA_HOME", root.join("data"))
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    };

    for name in ["First", "Second"] {
        assert!(
            run(&[
                "https://example.com",
                name,
                "--icon",
                icon,
                "--browser",
                "/bin/true",
            ])
            .status
            .success()
        );
    }
    let manifest = root.join("data/tack/apps.json");
    let mut apps: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    apps[0]["browser"] = "firefox".into();
    std::fs::write(&manifest, serde_json::to_vec_pretty(&apps).unwrap()).unwrap();
    let second_desktop = root.join("data/applications/second.desktop");
    std::fs::remove_file(&second_desktop).unwrap();

    let output = run(&["update", "--all"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to update First"));
    assert!(stderr.contains("1 of 2 apps failed to update"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("All apps updated successfully"));
    assert!(second_desktop.exists());
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    assert_eq!(saved, apps);

    std::fs::remove_dir_all(root).unwrap();
}
