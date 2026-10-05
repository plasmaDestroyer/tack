use std::error::Error;
use std::fs;

use crate::commands::export::BackupEntry;
use crate::commands::install::{IconSource, install_app};
use crate::icon::detect_format;
use crate::output;

pub fn import_apps(
    input_path: &str,
    browser: Option<&str>,
    dry_run: bool,
) -> Result<(), Box<dyn Error>> {
    let content = fs::read_to_string(input_path)?;
    let entries: Vec<BackupEntry> = serde_json::from_str(&content)?;

    if entries.is_empty() {
        output::info("Manifest is empty. Nothing to import.");
        return Ok(());
    }

    let total = entries.len();
    let mut failed = 0;
    for backup in entries {
        let app = backup.app;
        output::info(&format!(
            "{} {}...",
            if dry_run { "Previewing" } else { "Importing" },
            app.name
        ));
        let result = (|| {
            let icon = match backup.icon_data {
                Some(bytes) => {
                    let format = detect_format(&bytes)
                        .ok_or("Unsupported embedded icon format (expected PNG, SVG, or ICO)")?;
                    Some(IconSource::Bytes {
                        bytes,
                        format,
                        user_supplied: app.user_supplied_icon,
                    })
                }
                None => None,
            };
            install_app(
                &app.url,
                &app.name,
                true,
                icon,
                Some(browser.unwrap_or(&app.browser).to_string()),
                // ponytail: web apps import as --app windows; the target browser may not have them installed.
                None,
                dry_run,
            )
        })();
        if let Err(e) = result {
            output::error(&format!("Failed to import {}: {}", app.name, e));
            failed += 1;
        }
    }

    if failed > 0 {
        return Err(format!("{} of {} apps failed to import.", failed, total).into());
    }
    if !dry_run {
        output::success("Import completed successfully!");
    }
    Ok(())
}
