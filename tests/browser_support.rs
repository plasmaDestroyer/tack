use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[test]
fn unsupported_browsers_fail_before_writing_files() {
    let root = std::env::temp_dir().join(format!(
        "tack-browser-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    for name in ["firefox", "zen-browser"] {
        let path = bin.join(name);
        std::fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let icon = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/default.png");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_tack"))
            .args(args)
            .env("PATH", &bin)
            .env("XDG_DATA_HOME", root.join("data"))
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    };
    let install = ["https://example.com", "Example", "--icon", icon];
    let output = run(&install);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("No supported browser"));

    for browser in ["firefox", "zen-browser"] {
        let output = run(&[
            "https://example.com",
            "Example",
            "--icon",
            icon,
            "--browser",
            browser,
        ]);
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("cannot open standalone app windows")
        );
    }
    assert!(!root.join("data").exists());

    let chromium = bin.join("chromium");
    std::fs::write(&chromium, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(chromium, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(run(&install).status.success());
    let desktop_path = root.join("data/applications/example.desktop");
    let desktop = std::fs::read_to_string(&desktop_path).unwrap();
    assert!(desktop.contains("--app=https://example.com"));

    let manifest = root.join("data/tack/apps.json");
    let old = std::fs::read_to_string(&manifest).unwrap();
    let output = run(&["update", "Example", "--browser", "firefox"]);
    assert!(!output.status.success());
    assert_eq!(std::fs::read_to_string(&manifest).unwrap(), old);
    assert_eq!(std::fs::read_to_string(&desktop_path).unwrap(), desktop);

    std::fs::write(
        &manifest,
        old.replace("\"browser\": \"chromium\"", "\"browser\": \"firefox\""),
    )
    .unwrap();
    let output = run(&["open", "Example", "--dry-run"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot open standalone app windows"));

    std::fs::remove_dir_all(root).unwrap();
}
