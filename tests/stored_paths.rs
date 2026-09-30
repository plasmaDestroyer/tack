use std::process::Command;

#[test]
fn unsafe_stored_slugs_cannot_write_or_remove_outside_files() {
    let root = std::env::temp_dir().join(format!(
        "tack-stored-paths-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let data = root.join("data");
    for directory in ["tack", "applications", "icons"] {
        std::fs::create_dir_all(data.join(directory)).unwrap();
    }
    let outside_desktop = root.join("outside.desktop");
    let outside_icon = root.join("outside.png");
    std::fs::write(&outside_desktop, b"keep launcher").unwrap();
    std::fs::write(&outside_icon, b"keep image").unwrap();
    let icon = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/default.png");
    let manifest = data.join("tack/apps.json");
    let entries = serde_json::json!([{"name":"Demo","slug":"../../outside","url":"https://example.com","browser":"/bin/true","icon_path":icon,"installed_at":0,"user_supplied_icon":true}]);
    let before = serde_json::to_vec(&entries).unwrap();
    std::fs::write(&manifest, &before).unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_tack"))
            .args(args)
            .env("XDG_DATA_HOME", &data)
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    };
    for args in [
        vec!["remove", "Demo"],
        vec!["update", "Demo", "--icon", icon],
        vec!["update", "Demo", "--name", "Renamed"],
    ] {
        let output = run(&args);
        assert!(!output.status.success(), "{args:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("Invalid stored app slug"));
        assert_eq!(std::fs::read(&outside_desktop).unwrap(), b"keep launcher");
        assert_eq!(std::fs::read(&outside_icon).unwrap(), b"keep image");
        assert_eq!(std::fs::read(&manifest).unwrap(), before);
    }
    assert!(run(&["list", "--json"]).status.success());
    assert!(run(&["export", "--no-icons"]).status.success());
    assert!(!run(&["doctor"]).status.success());
    std::fs::remove_dir_all(root).unwrap();
}
