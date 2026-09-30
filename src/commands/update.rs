use std::error::Error;
use std::path::PathBuf;

use crate::desktop::{create_desktop_file, get_desktop_file_path};
use crate::icon::{
    DEFAULT_ICON, ImageFormat, detect_format, fetch_favicon, remove_replaced_icon, save_icon,
};
use crate::manifest::{
    find_app_index, get_manifest_path, load_manifest, lock_manifest, save_manifest,
};
use crate::output;
use crate::util::{
    get_share_dir, normalize_url, validate_app_browser, validate_name, validate_url,
};

#[derive(Default)]
pub struct UpdateFlags {
    pub icon: Option<String>,
    pub url: Option<String>,
    pub browser: Option<String>,
    pub name: Option<String>,
}

pub fn update_app(
    current_name: &str,
    flags: UpdateFlags,
    dry_run: bool,
) -> Result<(), Box<dyn Error>> {
    let share_dir = get_share_dir()?;
    let manifest_path = get_manifest_path(&share_dir);
    let _lock = if dry_run {
        None
    } else {
        Some(lock_manifest(&manifest_path)?)
    };
    let mut entries = load_manifest(&manifest_path)?;

    let index = find_app_index(&entries, current_name)
        .ok_or_else(|| format!("App '{}' is not installed.", current_name))?;
    if let Some(new_name) = &flags.name {
        validate_name(new_name)?;
        if find_app_index(&entries, new_name).is_some_and(|other| other != index) {
            return Err(format!("App '{}' is already installed.", new_name).into());
        }
    }
    let entry = &mut entries[index];
    let slug = entry.slug.clone();
    let old_icon = entry.icon_path.clone();

    let has_overrides = flags.icon.is_some()
        || flags.url.is_some()
        || flags.browser.is_some()
        || flags.name.is_some();

    // Apply field overrides
    if let Some(new_url) = &flags.url {
        let normalized = normalize_url(new_url);
        // Validate new URL (#23)
        validate_url(&normalized)?;
        entry.url = normalized;
    }
    if let Some(new_browser) = &flags.browser {
        match crate::util::resolve_browser(new_browser) {
            Some(resolved) => {
                if resolved != *new_browser {
                    output::info(&format!(
                        "Resolved browser '{}' to '{}'",
                        new_browser, resolved
                    ));
                }
                entry.browser = resolved;
            }
            None => {
                output::error(&format!(
                    "Browser '{}' not found or not executable. Check its path and permissions.",
                    new_browser
                ));
                std::process::exit(1);
            }
        }
    }
    if let Some(new_name) = &flags.name {
        entry.name = new_name.clone();
    }
    validate_app_browser(&entry.browser)?;

    // Handle icon: explicit --icon flag, or repair-mode re-fetch
    if let Some(icon_arg) = &flags.icon {
        let icon_path_buf = PathBuf::from(icon_arg);
        if icon_path_buf.exists() {
            // User supplied a local file
            let bytes = std::fs::read(&icon_path_buf)?;
            let format = detect_format(&bytes)
                .ok_or("Unsupported icon format (expected PNG, SVG, or ICO)")?;
            let saved = save_icon(&slug, &bytes, format, &share_dir, dry_run)?;
            if !dry_run {
                output::info(&format!("Icon saved at: {}", saved.display()));
            }
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
            if !dry_run {
                output::info(&format!("Icon saved at: {}", icon_path.display()));
            }
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
    if !dry_run {
        output::info(&format!(
            "Desktop file updated at: {}",
            desktop_file_path.display()
        ));
    }

    let final_name = entry.name.clone();

    // Persist manifest
    save_manifest(&manifest_path, &entries, dry_run)?;
    remove_replaced_icon(&old_icon, &icon_path, &slug, &share_dir, dry_run);
    if !dry_run {
        output::info(&format!("Manifest updated at: {}", manifest_path.display()));
        output::success(&format!("✓ {} updated successfully!", final_name));
    }
    Ok(())
}

pub fn update_all_apps(dry_run: bool) -> Result<(), Box<dyn Error>> {
    let share_dir = get_share_dir()?;
    let manifest_path = get_manifest_path(&share_dir);
    let entries = load_manifest(&manifest_path)?;

    if entries.is_empty() {
        output::info("No apps installed to update.");
        return Ok(());
    }

    let mut failed = 0;
    for app in &entries {
        output::info(&format!(
            "{} {}...",
            if dry_run { "Previewing" } else { "Updating" },
            app.name
        ));
        if let Err(e) = update_app(&app.name, UpdateFlags::default(), dry_run) {
            output::error(&format!("Failed to update {}: {}", app.name, e));
            failed += 1;
        }
    }

    if failed > 0 {
        return Err(format!("{} of {} apps failed to update.", failed, entries.len()).into());
    }
    if !dry_run {
        output::success("All apps updated successfully!");
    }
    Ok(())
}
