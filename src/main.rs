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

use commands::config::handle_config;
use commands::export::export_apps;
use commands::import::import_apps;
use commands::install::install_app;
use commands::list::list_apps;
use commands::open::open_app;
use commands::remove::remove_app;
use commands::update::{parse_update_flags, update_all_apps, update_app};
use output::OutputMode;
use util::{detect_browsers, get_share_dir};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    // ── Global flags (parsed before subcommand routing) ──
    let mut dry_run = false;
    let mut interactive = false;
    let mut mode = OutputMode::Normal;

    // Quick scan for global flags
    for arg in &args[1..] {
        match arg.as_str() {
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
        print_usage();
        std::process::exit(1);
    }

    match args[1].as_str() {
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

    // 1. URL
    let url = prompt("Enter the URL");
    if url.is_empty() {
        output::error("URL cannot be empty.");
        std::process::exit(1);
    }

    // 2. Name
    let name = prompt("Enter the app name");
    if name.is_empty() {
        output::error("Name cannot be empty.");
        std::process::exit(1);
    }

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

    // 4. Icon
    output::info("\nIcon source:");
    output::info("  [1] Fetch from URL (default)");
    output::info("  [2] Custom local file");
    output::info("  [3] Use default icon");
    let icon_choice = prompt("Pick an option (1/2/3)");
    let icon_arg = match icon_choice.as_str() {
        "2" => {
            let path = prompt("Enter the icon file path");
            if path.is_empty() {
                output::error("Icon path cannot be empty.");
                std::process::exit(1);
            }
            Some(path)
        }
        "3" => {
            // We'll pass a sentinel — install_app will use DEFAULT_ICON if fetch returns None,
            // but we need a different approach. We'll set icon_arg to None and set a flag.
            // Actually, the simplest: pass the embedded default icon path. But that doesn't exist as a file.
            // Instead, we'll handle this: if user picks "3", we skip the fetch entirely.
            // Let's use a special marker that install_app recognizes.
            // Better approach: just don't pass an icon arg, and let the logic handle it.
            // But that would trigger a favicon fetch. We need to use --icon with a temp default.
            // Simplest: write the default icon to a temp location and pass that path.
            let share_dir = get_share_dir()?;
            let default_path = share_dir.join("icons").join("_default_tack.png");
            std::fs::create_dir_all(default_path.parent().unwrap())?;
            std::fs::write(&default_path, icon::DEFAULT_ICON)?;
            Some(default_path.display().to_string())
        }
        _ => None, // "1" or Enter — fetch from URL
    };

    output::info(""); // blank line before install output
    install_app(&url, &name, false, icon_arg, browser, dry_run)
}
