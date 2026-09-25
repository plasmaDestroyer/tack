use std::process::Command;

#[test]
fn renamed_app_works_by_new_name_without_moving_files() {
    let root = std::env::temp_dir().join(format!(
        "tack-rename-{}-{}",
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

    assert!(
        run(&[
            "https://example.com",
            "Original",
            "--icon",
            icon,
            "--browser",
            "/bin/true"
        ])
        .status
        .success()
    );
    let desktop = root.join("data/applications/original.desktop");
    let managed_icon = root.join("data/icons/original.png");
    let manifest = root.join("data/tack/apps.json");
    assert!(desktop.exists());
    assert!(managed_icon.exists());

    assert!(
        run(&["update", "Original", "--name", "Renamed"])
            .status
            .success()
    );
    assert!(
        std::fs::read_to_string(&desktop)
            .unwrap()
            .contains("Name=Renamed")
    );
    assert!(
        std::fs::read_to_string(&manifest)
            .unwrap()
            .contains("\"slug\": \"original\"")
    );
    assert!(run(&["open", "Renamed", "--dry-run"]).status.success());
    assert!(
        run(&[
            "https://example.org",
            "Other",
            "--icon",
            icon,
            "--browser",
            "/bin/true",
        ])
        .status
        .success()
    );
    let before_collision = std::fs::read_to_string(&manifest).unwrap();
    assert!(
        !run(&["update", "Other", "--name", "Renamed"])
            .status
            .success()
    );
    assert_eq!(
        std::fs::read_to_string(&manifest).unwrap(),
        before_collision
    );
    assert!(run(&["update", "--all"]).status.success());
    assert!(
        !run(&[
            "https://example.org",
            "Renamed",
            "--icon",
            icon,
            "--browser",
            "/bin/true",
            "--force"
        ])
        .status
        .success()
    );
    assert!(run(&["remove", "Renamed"]).status.success());
    assert!(run(&["remove", "Other"]).status.success());
    assert!(!desktop.exists());
    assert!(!managed_icon.exists());
    assert_eq!(std::fs::read_to_string(&manifest).unwrap(), "[]");

    std::fs::remove_dir_all(root).unwrap();
}
