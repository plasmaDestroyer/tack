use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};

#[test]
fn guided_setup_retries_invalid_input_and_cancels_on_eof() {
    let root = std::env::temp_dir().join(format!(
        "tack-interactive-input-{}-{}",
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
    std::fs::set_permissions(&browser, std::fs::Permissions::from_mode(0o755)).unwrap();
    let run = |input: &[u8], dry_run: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_tack"));
        command.arg("-i");
        if dry_run {
            command.arg("--dry-run");
        }
        let mut child = command
            .env("PATH", &bin)
            .env("XDG_DATA_HOME", root.join("data"))
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("NO_COLOR", "1")
            .env("HTTPS_PROXY", "http://127.0.0.1:1")
            .env("HTTP_PROXY", "http://127.0.0.1:1")
            .env("NO_PROXY", "")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        child.wait_with_output().unwrap()
    };
    for input in [&b""[..], &b"localhost\n\nDemo\n"[..]] {
        let cancelled = run(input, false);
        assert!(!cancelled.status.success());
        assert!(String::from_utf8_lossy(&cancelled.stderr).contains("setup cancelled"));
        assert!(!root.join("data").exists());
    }
    let retried = run(b"invalid\nlocalhost\n9\n\n***\nDemo\n99\n\n", true);
    assert!(retried.status.success(), "{retried:?}");
    let output = String::from_utf8_lossy(&retried.stdout);
    assert!(output.contains("missing '.'"));
    assert!(output.contains("Enter a number between"));
    assert!(output.contains("ASCII letter or digit"));
    assert!(output.contains("would create"));
    assert!(!root.join("data").exists());
    std::fs::remove_dir_all(root).unwrap();
}
