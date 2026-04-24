mod commands;
mod config;
mod desktop;
mod ico;
mod icon;
mod manifest;
mod util;

use std::error::Error;

use commands::config::handle_config;
use commands::install::install_app;
use commands::list::list_apps;
use commands::open::open_app;
use commands::remove::remove_app;
use commands::update::{parse_update_flags, update_app};
use util::get_share_dir;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: tack <url> <name> [--force] [--icon PATH] [--browser BROWSER]");
        eprintln!("       tack list");
        eprintln!("       tack open <name>");
        eprintln!("       tack remove <name>");
        eprintln!(
            "       tack update <name> [--name NAME] [--url URL] [--browser BROWSER] [--icon PATH]"
        );
        eprintln!("       tack config show");
        eprintln!("       tack config set <key> <value>");
        std::process::exit(1);
    }

    match args[1].as_str() {
        "list" => {
            let share_dir = get_share_dir()?;
            list_apps(&share_dir)?;
        }
        "remove" => {
            if args.len() < 3 {
                eprintln!("Usage: tack remove <name>");
                std::process::exit(1);
            }
            remove_app(&args[2])?;
        }
        "open" => {
            if args.len() < 3 {
                eprintln!("Usage: tack open <name>");
                std::process::exit(1);
            }
            open_app(&args[2])?;
        }
        "update" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: tack update <name> [--name NAME] [--url URL] [--browser BROWSER] [--icon PATH]"
                );
                std::process::exit(1);
            }
            let flags = parse_update_flags(&args[3..])?;
            update_app(&args[2], flags)?;
        }
        "config" => {
            handle_config(&args[2..])?;
        }
        _ => {
            let force = args.contains(&"--force".to_string());
            let mut icon_path = None;
            let mut browser = None;
            let mut positional = Vec::new();

            let mut i = 1;
            while i < args.len() {
                if args[i] == "--force" {
                    // skip
                } else if args[i] == "--icon" {
                    if i + 1 < args.len() {
                        icon_path = Some(args[i + 1].clone());
                        i += 1; // skip next
                    } else {
                        eprintln!("--icon requires a value");
                        std::process::exit(1);
                    }
                } else if args[i] == "--browser" {
                    if i + 1 < args.len() {
                        browser = Some(args[i + 1].clone());
                        i += 1; // skip next
                    } else {
                        eprintln!("--browser requires a value");
                        std::process::exit(1);
                    }
                } else {
                    positional.push(&args[i]);
                }
                i += 1;
            }

            if positional.len() < 2 {
                eprintln!("Usage: tack <url> <name> [--force] [--icon PATH] [--browser BROWSER]");
                std::process::exit(1);
            }
            install_app(positional[0], positional[1], force, icon_path, browser)?;
        }
    }

    Ok(())
}
