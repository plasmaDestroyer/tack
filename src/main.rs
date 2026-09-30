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
use util::{
    detect_browsers, get_share_dir, normalize_url, resolve_app_browser, slugify, validate_name,
    validate_url,
};

fn main() -> Result<(), Box<dyn Error>> {
    let args = build_cli().get_matches();
    if args.get_flag("quiet") && args.get_flag("verbose") {
        build_cli()
            .error(
                clap::error::ErrorKind::ArgumentConflict,
                "Cannot use --quiet and --verbose together.",
            )
            .exit();
    }
    if args.subcommand().is_some() && (args.contains_id("app") || args.get_flag("interactive")) {
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
        Some(("list", command)) => list_apps(
            &get_share_dir()?,
            command.get_one::<String>("query").map(String::as_str),
            command.get_flag("json"),
            command.get_flag("names"),
        )?,
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
            import_apps(
                command.get_one::<String>("file").unwrap(),
                command.get_one::<String>("browser").map(String::as_str),
                dry_run,
            )?;
        }
        Some(("completions", command)) => {
            generate_completions(*command.get_one::<clap_complete::Shell>("shell").unwrap());
        }
        Some(("manpage", _)) => generate_manpage()?,
        None => {
            if let Some(mut app) = args.get_many::<String>("app") {
                install_app(
                    app.next().unwrap(),
                    app.next().unwrap(),
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

fn prompt(label: &str) -> Result<String, Box<dyn Error>> {
    print!("{}: ", label);
    io::stdout().flush()?;
    let mut input = String::new();
    if io::stdin().read_line(&mut input)? == 0 {
        return Err("Input ended. Interactive setup cancelled.".into());
    }
    Ok(input.trim().to_string())
}

fn prompt_choice(label: &str, count: usize, default: usize) -> Result<usize, Box<dyn Error>> {
    loop {
        let input = prompt(label)?;
        if input.is_empty() {
            return Ok(default);
        }
        if let Ok(choice) = input.parse::<usize>()
            && (1..=count).contains(&choice)
        {
            return Ok(choice);
        }
        output::warn(&format!("Enter a number between 1 and {count}."));
    }
}

fn run_interactive(dry_run: bool) -> Result<(), Box<dyn Error>> {
    if output::is_quiet() {
        return Err("Interactive setup needs visible choices. Remove --quiet.".into());
    }
    output::info("🔧 tack — interactive setup\n");

    // 1. URL — kick off icon fetch in a background thread
    let url = loop {
        let url = normalize_url(&prompt("Enter the URL")?);
        match validate_url(&url) {
            Ok(()) => break url,
            Err(message) => output::warn(&message),
        }
    };

    output::info("Fetching favicon in background...");
    let (tx, rx) = std::sync::mpsc::channel::<Option<(Vec<u8>, icon::ImageFormat)>>();
    let fetch_url = url.clone();
    std::thread::spawn(move || {
        let _ = tx.send(fetch_favicon_async(&fetch_url));
    });

    // 2. Browser (numbered list of detected browsers)
    let mut browsers = detect_browsers();
    if let Some(browser) = config::load_config().browser {
        let browser = resolve_app_browser(&browser)?;
        browsers.retain(|detected| detected != &browser);
        browsers.insert(0, browser);
    }
    let browser = if browsers.is_empty() {
        return Err("No supported browser found on PATH. Install a Chromium-based browser.".into());
    } else {
        output::info("\nAvailable browsers:");
        for (i, b) in browsers.iter().enumerate() {
            output::info(&format!("  [{}] {}", i + 1, b));
        }
        let choice = prompt_choice(
            "Pick a browser number (or press Enter for default)",
            browsers.len(),
            1,
        )?;
        Some(browsers[choice - 1].clone())
    };

    let share_dir = get_share_dir()?;
    let entries = manifest::load_manifest(&manifest::get_manifest_path(&share_dir))?;
    let name = loop {
        let name = prompt("Enter the app name")?;
        if let Err(message) = validate_name(&name) {
            output::warn(&message);
        } else if get_desktop_file_path(&slugify(&name), &share_dir).exists()
            || manifest::find_app_index(&entries, &name).is_some()
        {
            output::warn("That app is already installed. Choose another name or use tack update.");
        } else {
            break name;
        }
    };

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
    let icon_choice = prompt_choice(
        "Pick an option (or press Enter for default)",
        default_num,
        fetched_num.unwrap_or(default_num),
    )?;

    let icon_arg = if Some(icon_choice) == fetched_num {
        fetched.map(|(bytes, format)| IconSource::Bytes {
            bytes,
            format,
            user_supplied: false,
        })
    } else if icon_choice == custom_num {
        loop {
            let path = prompt("Enter the icon file path")?;
            match std::fs::read(&path) {
                Ok(bytes) => {
                    if let Some(format) = icon::detect_format(&bytes) {
                        break Some(IconSource::Bytes {
                            bytes,
                            format,
                            user_supplied: true,
                        });
                    }
                    output::warn("Unsupported icon format. Choose a PNG, SVG, or ICO file.");
                }
                Err(error) => output::warn(&format!("Cannot read icon: {error}")),
            }
        }
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
