use std::error::Error;
use std::path::Path;

use crate::desktop::get_desktop_file_path;
use crate::manifest::{get_manifest_path, load_manifest, save_manifest};
use crate::util::{get_share_dir, slugify};

pub fn remove_app(name: &str) -> Result<(), Box<dyn Error>> {
    let share_dir = get_share_dir()?;
    let slug = slugify(name);
    let manifest_path = get_manifest_path(&share_dir);

    let mut entries = load_manifest(&manifest_path)?;
    let position = entries.iter().position(|e| e.slug == slug);

    let entry = match position {
        Some(i) => entries.remove(i),
        None => {
            eprintln!("App '{}' is not installed.", name);
            std::process::exit(1);
        }
    };

    // Delete .desktop file
    let desktop_file_path = get_desktop_file_path(&slug, &share_dir);
    if desktop_file_path.exists() {
        std::fs::remove_file(&desktop_file_path)?;
        println!("Removed desktop file: {}", desktop_file_path.display());
    }

    // Delete icon only if it lives inside share_dir/icons/ (i.e. managed by tack)
    let icons_dir = share_dir.join("icons");
    let icon_path = Path::new(&entry.icon_path);
    if icon_path.exists() {
        if icon_path.starts_with(&icons_dir) {
            std::fs::remove_file(icon_path)?;
            println!("Removed icon: {}", entry.icon_path);
        } else {
            println!("Skipping user-supplied icon: {}", entry.icon_path);
        }
    }

    // Save updated manifest
    save_manifest(&manifest_path, &entries)?;
    println!("Manifest updated.");

    println!("✓ {} removed successfully!", name);
    Ok(())
}
