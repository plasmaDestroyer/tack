mod commands;
mod desktop;
mod icon;
mod manifest;
mod util;

use std::error::Error;

use commands::install::install_app;
use commands::list::list_apps;
use commands::open::open_app;
use commands::remove::remove_app;
use commands::update::{parse_update_flags, update_app};
use util::get_share_dir;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: tack <url> <name> [--force]");
        eprintln!("       tack list");
        eprintln!("       tack open <name>");
        eprintln!("       tack remove <name>");
        eprintln!(
            "       tack update <name> [--name NAME] [--url URL] [--browser BROWSER] [--icon PATH]"
        );
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
        _ => {
            let force = args.contains(&"--force".to_string());
            let positional: Vec<&String> = args[1..].iter().filter(|a| *a != "--force").collect();

            if positional.len() < 2 {
                eprintln!("Usage: tack <url> <name> [--force]");
                std::process::exit(1);
            }
            install_app(positional[0], positional[1], force)?;
        }
    }

    Ok(())
}
