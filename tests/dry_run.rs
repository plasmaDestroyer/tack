use std::path::PathBuf;
use std::process::Command;

#[test]
fn remove_dry_run_preserves_app_files() {
    let data = std::env::temp_dir().join(format!(
        "tack-dry-run-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let icon = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/default.png");
    let tack = env!("CARGO_BIN_EXE_tack");
    let run = |args: &[&str]| {
        Command::new(tack)
            .args(args)
            .env("XDG_DATA_HOME", &data)
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    };

    assert!(
        run(&[
            "https://example.com",
            "Example",
            "--icon",
            icon.to_str().unwrap(),
            "--browser",
            "/bin/true"
        ])
        .status
        .success()
    );

    let desktop = data.join("applications/example.desktop");
    let icon = data.join("icons/example.png");
    let manifest = data.join("tack/apps.json");
    let original_manifest = std::fs::read(&manifest).unwrap();
    let output = run(&["remove", "Example", "--dry-run"]);
    assert!(output.status.success());
    assert!(desktop.exists());
    assert!(icon.exists());
    assert_eq!(std::fs::read(&manifest).unwrap(), original_manifest);
    std::fs::remove_dir_all(data).unwrap();
}
