use std::process::Command;

#[test]
fn subcommands_reject_extra_arguments_without_changing_apps() {
    let root = std::env::temp_dir().join(format!(
        "tack-command-args-{}-{}",
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
            "Demo",
            "--icon",
            icon,
            "--browser",
            "/bin/true"
        ])
        .status
        .success()
    );
    let manifest = root.join("data/tack/apps.json");
    let before = std::fs::read(&manifest).unwrap();
    let backup = root.join("backup.json");
    for args in [
        vec!["-i", "list"],
        vec!["--force", "list"],
        vec!["list", "query", "extra"],
        vec!["open", "Demo", "extra"],
        vec!["remove", "Demo", "extra"],
        vec!["update", "--all", "--name", "Oops"],
        vec!["export", backup.to_str().unwrap(), "extra"],
        vec!["export", "--bogus"],
        vec!["import", manifest.to_str().unwrap(), "extra"],
        vec!["config", "show", "extra"],
        vec!["config", "set", "browser", "chromium", "extra"],
        vec!["completions", "fish", "extra"],
        vec!["manpage", "extra"],
    ] {
        assert!(!run(&args).status.success(), "{args:?}");
    }
    assert_eq!(std::fs::read(&manifest).unwrap(), before);
    assert!(root.join("data/applications/demo.desktop").exists());
    assert!(!backup.exists());
    std::fs::remove_dir_all(root).unwrap();
}
