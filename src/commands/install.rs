use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::desktop::{create_desktop_file, get_desktop_file_path};
use crate::icon::{DEFAULT_ICON, ImageFormat, detect_format, fetch_favicon, save_icon};
use crate::manifest::{AppEntry, add_or_update_app, get_manifest_path};
use crate::util::{get_share_dir, normalize_url, slugify};

pub fn install_app(url: &str, name: &str, force: bool) -> Result<(), Box<dyn Error>> {
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

    println!("Fetching favicon for {}...", url);
    let icon_path = if let Some(bytes) = fetch_favicon(&url) {
        if let Some(icon_format) = detect_format(&bytes) {
            println!("Favicon fetched successfully!");
            save_icon(&slug, &bytes, icon_format, &share_dir)
        } else {
            println!("Wrong image format ... Installing with Default icon.");
            save_icon(&slug, DEFAULT_ICON, ImageFormat::Png, &share_dir)
        }
    } else {
        println!("Favicon not found ... Installing with Default icon.");
        save_icon(&slug, DEFAULT_ICON, ImageFormat::Png, &share_dir)
    }?;

    println!("Icon saved at: {}", icon_path.display());

    create_desktop_file(name, &icon_path, &url, "chromium", &desktop_file_path)?;
    println!("Desktop file created at: {}", desktop_file_path.display());

    let manifest_path = get_manifest_path(&share_dir);
    let entry = AppEntry {
        name: name.to_string(),
        slug,
        url,
        browser: String::from("chromium"),
        icon_path: icon_path.display().to_string(),
        installed_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    };
    add_or_update_app(&manifest_path, entry)?;
    println!("Manifest updated at: {}", manifest_path.display());

    println!("✓ {} installed successfully!", name);

    Ok(())
}
