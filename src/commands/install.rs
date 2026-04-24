use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::desktop::{create_desktop_file, get_desktop_file_path};
use crate::icon::{DEFAULT_ICON, ImageFormat, detect_format, fetch_favicon, save_icon};
use crate::manifest::{AppEntry, add_or_update_app, get_manifest_path};
use crate::output;
use crate::util::{
    check_online, detect_browser, get_share_dir, normalize_url, slugify, validate_url,
};

pub fn install_app(
    url: &str,
    name: &str,
    force: bool,
    icon_arg: Option<String>,
    browser_arg: Option<String>,
    dry_run: bool,
) -> Result<(), Box<dyn Error>> {
    let url = normalize_url(url);

    // Validate URL early (#23)
    if let Err(msg) = validate_url(&url) {
        output::error(&msg);
        std::process::exit(1);
    }

    let share_dir = get_share_dir()?;
    let slug = slugify(name);

    let desktop_file_path = get_desktop_file_path(&slug, &share_dir);
    if !force && desktop_file_path.exists() {
        output::error(&format!(
            "{} is already installed. Use `tack update {}` to modify it.",
            name, name
        ));
        std::process::exit(1);
    }

    output::info(&format!("Installing {} from {}", name, url));

    let mut user_supplied_icon = false;

    let icon_path = if let Some(icon_path_str) = icon_arg {
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
            output::error(&format!("Icon file not found: {}", icon_path_str));
            std::process::exit(1);
        }
    } else {
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
            // Offline check before network fetch (#24)
            if !check_online() {
                output::error("No network connection. Use --icon to install with a custom icon.");
                std::process::exit(1);
            }

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
    };

    output::verbose(&format!("Icon path: {}", icon_path.display()));

    let config = crate::config::load_config();
    let browser_name = browser_arg
        .or(config.browser)
        .or_else(detect_browser)
        .unwrap_or_else(|| String::from("chromium"));

    output::verbose(&format!("Browser: {}", browser_name));

    create_desktop_file(
        name,
        &icon_path,
        &url,
        &browser_name,
        config.categories.as_deref(),
        &desktop_file_path,
        dry_run,
    )?;
    output::info(&format!(
        "Desktop file created at: {}",
        desktop_file_path.display()
    ));

    let manifest_path = get_manifest_path(&share_dir);
    let entry = AppEntry {
        name: name.to_string(),
        slug,
        url,
        browser: browser_name,
        icon_path: icon_path.display().to_string(),
        installed_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        user_supplied_icon,
    };
    add_or_update_app(&manifest_path, entry, dry_run)?;
    output::info(&format!("Manifest updated at: {}", manifest_path.display()));

    output::success(&format!("✓ {} installed successfully!", name));

    Ok(())
}

fn format_name(fmt: &ImageFormat) -> &'static str {
    match fmt {
        ImageFormat::Png => "PNG",
        ImageFormat::Svg => "SVG",
        ImageFormat::Ico => "ICO",
    }
}
