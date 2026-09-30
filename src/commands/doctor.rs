use std::error::Error;

use crate::desktop::get_desktop_file_path;
use crate::icon::detect_format;
use crate::manifest::{find_app_index, get_manifest_path, load_manifest};
use crate::output;
use crate::util::{get_share_dir, resolve_app_browser, slugify, validate_url};

pub fn doctor_apps(name: Option<&str>) -> Result<(), Box<dyn Error>> {
    let share_dir = get_share_dir()?;
    let mut entries = load_manifest(&get_manifest_path(&share_dir))?;
    if let Some(name) = name {
        let index = find_app_index(&entries, name)
            .ok_or_else(|| format!("App '{name}' is not installed."))?;
        entries = vec![entries.remove(index)];
    }
    if entries.is_empty() {
        output::info("No apps installed to check.");
        return Ok(());
    }

    let mut failed = 0;
    for entry in &entries {
        let mut issues = Vec::new();
        let safe_slug = !entry.slug.is_empty() && slugify(&entry.slug) == entry.slug;
        if !safe_slug {
            issues.push("Invalid stored slug; restore this app from a backup.".to_string());
        }
        if let Err(error) = validate_url(&entry.url) {
            issues.push(error);
            if safe_slug {
                issues.push(format!("Fix with: tack update {} --url URL", entry.slug));
            }
        }
        if let Err(error) = resolve_app_browser(&entry.browser) {
            issues.push(error);
            if safe_slug {
                issues.push(format!(
                    "Fix with: tack update {} --browser BROWSER",
                    entry.slug
                ));
            }
        }
        if safe_slug && !get_desktop_file_path(&entry.slug, &share_dir).is_file() {
            issues.push(format!(
                "Launcher is missing. Repair with: tack update {}",
                entry.slug
            ));
        }
        let icon_error = match std::fs::read(&entry.icon_path) {
            Ok(bytes) if detect_format(&bytes).is_some() => None,
            Ok(_) => Some("Icon has an unsupported format.".to_string()),
            Err(error) => Some(format!("Cannot read icon: {error}")),
        };
        if let Some(error) = icon_error {
            issues.push(error);
            if safe_slug {
                issues.push(if entry.user_supplied_icon {
                    format!(
                        "Restore custom icon with: tack update {} --icon PATH (or --default-icon)",
                        entry.slug
                    )
                } else {
                    format!("Repair icon with: tack update {}", entry.slug)
                });
            }
        }
        if issues.is_empty() {
            output::info(&format!("OK: {}", entry.name.escape_debug()));
        } else {
            failed += 1;
            for issue in issues {
                output::error(&format!("{}: {}", entry.name.escape_debug(), issue));
            }
        }
    }
    if failed > 0 {
        return Err(format!("{failed} of {} app(s) need attention.", entries.len()).into());
    }
    output::success(&format!(
        "Checked {} app(s). No problems found.",
        entries.len()
    ));
    Ok(())
}
