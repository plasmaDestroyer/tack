use std::process::Command;

#[test]
fn install_rejects_unknown_flags_and_extra_arguments() {
    let root = std::env::temp_dir().join(format!(
        "tack-install-args-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let icon = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/default.png");
    let run = |extra: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_tack"))
            .args([
                "https://example.com",
                "Demo",
                "--icon",
                icon,
                "--browser",
                "/bin/true",
            ])
            .args(extra)
            .env("XDG_DATA_HOME", root.join("data"))
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    };

    let typo = run(&["--dry-rnu"]);
    assert!(!typo.status.success());
    assert!(String::from_utf8_lossy(&typo.stderr).contains("unexpected argument '--dry-rnu'"));
    assert!(!run(&["extra"]).status.success());
    assert!(!root.join("data").exists());
    assert!(run(&["--dry-run"]).status.success());
    assert!(!root.join("data").exists());
}
