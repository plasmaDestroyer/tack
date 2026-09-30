use std::error::Error;

use crate::config::{get_config_path, load_config};
use crate::output;
use crate::util::validate_app_browser;

pub fn show_config() -> Result<(), Box<dyn Error>> {
    let config = load_config();
    let browser = config
        .browser
        .unwrap_or_else(|| "detect from PATH".to_string());
    let categories = config.categories.unwrap_or_else(|| "Network;".to_string());

    output::info("Current configuration:");
    output::info(&format!("  browser = \"{}\"", browser));
    output::info(&format!("  categories = \"{}\"", categories));

    Ok(())
}

pub fn set_config(key: &str, value: &str, dry_run: bool) -> Result<(), Box<dyn Error>> {
    if !matches!(key, "browser" | "categories") {
        return Err(format!("Unknown config key: {key}. Use browser or categories.").into());
    }
    if value.trim().is_empty()
        || value
            .chars()
            .any(|c| c.is_control() || matches!(c, '"' | '\'' | '\\'))
    {
        return Err("Config value must be nonempty and contain no quotes, backslashes, or control characters.".into());
    }
    if key == "browser" {
        validate_app_browser(value)?;
    }

    let config_path = get_config_path();
    if dry_run {
        output::dry_run(&format!("would set {} in {}", key, config_path.display()));
        return Ok(());
    }

    // Read existing file
    let mut lines = Vec::new();
    if config_path.exists() {
        let contents = std::fs::read_to_string(&config_path)?;
        lines = contents.lines().map(|s| s.to_string()).collect();
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
