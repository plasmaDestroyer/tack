mod commands;
mod config;
mod desktop;
mod ico;
mod icon;
mod manifest;
mod output;
mod util;

use std::error::Error;
use std::io::{self, Write};

use commands::completions::{generate_completions, generate_manpage};
use commands::config::handle_config;
use commands::export::export_apps;
use commands::import::import_apps;
use commands::install::install_app;
use commands::list::list_apps;
use commands::open::open_app;
use commands::remove::remove_app;
use commands::update::{parse_update_flags, update_all_apps, update_app};
use output::OutputMode;
use util::{check_online, detect_browsers, get_share_dir, normalize_url, validate_url};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    // ── Global flags (parsed before subcommand routing) ──
    let mut dry_run = false;
    let mut interactive = false;
    let mut mode = OutputMode::Normal;

    // Quick scan for global flags
    for arg in &args[1..] {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("tack {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            "--dry-run" => dry_run = true,
            "--quiet" | "-q" => mode = OutputMode::Quiet,
            "--verbose" | "-v" => mode = OutputMode::Verbose,
            "-i" | "--interactive" => interactive = true,
            _ => {}
        }
    }

    // Quiet and verbose are mutually exclusive
    if args.contains(&"--quiet".to_string()) && args.contains(&"--verbose".to_string()) {
        output::error("Cannot use --quiet and --verbose together.");
        std::process::exit(1);
    }

    output::set_output_mode(mode);

    // ── Interactive mode (#19) ──
    if interactive {
        return run_interactive(dry_run);
    }

    if args.len() < 2 {
        return run_interactive(dry_run);
    }

    match args[1].as_str() {
        "help" => {
            print_usage();
            std::process::exit(0);
        }
        "list" => {
            let share_dir = get_share_dir()?;
            list_apps(&share_dir)?;
        }
        "remove" => {
            if args.len() < 3 {
                output::error("Usage: tack remove <name>");
                std::process::exit(1);
            }
            remove_app(&args[2])?;
        }
        "open" => {
            if args.len() < 3 {
                output::error("Usage: tack open <name>");
                std::process::exit(1);
            }
            open_app(&args[2])?;
        }
        "update" => {
            if args.len() < 3 {
                output::error(
                    "Usage: tack update <name> [--name NAME] [--url URL] [--browser BROWSER] [--icon PATH]\n       tack update --all",
                );
                std::process::exit(1);
            }
            if args[2] == "--all" {
                update_all_apps(dry_run)?;
            } else {
                let flags = parse_update_flags(&args[3..])?;
                update_app(&args[2], flags, dry_run)?;
            }
        }
        "config" => {
            handle_config(&args[2..])?;
        }
        "export" => {
            let output_path = args.get(2).map(|s| s.as_str());
            export_apps(output_path)?;
        }
        "import" => {
            if args.len() < 3 {
                output::error("Usage: tack import <file>");
                std::process::exit(1);
            }
            import_apps(&args[2], dry_run)?;
        }
        "completions" => {
            if args.len() < 3 {
                output::error("Usage: tack completions <bash|zsh|fish>");
                std::process::exit(1);
            }
            generate_completions(&args[2])?;
        }
        "manpage" => {
            generate_manpage()?;
        }
        _ => {
            let force = args.contains(&"--force".to_string());
            let mut icon_path = None;
            let mut browser = None;
            let mut positional = Vec::new();

            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--force" | "--dry-run" | "--quiet" | "-q" | "--verbose" | "-v" => {
                        // already handled or skip
                    }
                    "--icon" => {
                        if i + 1 < args.len() {
                            icon_path = Some(args[i + 1].clone());
                            i += 1; // skip next
                        } else {
                            output::error("--icon requires a value");
                            std::process::exit(1);
                        }
                    }
                    "--browser" => {
                        if i + 1 < args.len() {
                            browser = Some(args[i + 1].clone());
                            i += 1; // skip next
                        } else {
                            output::error("--browser requires a value");
                            std::process::exit(1);
                        }
                    }
                    _ => {
                        positional.push(&args[i]);
                    }
                }
                i += 1;
            }

            if positional.len() < 2 {
                output::error(
                    "Usage: tack <url> <name> [--force] [--icon PATH] [--browser BROWSER] [--dry-run] [--quiet] [--verbose]",
                );
                std::process::exit(1);
            }
            install_app(
                positional[0],
                positional[1],
                force,
                icon_path,
                browser,
                dry_run,
            )?;
        }
    }

    Ok(())
}

fn print_usage() {
    eprintln!(
        "Usage: tack <url> <name> [--force] [--icon PATH] [--browser BROWSER] [--dry-run] [--quiet] [--verbose]"
    );
    eprintln!("       tack -V, --version                (print version)");
    eprintln!("       tack -i                           (interactive mode)");
    eprintln!("       tack list");
    eprintln!("       tack open <name>");
    eprintln!("       tack remove <name>");
    eprintln!(
        "       tack update <name> [--name NAME] [--url URL] [--browser BROWSER] [--icon PATH] [--dry-run]"
    );
    eprintln!("       tack update --all                 (update all apps)");
    eprintln!("       tack export [file]");
    eprintln!("       tack import <file>");
    eprintln!("       tack completions <bash|zsh|fish>");
    eprintln!("       tack manpage");
    eprintln!("       tack config show");
    eprintln!("       tack config set <key> <value>");
}

// ── Interactive mode (#19) ──────────────────────────────────────────

fn prompt(label: &str) -> String {
    print!("{}: ", label);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn run_interactive(dry_run: bool) -> Result<(), Box<dyn Error>> {
    output::info("🔧 tack — interactive setup\n");

    // 1. URL — kick off icon fetch in a background thread
    let url = prompt("Enter the URL");
    if url.is_empty() {
        output::error("URL cannot be empty.");
        std::process::exit(1);
    }
    let url = normalize_url(&url);
    if let Err(msg) = validate_url(&url) {
        output::error(&msg);
        std::process::exit(1);
    }

    output::info("Fetching favicon in background...");
    let (tx, rx) = std::sync::mpsc::channel::<Option<(Vec<u8>, icon::ImageFormat)>>();
    let fetch_url = url.clone();
    std::thread::spawn(move || {
        let _ = tx.send(fetch_favicon_async(&fetch_url));
    });

    // 2. Name — typed while the favicon fetch runs in parallel
    let name = prompt("Enter the app name");
    if name.is_empty() {
        output::error("Name cannot be empty.");
        std::process::exit(1);
    }

    // Wait for the background fetch to finish
    let fetched = rx.recv().unwrap_or(None);

    // Save the fetched icon to a temp directory so we can preview it immediately.
    // Kept around for now — cleaned up later when we add temp management.
    let preview_path = match &fetched {
        Some((bytes, format)) => {
            let ext = match format {
                icon::ImageFormat::Png => "png",
                icon::ImageFormat::Svg => "svg",
                icon::ImageFormat::Ico => "png",
            };
            let tmp_dir =
                std::env::temp_dir().join(format!("tack_{}", std::process::id()));
            let path = tmp_dir.join(format!("icon.{}", ext));
            let out = if matches!(format, icon::ImageFormat::Ico) {
                ico::ico_to_png(bytes).unwrap_or_else(|_| bytes.clone())
            } else {
                bytes.clone()
            };
            if std::fs::create_dir_all(&tmp_dir).is_ok()
                && std::fs::write(&path, &out).is_ok()
            {
                output::success(&format!("Icon fetched: {:?}", format));
                preview_icon(&path);
                Some(path)
            } else {
                None
            }
        }
        _ => {
            output::warn("Could not fetch an icon — will use default or custom.");
            None
        }
    };

    // 3. Browser (numbered list of detected browsers)
    let browsers = detect_browsers();
    let browser = if browsers.is_empty() {
        output::warn("No browsers detected on PATH. Falling back to 'chromium'.");
        None
    } else {
        output::info("\nAvailable browsers:");
        for (i, b) in browsers.iter().enumerate() {
            output::info(&format!("  [{}] {}", i + 1, b));
        }
        let choice = prompt("Pick a browser number (or press Enter for default)");
        if choice.is_empty() {
            Some(browsers[0].clone())
        } else if let Ok(n) = choice.parse::<usize>() {
            if n >= 1 && n <= browsers.len() {
                Some(browsers[n - 1].clone())
            } else {
                output::warn("Invalid choice — using first detected browser.");
                Some(browsers[0].clone())
            }
        } else {
            output::warn("Invalid input — using first detected browser.");
            Some(browsers[0].clone())
        }
    };

    // 4. Icon — verify the fetched one against custom/default
    output::info("\nIcon source:");
    let fetched_num = if preview_path.is_some() { Some(1) } else { None };
    let custom_num = fetched_num.map(|n| n + 1).unwrap_or(1);
    let default_num = custom_num + 1;
    if let Some(n) = fetched_num {
        output::info(&format!("  [{}] Use fetched icon (default)", n));
    }
    output::info(&format!("  [{}] Custom local file", custom_num));
    output::info(&format!("  [{}] Use default icon", default_num));
    let icon_choice = prompt("Pick an option");

    let icon_arg = if fetched_num.is_some() && icon_choice.trim().is_empty() {
        // Enter = default = fetched icon
        preview_path.as_ref().map(|p| p.display().to_string())
    } else if icon_choice.trim() == custom_num.to_string() {
        let path = prompt("Enter the icon file path");
        if path.is_empty() {
            output::error("Icon path cannot be empty.");
            std::process::exit(1);
        }
        let path_buf = std::path::PathBuf::from(&path);
        if !path_buf.exists() {
            output::error(&format!("Icon file not found: {}", path));
            std::process::exit(1);
        }
        Some(path)
    } else {
        let share_dir = get_share_dir()?;
        let default_path = share_dir.join("icons").join("_default_tack.png");
        std::fs::create_dir_all(default_path.parent().unwrap())?;
        std::fs::write(&default_path, icon::DEFAULT_ICON)?;
        Some(default_path.display().to_string())
    };

    output::info(""); // blank line before install output
    install_app(&url, &name, false, icon_arg, browser, dry_run)
}

/// Fetch the favicon off the main thread. Returns (bytes, format) on success.
fn fetch_favicon_async(url: &str) -> Option<(Vec<u8>, icon::ImageFormat)> {
    if !check_online() {
        output::warn("No network connection.");
        return None;
    }
    let bytes = icon::fetch_favicon(url)?;
    let format = icon::detect_format(&bytes)?;
    Some((bytes, format))
}

/// Render an icon inline in the terminal via sixel (img2sixel).
/// Returns true if the preview was shown.
fn preview_icon(path: &std::path::Path) -> bool {
    if !path.extension().map(|e| e == "png").unwrap_or(false) {
        output::info("SVG icons can't be previewed in the terminal.");
        return false;
    }
    if let Ok(output) = std::process::Command::new("img2sixel").arg(path).output() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
        return true;
    }
    output::info("img2sixel not installed — skipping icon preview.");
    false
}
