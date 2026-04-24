use std::error::Error;

use crate::config::{get_config_path, load_config};
use crate::output;

pub fn handle_config(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.is_empty() {
        output::error("Usage: tack config show");
        output::error("       tack config set <key> <value>");
        std::process::exit(1);
    }

    match args[0].as_str() {
        "show" => show_config(),
        "set" => {
            if args.len() < 3 {
                output::error("Usage: tack config set <key> <value>");
                std::process::exit(1);
            }
            set_config(&args[1], &args[2])
        }
        _ => {
            output::error(&format!("Unknown config command: {}", args[0]));
            std::process::exit(1);
        }
    }
}

fn show_config() -> Result<(), Box<dyn Error>> {
    let config = load_config();
    let browser = config
        .browser
        .unwrap_or_else(|| "detect from PATH (fallback to chromium)".to_string());
    let categories = config.categories.unwrap_or_else(|| "Network;".to_string());

    output::info("Current configuration:");
    output::info(&format!("  browser = \"{}\"", browser));
    output::info(&format!("  categories = \"{}\"", categories));

    Ok(())
}

fn set_config(key: &str, value: &str) -> Result<(), Box<dyn Error>> {
    let config_path = get_config_path();

    // Read existing file
    let mut lines = Vec::new();
    if config_path.exists() {
        if let Ok(contents) = std::fs::read_to_string(&config_path) {
            lines = contents.lines().map(|s| s.to_string()).collect();
        }
    } else {
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
    }

    // Check if key exists
    let mut updated = false;
    for line in &mut lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('[') {
            continue;
        }
        if let Some((k, _)) = trimmed.split_once('=')
            && k.trim() == key
        {
            *line = format!("{} = \"{}\"", key, value);
            updated = true;
            break;
        }
    }

    if !updated {
        lines.push(format!("{} = \"{}\"", key, value));
    }

    std::fs::write(&config_path, lines.join("\n") + "\n")?;
    output::success("Config updated successfully.");
    Ok(())
}
