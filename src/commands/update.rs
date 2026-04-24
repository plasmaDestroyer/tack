use std::error::Error;
use std::path::PathBuf;

use crate::desktop::{create_desktop_file, get_desktop_file_path};
use crate::icon::{DEFAULT_ICON, ImageFormat, detect_format, fetch_favicon, save_icon};
use crate::manifest::{get_manifest_path, load_manifest, save_manifest};
use crate::output;
use crate::util::{check_online, get_share_dir, normalize_url, slugify, validate_url};

#[derive(Default)]
pub struct UpdateFlags {
    pub icon: Option<String>,
    pub url: Option<String>,
    pub browser: Option<String>,
    pub name: Option<String>,
}

pub fn parse_update_flags(args: &[String]) -> Result<UpdateFlags, Box<dyn Error>> {
    let mut flags = UpdateFlags::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--icon" => {
                let val = args.get(i + 1).ok_or("--icon requires a value")?;
                flags.icon = Some(val.clone());
                i += 2;
            }
            "--url" => {
                let val = args.get(i + 1).ok_or("--url requires a value")?;
                flags.url = Some(val.clone());
                i += 2;
            }
            "--browser" => {
                let val = args.get(i + 1).ok_or("--browser requires a value")?;
                flags.browser = Some(val.clone());
                i += 2;
            }
            "--name" => {
                let val = args.get(i + 1).ok_or("--name requires a value")?;
                flags.name = Some(val.clone());
                i += 2;
            }
            other => {
                return Err(format!("Unknown flag: {}", other).into());
            }
        }
    }
    Ok(flags)
}

pub fn update_app(
    current_name: &str,
    flags: UpdateFlags,
    dry_run: bool,
) -> Result<(), Box<dyn Error>> {
    let share_dir = get_share_dir()?;
    let slug = slugify(current_name);
    let manifest_path = get_manifest_path(&share_dir);
    let mut entries = load_manifest(&manifest_path)?;

    let entry = entries
        .iter_mut()
        .find(|e| e.slug == slug)
        .ok_or_else(|| format!("App '{}' is not installed.", current_name))?;

    let has_overrides = flags.icon.is_some()
        || flags.url.is_some()
        || flags.browser.is_some()
        || flags.name.is_some();

    // Apply field overrides
    if let Some(new_url) = &flags.url {
        let normalized = normalize_url(new_url);
        // Validate new URL (#23)
        if let Err(msg) = validate_url(&normalized) {
            output::error(&msg);
            std::process::exit(1);
        }
        entry.url = normalized;
    }
    if let Some(new_browser) = &flags.browser {
        entry.browser = new_browser.clone();
    }
    if let Some(new_name) = &flags.name {
        entry.name = new_name.clone();
    }

    // Handle icon: explicit --icon flag, or repair-mode re-fetch
    if let Some(icon_arg) = &flags.icon {
        let icon_path_buf = PathBuf::from(icon_arg);
        if icon_path_buf.exists() {
            // User supplied a local file
            let bytes = std::fs::read(&icon_path_buf)?;
            let format = detect_format(&bytes)
                .ok_or("Unsupported icon format (expected PNG, SVG, or ICO)")?;
            let saved = save_icon(&slug, &bytes, format, &share_dir, dry_run)?;
            output::info(&format!("Icon saved at: {}", saved.display()));
            entry.icon_path = saved.display().to_string();
            entry.user_supplied_icon = true;
        } else {
            output::error(&format!("Icon file not found: {}", icon_arg));
            std::process::exit(1);
        }
    } else if !has_overrides {
        if entry.user_supplied_icon {
            output::info("Repair mode: skipping favicon re-fetch because it is user-supplied.");
        } else {
            // Repair mode: re-fetch favicon from the app's URL
            output::info(&format!(
                "Repair mode: re-fetching favicon for {}...",
                entry.url
            ));

            // Offline check (#24)
            if !check_online() {
                output::error("No network connection. Use --icon to update with a custom icon.");
                std::process::exit(1);
            }

            let icon_path = if let Some(bytes) = fetch_favicon(&entry.url) {
                if let Some(icon_format) = detect_format(&bytes) {
                    output::success("Favicon fetched successfully!");
                    save_icon(&slug, &bytes, icon_format, &share_dir, dry_run)
                } else {
                    output::warn("Wrong image format — restoring default icon.");
                    save_icon(&slug, DEFAULT_ICON, ImageFormat::Png, &share_dir, dry_run)
                }
            } else {
                output::warn("Favicon not found — restoring default icon.");
                save_icon(&slug, DEFAULT_ICON, ImageFormat::Png, &share_dir, dry_run)
            }?;
            output::info(&format!("Icon saved at: {}", icon_path.display()));
            entry.icon_path = icon_path.display().to_string();
        }
    }

    // Rewrite .desktop file
    let config = crate::config::load_config();
    let desktop_file_path = get_desktop_file_path(&slug, &share_dir);
    let icon_path = PathBuf::from(&entry.icon_path);
    create_desktop_file(
        &entry.name,
        &icon_path,
        &entry.url,
        &entry.browser,
        config.categories.as_deref(),
        &desktop_file_path,
        dry_run,
    )?;
    output::info(&format!(
        "Desktop file updated at: {}",
        desktop_file_path.display()
    ));

    let final_name = entry.name.clone();

    // Persist manifest
    save_manifest(&manifest_path, &entries, dry_run)?;
    output::info(&format!("Manifest updated at: {}", manifest_path.display()));

    output::success(&format!("✓ {} updated successfully!", final_name));
    Ok(())
}
