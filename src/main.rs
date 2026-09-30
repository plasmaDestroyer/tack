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

use commands::completions::{build_cli, generate_completions, generate_manpage};
use commands::config::{set_config, show_config};
use commands::export::export_apps;
use commands::import::import_apps;
use commands::install::{IconSource, install_app};
use commands::list::list_apps;
use commands::open::open_app;
use commands::remove::remove_app;
use commands::update::{UpdateFlags, update_all_apps, update_app};
use desktop::get_desktop_file_path;
use output::OutputMode;
use util::{detect_browsers, get_share_dir, normalize_url, slugify, validate_name, validate_url};

fn main() -> Result<(), Box<dyn Error>> {
    let args = build_cli().get_matches();
    if args.subcommand().is_some()
        && (args.get_one::<String>("url").is_some() || args.get_flag("interactive"))
    {
        build_cli()
            .error(
                clap::error::ErrorKind::ArgumentConflict,
                "Install and interactive arguments cannot be combined with subcommands.",
            )
            .exit();
    }
    let dry_run = args.get_flag("dry-run");
    let mode = if args.get_flag("quiet") {
        OutputMode::Quiet
    } else if args.get_flag("verbose") {
        OutputMode::Verbose
    } else {
        OutputMode::Normal
    };
    output::set_output_mode(mode);

    match args.subcommand() {
        Some(("list", _)) => list_apps(&get_share_dir()?)?,
        Some(("remove", command)) => {
            remove_app(command.get_one::<String>("name").unwrap(), dry_run)?;
        }
        Some(("open", command)) => {
            open_app(command.get_one::<String>("name").unwrap(), dry_run)?;
        }
        Some(("update", command)) => {
            if command.get_flag("all") {
                update_all_apps(dry_run)?;
            } else {
                let flags = UpdateFlags {
                    name: command.get_one::<String>("new-name").cloned(),
                    url: command.get_one::<String>("url").cloned(),
                    browser: command.get_one::<String>("browser").cloned(),
                    icon: command.get_one::<String>("icon").cloned(),
                };
                update_app(command.get_one::<String>("name").unwrap(), flags, dry_run)?;
            }
        }
        Some(("config", command)) => match command.subcommand() {
            Some(("show", _)) => show_config()?,
            Some(("set", values)) => set_config(
                values.get_one::<String>("key").unwrap(),
                values.get_one::<String>("value").unwrap(),
                dry_run,
            )?,
            _ => unreachable!("Clap requires a config subcommand"),
        },
        Some(("export", command)) => {
            export_apps(
                command.get_one::<String>("file").map(String::as_str),
                !command.get_flag("no-icons"),
                dry_run,
            )?;
        }
        Some(("import", command)) => {
            import_apps(command.get_one::<String>("file").unwrap(), dry_run)?;
        }
        Some(("completions", command)) => {
            generate_completions(*command.get_one::<clap_complete::Shell>("shell").unwrap());
        }
        Some(("manpage", _)) => generate_manpage()?,
        None => {
            if let Some(url) = args.get_one::<String>("url") {
                install_app(
                    url,
                    args.get_one::<String>("name").unwrap(),
                    args.get_flag("force"),
                    args.get_one::<String>("icon")
                        .cloned()
                        .map(IconSource::File),
                    args.get_one::<String>("browser").cloned(),
                    dry_run,
                )?;
            } else {
                run_interactive(dry_run)?;
            }
        }
        _ => unreachable!("Clap validates subcommands"),
    }
    Ok(())
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

    // 2. Browser (numbered list of detected browsers)
    let browsers = detect_browsers();
    let browser = if browsers.is_empty() {
        return Err("No supported browser found on PATH. Install a Chromium-based browser.".into());
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

    // 3. Name — typed while the favicon fetch runs in parallel
    let name = prompt("Enter the app name");
    if let Err(message) = validate_name(&name) {
        output::error(&message);
        std::process::exit(1);
    }

    // Fail fast if the app already exists
    let slug = slugify(&name);
    let share_dir = get_share_dir()?;
    if get_desktop_file_path(&slug, &share_dir).exists() {
        output::error(&format!(
            "{} is already installed. Use `tack update {}` to modify it.",
            name, name
        ));
        std::process::exit(1);
    }

    // Wait for the background fetch to finish
    let fetched = rx.recv().unwrap_or(None);

    // Preview uses a temporary file only during a real install.
    match &fetched {
        Some((bytes, format)) => {
            output::success(&format!("Icon fetched: {:?}", format));
            if !dry_run && !matches!(format, icon::ImageFormat::Svg) {
                let tmp_dir = std::env::temp_dir().join(format!("tack_{}", std::process::id()));
                let path = tmp_dir.join("icon.png");
                let out = if matches!(format, icon::ImageFormat::Ico) {
                    ico::ico_to_png(bytes).unwrap_or_else(|_| bytes.clone())
                } else {
                    bytes.clone()
                };
                if std::fs::create_dir_all(&tmp_dir).is_ok() && std::fs::write(&path, &out).is_ok()
                {
                    preview_icon(&path);
                    let _ = std::fs::remove_file(&path);
                    let _ = std::fs::remove_dir(&tmp_dir);
                }
            }
        }
        _ => {
            output::warn("Could not fetch an icon — will use default or custom.");
        }
    }

    // 4. Icon — verify the fetched one against custom/default
    output::info("\nIcon source:");
    let fetched_num = if fetched.is_some() { Some(1) } else { None };
    let custom_num = fetched_num.map(|n| n + 1).unwrap_or(1);
    let default_num = custom_num + 1;
    if let Some(n) = fetched_num {
        output::info(&format!("  [{}] Use fetched icon (default)", n));
    }
    output::info(&format!("  [{}] Custom local file", custom_num));
    output::info(&format!("  [{}] Use default icon", default_num));
    let icon_choice = prompt("Pick an option");

    let icon_arg =
        if fetched_num.is_some() && (icon_choice.trim().is_empty() || icon_choice.trim() == "1") {
            fetched.map(|(bytes, format)| IconSource::Bytes {
                bytes,
                format,
                user_supplied: false,
            })
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
            Some(IconSource::File(path))
        } else {
            Some(IconSource::Default)
        };

    output::info(""); // blank line before install output
    install_app(&url, &name, false, icon_arg, browser, dry_run)
}

/// Fetch the favicon off the main thread. Returns (bytes, format) on success.
fn fetch_favicon_async(url: &str) -> Option<(Vec<u8>, icon::ImageFormat)> {
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
