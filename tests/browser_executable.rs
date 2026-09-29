use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[test]
fn browser_must_be_executable() {
    let root = std::env::temp_dir().join(format!(
        "tack-browser-permissions-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let browser = bin.join("chromium");
    std::fs::write(&browser, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&browser, std::fs::Permissions::from_mode(0o644)).unwrap();
    let icon = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/default.png");
    let run = |name: &str, explicit: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_tack"));
        command.args(["https://example.com", name, "--icon", icon]);
        if explicit {
            command.args(["--browser", "chromium"]);
        }
        command
            .env("PATH", &bin)
            .env("XDG_DATA_HOME", root.join("data"))
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    };

    assert!(!run("Auto", false).status.success());
    let explicit = run("Explicit", true);
    assert!(!explicit.status.success());
    assert!(String::from_utf8_lossy(&explicit.stderr).contains("not executable"));
    assert!(!root.join("data").exists());

    std::fs::set_permissions(&browser, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(run("Auto", false).status.success());
    assert!(run("Explicit", true).status.success());
    std::fs::remove_dir_all(root).unwrap();
}
