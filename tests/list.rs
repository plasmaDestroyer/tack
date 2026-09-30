use std::process::Command;

#[test]
fn listings_search_and_keep_machine_output_complete() {
    let root = std::env::temp_dir().join(format!(
        "tack-list-{}-{}",
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
            .env("XDG_DATA_HOME", &root)
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    };
    let name = "Music café with a very long display name";
    let url = format!("https://example.com/{}", "long/".repeat(20));
    for (name, url) in [(name, url.as_str()), ("Mail", "https://mail.example.com")] {
        assert!(
            run(&[url, name, "--icon", icon, "--browser", "/bin/true"])
                .status
                .success()
        );
    }
    let table = run(&["list"]);
    assert!(table.status.success());
    let table = String::from_utf8_lossy(&table.stdout);
    assert!(table.contains('…'));
    assert!(table.lines().all(|line| line.chars().count() <= 74));
    assert!(!table.contains(root.to_str().unwrap()));
    let json = run(&["list", "MUSIC", "--json", "--quiet"]);
    assert!(json.status.success());
    let entries: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(entries.as_array().unwrap().len(), 1);
    assert_eq!(entries[0]["name"], name);
    assert_eq!(entries[0]["url"], url);
    assert_eq!(
        run(&["list", "music", "--names"]).stdout,
        format!("{name}\n").as_bytes()
    );
    assert_eq!(run(&["list", "nonexistent", "--json"]).stdout, b"[]\n");
    let verbose = run(&["list", "mail", "--verbose"]);
    assert!(String::from_utf8_lossy(&verbose.stdout).contains("Browser: /bin/true"));
    assert!(!run(&["list", "--json", "--names"]).status.success());
    std::fs::remove_dir_all(root).unwrap();
}
