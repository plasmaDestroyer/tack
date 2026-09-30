use std::error::Error;
use std::fs;

use serde::{Deserialize, Serialize};

use crate::icon::detect_format;
use crate::manifest::{AppEntry, get_manifest_path, load_manifest};
use crate::output;
use crate::util::{atomic_write, get_share_dir};

#[derive(Serialize, Deserialize)]
pub struct BackupEntry {
    #[serde(flatten)]
    pub app: AppEntry,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_data: Option<Vec<u8>>,
}

pub fn export_apps(
    output_path: Option<&str>,
    include_icons: bool,
    dry_run: bool,
) -> Result<(), Box<dyn Error>> {
    let share_dir = get_share_dir()?;
    let manifest_path = get_manifest_path(&share_dir);
    let entries = load_manifest(&manifest_path)?;

    let mut backup = Vec::new();
    for app in entries {
        let icon_data = if include_icons {
            match fs::read(&app.icon_path) {
                Ok(bytes) => {
                    if detect_format(&bytes).is_none() {
                        return Err(format!(
                            "Unsupported icon for {}. Use --no-icons to export metadata only.",
                            app.name
                        )
                        .into());
                    }
                    Some(bytes)
                }
                Err(error) if app.user_supplied_icon => {
                    return Err(format!("Cannot back up custom icon for {}: {error}. Use --no-icons to export metadata only.", app.name).into());
                }
                Err(_) => None,
            }
        } else {
            None
        };
        backup.push(BackupEntry { app, icon_data });
    }
    let json = serde_json::to_string_pretty(&backup)?;

    if let Some(path) = output_path {
        if dry_run {
            output::dry_run(&format!("would export manifest to {}", path));
        } else {
            atomic_write(std::path::Path::new(path), json.as_bytes())?;
            output::success(&format!("Exported manifest to {}", path));
        }
    } else {
        println!("{}", json);
    }

    Ok(())
}
