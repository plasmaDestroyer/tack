use std::process::Command;

#[test]
fn config_rejects_unknown_keys_and_unreadable_values() {
    let root = std::env::temp_dir().join(format!(
        "tack-config-validation-{}-{}",
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
            .env("XDG_CONFIG_HOME", &root)
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    };
    for args in [
        &["config", "set", "typo", "value", "--dry-run"][..],
        &["config", "set", "browser", "firefox"][..],
        &["config", "set", "categories", "Network\nExec=oops"][..],
        &["config", "set", "categories", "Network\";Exec=oops"][..],
    ] {
        assert!(!run(args).status.success(), "{args:?}");
    }
    let path = root.join("tack/config.toml");
    assert!(!path.exists());

    assert!(
        run(&["config", "set", "categories", "Network;Utility;"])
            .status
            .success()
    );
    let before = std::fs::read(&path).unwrap();
    assert!(!run(&["config", "set", "typo", "value"]).status.success());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let show = run(&["config", "show"]);
    assert!(show.status.success());
    assert!(String::from_utf8_lossy(&show.stdout).contains("detect from PATH"));

    std::fs::remove_dir_all(root).unwrap();
}
