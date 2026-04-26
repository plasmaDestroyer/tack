use std::error::Error;
use std::fs;

use crate::commands::install::install_app;
use crate::manifest::AppEntry;
use crate::output;

pub fn import_apps(input_path: &str, dry_run: bool) -> Result<(), Box<dyn Error>> {
    let content = fs::read_to_string(input_path)?;
    let entries: Vec<AppEntry> = serde_json::from_str(&content)?;

    if entries.is_empty() {
        output::info("Manifest is empty. Nothing to import.");
        return Ok(());
    }

    for app in entries {
        output::info(&format!("Importing {}...", app.name));
        // Force install to recreate desktop files and re-fetch icons
        if let Err(e) = install_app(&app.url, &app.name, true, None, Some(app.browser), dry_run) {
            output::error(&format!("Failed to import {}: {}", app.name, e));
        }
    }

    output::success("Import completed successfully!");
    Ok(())
}
