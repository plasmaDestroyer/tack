use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::desktop::{create_desktop_file, get_desktop_file_path};
use crate::icon::{
    DEFAULT_ICON, ImageFormat, detect_format, fetch_favicon, remove_replaced_icon, save_icon,
};
use crate::manifest::{
    AppEntry, add_or_update_app, find_app_index, get_manifest_path, load_manifest, lock_manifest,
};
use crate::output;
use crate::util::{
    detect_browser, get_share_dir, normalize_url, resolve_browser, slugify, validate_app_browser,
    validate_name, validate_url,
};

pub enum IconSource {
    File(String),
    Fetched(Vec<u8>, ImageFormat),
    Default,
}

pub fn install_app(
    url: &str,
    name: &str,
    force: bool,
    icon_arg: Option<IconSource>,
    browser_arg: Option<String>,
    dry_run: bool,
) -> Result<(), Box<dyn Error>> {
    let url = normalize_url(url);

    // Validate URL early (#23)
    validate_url(&url)?;
    validate_name(name)?;

    let share_dir = get_share_dir()?;
    let slug = slugify(name);

    let desktop_file_path = get_desktop_file_path(&slug, &share_dir);
    if !force && desktop_file_path.exists() {
        return Err(format!(
            "{} is already installed. Use `tack update {}` to modify it.",
            name, name
        )
        .into());
    }
    if dry_run {
        output::info(&format!("Previewing installation of {} from {}", name, url));
    } else {
        output::info(&format!("Installing {} from {}", name, url));
    }

    let config = crate::config::load_config();
    let user_supplied_browser = browser_arg.is_some() || config.browser.is_some();
    let browser_name = browser_arg
        .or(config.browser)
        .or_else(detect_browser)
        .ok_or("No supported browser found on PATH. Install a Chromium-based browser or use --browser.")?;
    validate_app_browser(&browser_name)?;

    let browser_name = if user_supplied_browser {
        match resolve_browser(&browser_name) {
            Some(resolved) => {
                if resolved != browser_name {
                    output::info(&format!(
                        "Resolved browser '{}' to '{}'",
                        browser_name, resolved
                    ));
                }
                resolved
            }
            None => {
                return Err(format!(
                    "Browser '{}' not found or not executable. Check its path and permissions.",
                    browser_name
                )
                .into());
            }
        }
    } else {
        browser_name
    };

    output::verbose(&format!("Browser: {}", browser_name));

    let manifest_path = get_manifest_path(&share_dir);
    let _lock = if dry_run {
        None
    } else {
        Some(lock_manifest(&manifest_path)?)
    };
    let entries = load_manifest(&manifest_path)?;
    if find_app_index(&entries, name).is_some_and(|index| !force || entries[index].slug != slug) {
        return Err(format!("App '{}' is already installed.", name).into());
    }
    let old_icon = entries
        .iter()
        .find(|entry| entry.slug == slug)
        .map(|entry| entry.icon_path.clone());

    let mut user_supplied_icon = false;

    let icon_path = match icon_arg {
        Some(IconSource::File(icon_path_str)) => {
            let icon_path_buf = std::path::PathBuf::from(&icon_path_str);
            if icon_path_buf.exists() {
                output::info(&format!("Using custom icon: {}", icon_path_str));
                let bytes = std::fs::read(&icon_path_buf)?;
                let format = detect_format(&bytes)
                    .ok_or("Unsupported icon format (expected PNG, SVG, or ICO)")?;
                output::verbose(&format!("Detected icon format: {:?}", format_name(&format)));
                user_supplied_icon = true;
                save_icon(&slug, &bytes, format, &share_dir, dry_run)?
            } else {
                return Err(format!("Icon file not found: {}", icon_path_str).into());
            }
        }
        Some(IconSource::Fetched(bytes, format)) => {
            save_icon(&slug, &bytes, format, &share_dir, dry_run)?
        }
        Some(IconSource::Default) => {
            save_icon(&slug, DEFAULT_ICON, ImageFormat::Png, &share_dir, dry_run)?
        }
        None => {
            let icons_dir = share_dir.join("icons");
            let cached_png = icons_dir.join(format!("{}.png", slug));
            let cached_svg = icons_dir.join(format!("{}.svg", slug));

            if cached_png.exists() {
                output::info(&format!("Found cached icon: {}", cached_png.display()));
                cached_png
            } else if cached_svg.exists() {
                output::info(&format!("Found cached icon: {}", cached_svg.display()));
                cached_svg
            } else {
                output::info(&format!("Fetching favicon for {}...", url));
                if let Some(bytes) = fetch_favicon(&url) {
                    if let Some(icon_format) = detect_format(&bytes) {
                        output::verbose(&format!(
                            "Favicon fetched — format: {}",
                            format_name(&icon_format)
                        ));
                        output::success("Favicon fetched successfully!");
                        save_icon(&slug, &bytes, icon_format, &share_dir, dry_run)?
                    } else {
                        output::warn("Wrong image format — installing with default icon.");
                        save_icon(&slug, DEFAULT_ICON, ImageFormat::Png, &share_dir, dry_run)?
                    }
                } else {
                    output::warn("Favicon not found — installing with default icon.");
                    save_icon(&slug, DEFAULT_ICON, ImageFormat::Png, &share_dir, dry_run)?
                }
            }
        }
    };

    output::verbose(&format!("Icon path: {}", icon_path.display()));

    create_desktop_file(
        name,
        &icon_path,
        &url,
        &browser_name,
        config.categories.as_deref(),
        &desktop_file_path,
        dry_run,
    )?;
    if !dry_run {
        output::info(&format!(
            "Desktop file created at: {}",
            desktop_file_path.display()
        ));
    }

    let entry = AppEntry {
        name: name.to_string(),
        slug: slug.clone(),
        url,
        browser: browser_name,
        icon_path: icon_path.display().to_string(),
        installed_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        user_supplied_icon,
    };
    add_or_update_app(&manifest_path, entry, dry_run)?;
    if let Some(old_icon) = old_icon {
        remove_replaced_icon(&old_icon, &icon_path, &slug, &share_dir, dry_run);
    }
    if !dry_run {
        output::info(&format!("Manifest updated at: {}", manifest_path.display()));
        output::success(&format!("✓ {} installed successfully!", name));
    }

    Ok(())
}

fn format_name(fmt: &ImageFormat) -> &'static str {
    match fmt {
        ImageFormat::Png => "PNG",
        ImageFormat::Svg => "SVG",
        ImageFormat::Ico => "ICO",
    }
}
