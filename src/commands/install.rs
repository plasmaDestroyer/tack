use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::desktop::{create_desktop_file, get_desktop_file_path};
use crate::icon::{DEFAULT_ICON, ImageFormat, detect_format, fetch_favicon, save_icon};
use crate::manifest::{AppEntry, add_or_update_app, get_manifest_path};
use crate::util::{detect_browser, get_share_dir, normalize_url, slugify};

pub fn install_app(
    url: &str,
    name: &str,
    force: bool,
    icon_arg: Option<String>,
    browser_arg: Option<String>,
) -> Result<(), Box<dyn Error>> {
    let url = normalize_url(url);

    let share_dir = get_share_dir()?;
    let slug = slugify(name);

    let desktop_file_path = get_desktop_file_path(&slug, &share_dir);
    if !force && desktop_file_path.exists() {
        eprintln!(
            "{} is already installed. Use `tack update {}` to modify it.",
            name, name
        );
        std::process::exit(1);
    }

    println!("Installing {} from {}", name, url);

    let mut user_supplied_icon = false;

    let icon_path = if let Some(icon_path_str) = icon_arg {
        let icon_path_buf = std::path::PathBuf::from(&icon_path_str);
        if icon_path_buf.exists() {
            println!("Using custom icon: {}", icon_path_str);
            let bytes = std::fs::read(&icon_path_buf)?;
            let format = detect_format(&bytes)
                .ok_or("Unsupported icon format (expected PNG, SVG, or ICO)")?;
            user_supplied_icon = true;
            save_icon(&slug, &bytes, format, &share_dir)?
        } else {
            return Err(format!("Icon file not found: {}", icon_path_str).into());
        }
    } else {
        let icons_dir = share_dir.join("icons");
        let cached_png = icons_dir.join(format!("{}.png", slug));
        let cached_svg = icons_dir.join(format!("{}.svg", slug));

        if cached_png.exists() {
            println!("Found cached icon: {}", cached_png.display());
            cached_png
        } else if cached_svg.exists() {
            println!("Found cached icon: {}", cached_svg.display());
            cached_svg
        } else {
            println!("Fetching favicon for {}...", url);
            if let Some(bytes) = fetch_favicon(&url) {
                if let Some(icon_format) = detect_format(&bytes) {
                    println!("Favicon fetched successfully!");
                    save_icon(&slug, &bytes, icon_format, &share_dir)?
                } else {
                    println!("Wrong image format ... Installing with Default icon.");
                    save_icon(&slug, DEFAULT_ICON, ImageFormat::Png, &share_dir)?
                }
            } else {
                println!("Favicon not found ... Installing with Default icon.");
                save_icon(&slug, DEFAULT_ICON, ImageFormat::Png, &share_dir)?
            }
        }
    };

    println!("Icon saved at: {}", icon_path.display());

    let browser_name = browser_arg
        .or_else(detect_browser)
        .unwrap_or_else(|| String::from("chromium"));

    create_desktop_file(name, &icon_path, &url, &browser_name, &desktop_file_path)?;
    println!("Desktop file created at: {}", desktop_file_path.display());

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
    add_or_update_app(&manifest_path, entry)?;
    println!("Manifest updated at: {}", manifest_path.display());

    println!("✓ {} installed successfully!", name);

    Ok(())
}
