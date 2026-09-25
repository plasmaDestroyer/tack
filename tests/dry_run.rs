use std::path::PathBuf;
use std::process::{Command, Stdio};

#[test]
fn update_and_remove_dry_runs_preserve_app_files() {
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
            .env("XDG_CONFIG_HOME", data.join("config"))
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    };

    let preview = run(&[
        "https://example.com",
        "Preview",
        "--icon",
        icon.to_str().unwrap(),
        "--browser",
        "/bin/true",
        "--dry-run",
    ]);
    assert!(preview.status.success());
    assert!(!data.exists());

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
    let original_desktop = std::fs::read(&desktop).unwrap();
    let output = run(&["update", "Example", "--name", "Changed", "--dry-run"]);
    assert!(output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("updated successfully"));
    assert_eq!(std::fs::read(&desktop).unwrap(), original_desktop);
    assert_eq!(std::fs::read(&manifest).unwrap(), original_manifest);
    assert!(run(&["update", "--all", "--dry-run"]).status.success());
    assert_eq!(std::fs::read(&manifest).unwrap(), original_manifest);
    assert!(
        run(&["import", manifest.to_str().unwrap(), "--dry-run"])
            .status
            .success()
    );
    assert_eq!(std::fs::read(&manifest).unwrap(), original_manifest);

    let output = run(&["remove", "Example", "--dry-run"]);
    assert!(output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("removed successfully"));
    assert!(desktop.exists());
    assert!(icon.exists());
    assert_eq!(std::fs::read(&manifest).unwrap(), original_manifest);

    let backup = data.join("backup.json");
    assert!(
        run(&["export", backup.to_str().unwrap(), "--dry-run"])
            .status
            .success()
    );
    assert!(!backup.exists());
    assert!(
        run(&["config", "set", "browser", "chromium", "--dry-run"])
            .status
            .success()
    );
    assert!(!data.join("config/tack/config.toml").exists());
    let open = run(&["open", "Example", "--dry-run"]);
    assert!(open.status.success());
    assert!(String::from_utf8_lossy(&open.stdout).contains("would open"));

    std::fs::remove_dir_all(data).unwrap();
}

#[test]
fn interactive_dry_run_writes_nothing() {
    let root = std::env::temp_dir().join(format!(
        "tack-interactive-dry-run-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let bin = root.join("bin");
    let temp = root.join("temp");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::create_dir_all(&temp).unwrap();
    let browser = bin.join("chromium");
    std::fs::write(&browser, "#!/bin/sh\nexit 0\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&browser, std::fs::Permissions::from_mode(0o755)).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_tack"))
        .args(["-i", "--dry-run"])
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("TMPDIR", &temp)
        .env("PATH", &bin)
        .env("HTTPS_PROXY", "http://127.0.0.1:1")
        .env("NO_PROXY", "")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"localhost\n\nDry Demo\n\n")
        .unwrap();
    assert!(child.wait_with_output().unwrap().status.success());
    assert!(!root.join("data").exists());
    assert!(!root.join("config").exists());
    assert_eq!(std::fs::read_dir(&temp).unwrap().count(), 0);
    std::fs::remove_dir_all(root).unwrap();
}
