use std::process::Command;

#[test]
fn install_rejects_unusable_names_and_urls_before_writing() {
    let root = std::env::temp_dir().join(format!(
        "tack-validation-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let icon = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/default.png");
    let run = |url: &str, name: &str| {
        Command::new(env!("CARGO_BIN_EXE_tack"))
            .args([url, name, "--icon", icon, "--browser", "/bin/true"])
            .env("XDG_DATA_HOME", root.join("data"))
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    };

    for (url, name) in [
        ("https://example.com", "!!!"),
        ("https://bad?domain=example.com", "Bad Host"),
        ("https://example.com:bad", "Bad Port"),
        ("ftp://example.com", "Bad Scheme"),
    ] {
        assert!(!run(url, name).status.success(), "{url} {name}");
    }
    assert!(!root.join("data").exists());
    let invalid = run("ftp://example.com", "Bad Scheme");
    assert!(String::from_utf8_lossy(&invalid.stderr).starts_with("Error: Invalid URL"));

    let redirected = Command::new(env!("CARGO_BIN_EXE_tack"))
        .args([
            "https://example.com",
            "Preview",
            "--default-icon",
            "--browser",
            "/bin/true",
            "--dry-run",
        ])
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env_remove("NO_COLOR")
        .output()
        .unwrap();
    assert!(redirected.status.success());
    assert!(!redirected.stdout.contains(&0x1b));
    assert!(!redirected.stderr.contains(&0x1b));

    assert!(
        run("HTTPS://example.com/path?x=1", "Valid App")
            .status
            .success()
    );
    assert!(root.join("data/applications/valid-app.desktop").exists());
    assert!(
        run(
            "example.com/path?next=https://elsewhere.com",
            "Scheme In Query"
        )
        .status
        .success()
    );
    assert!(
        root.join("data/applications/scheme-in-query.desktop")
            .exists()
    );

    std::fs::remove_dir_all(root).unwrap();
}
