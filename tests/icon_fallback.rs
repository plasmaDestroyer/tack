use std::process::Command;

#[test]
fn install_and_repair_use_default_icon_when_fetch_fails() {
    let root = std::env::temp_dir().join(format!(
        "tack-icon-fallback-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_tack"))
            .args(args)
            .env("XDG_DATA_HOME", root.join("data"))
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("NO_COLOR", "1")
            .env("HTTP_PROXY", "http://127.0.0.1:1")
            .env("HTTPS_PROXY", "http://127.0.0.1:1")
            .env("ALL_PROXY", "http://127.0.0.1:1")
            .env("NO_PROXY", "127.0.0.1")
            .env("http_proxy", "http://127.0.0.1:1")
            .env("https_proxy", "http://127.0.0.1:1")
            .env("all_proxy", "http://127.0.0.1:1")
            .env("no_proxy", "127.0.0.1")
            .output()
            .unwrap()
    };

    let install = run(&["http://127.0.0.1:1", "Offline", "--browser", "/bin/true"]);
    assert!(
        install.status.success(),
        "{}",
        String::from_utf8_lossy(&install.stderr)
    );
    let icon = root.join("data/icons/offline.png");
    let expected =
        std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/default.png")).unwrap();
    assert_eq!(std::fs::read(&icon).unwrap(), expected);

    let repair = run(&["update", "Offline"]);
    assert!(
        repair.status.success(),
        "{}",
        String::from_utf8_lossy(&repair.stderr)
    );
    assert_eq!(std::fs::read(&icon).unwrap(), expected);

    std::fs::remove_dir_all(root).unwrap();
}
