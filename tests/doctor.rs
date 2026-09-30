use std::process::Command;

#[test]
fn doctor_reports_missing_files_and_browsers_without_writes() {
    let root = std::env::temp_dir().join(format!(
        "tack-doctor-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_tack"))
            .args(args)
            .env("XDG_DATA_HOME", &root)
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    };
    assert!(run(&["doctor"]).status.success());
    assert!(!root.exists());
    let icon = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/default.png");
    assert!(
        run(&[
            "https://example.com",
            "Demo",
            "--icon",
            icon,
            "--browser",
            "/bin/true"
        ])
        .status
        .success()
    );
    let manifest = root.join("tack/apps.json");
    let original = std::fs::read(&manifest).unwrap();
    assert!(run(&["doctor", "Demo"]).status.success());
    std::fs::remove_file(root.join("applications/demo.desktop")).unwrap();
    std::fs::remove_file(root.join("icons/demo.png")).unwrap();
    let missing = run(&["doctor", "--dry-run"]);
    assert!(!missing.status.success());
    let errors = String::from_utf8_lossy(&missing.stderr);
    assert!(errors.contains("Launcher is missing"));
    assert!(errors.contains("--icon PATH"));
    assert!(errors.contains("1 of 1 app(s) need attention"));
    assert_eq!(std::fs::read(&manifest).unwrap(), original);
    assert!(!root.join("applications/demo.desktop").exists());
    assert!(!root.join("icons/demo.png").exists());
    let mut entries: serde_json::Value = serde_json::from_slice(&original).unwrap();
    entries[0]["browser"] = "/missing/browser".into();
    std::fs::write(&manifest, serde_json::to_vec(&entries).unwrap()).unwrap();
    let broken = run(&["doctor", "Demo", "--quiet"]);
    assert!(!broken.status.success());
    assert!(String::from_utf8_lossy(&broken.stderr).contains("--browser BROWSER"));
    assert!(broken.stdout.is_empty());
    assert!(!run(&["doctor", "Unknown"]).status.success());
    std::fs::remove_dir_all(root).unwrap();
}
