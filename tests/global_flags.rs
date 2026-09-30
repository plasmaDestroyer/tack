use std::process::Command;

#[test]
fn global_flags_work_before_commands() {
    let root = std::env::temp_dir().join(format!(
        "tack-global-flags-{}-{}",
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

    let install = [
        "https://example.com",
        "Demo",
        "--icon",
        icon,
        "--browser",
        "/bin/true",
    ];
    assert!(
        run(&[
            "--dry-run",
            "https://example.com",
            "Demo",
            "--icon",
            icon,
            "--browser",
            "/bin/true",
        ])
        .status
        .success()
    );
    assert!(!root.join("data").exists());
    assert!(run(&install).status.success());
    let manifest = root.join("data/tack/apps.json");
    let before = std::fs::read(&manifest).unwrap();

    assert!(run(&["--dry-run", "update", "--all"]).status.success());
    assert!(run(&["update", "--dry-run", "--all"]).status.success());
    assert!(run(&["--dry-run", "remove", "Demo"]).status.success());
    assert_eq!(std::fs::read(&manifest).unwrap(), before);

    let completions = run(&["completions", "fish"]);
    let verbose_completions = run(&["--verbose", "completions", "fish"]);
    assert!(completions.status.success());
    assert!(verbose_completions.status.success());
    assert!(!completions.stdout.is_empty());
    assert_eq!(verbose_completions.stdout, completions.stdout);
    let quiet = run(&["--quiet", "list"]);
    assert!(quiet.status.success());
    assert!(quiet.stdout.is_empty());

    for args in [
        vec!["-q", "list", "-v"],
        vec!["--verbose", "list", "--quiet"],
        vec!["config", "-q", "set", "browser", "chromium", "-v"],
    ] {
        let output = run(&args);
        assert!(!output.status.success(), "{args:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("Cannot use --quiet and --verbose together")
        );
    }
    assert!(!root.join("config").exists());
    assert_eq!(std::fs::read(&manifest).unwrap(), before);

    std::fs::remove_dir_all(root).unwrap();
}
