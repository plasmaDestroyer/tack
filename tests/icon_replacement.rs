use std::process::Command;

#[test]
fn replacing_icon_format_removes_old_managed_file() {
    let root = std::env::temp_dir().join(format!(
        "tack-icon-replacement-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source_svg = root.join("source.svg");
    std::fs::write(
        &source_svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>",
    )
    .unwrap();
    let source_png = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/default.png");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_tack"))
            .args(args)
            .env("XDG_DATA_HOME", root.join("data"))
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    };

    let svg = source_svg.to_str().unwrap();
    assert!(
        run(&[
            "https://example.com",
            "Demo",
            "--icon",
            svg,
            "--browser",
            "/bin/true"
        ])
        .status
        .success()
    );
    let managed_svg = root.join("data/icons/demo.svg");
    let managed_png = root.join("data/icons/demo.png");
    assert!(managed_svg.exists());

    assert!(
        run(&["update", "Demo", "--icon", source_png, "--dry-run"])
            .status
            .success()
    );
    assert!(managed_svg.exists());
    assert!(!managed_png.exists());

    assert!(
        run(&["update", "Demo", "--icon", source_png])
            .status
            .success()
    );
    assert!(!managed_svg.exists());
    assert!(managed_png.exists());

    assert!(
        run(&[
            "https://example.com",
            "Demo",
            "--force",
            "--icon",
            svg,
            "--browser",
            "/bin/true"
        ])
        .status
        .success()
    );
    assert!(managed_svg.exists());
    assert!(!managed_png.exists());

    std::fs::remove_dir_all(root).unwrap();
}
