use std::error::Error;
use std::path::Path;

use crate::desktop::get_desktop_file_path;
use crate::manifest::{find_app_index, get_manifest_path, load_manifest, save_manifest};
use crate::output;
use crate::util::get_share_dir;

pub fn remove_app(name: &str, dry_run: bool) -> Result<(), Box<dyn Error>> {
    let share_dir = get_share_dir()?;
    let manifest_path = get_manifest_path(&share_dir);

    let mut entries = load_manifest(&manifest_path)?;
    let position = find_app_index(&entries, name);

    let entry = match position {
        Some(i) => entries.remove(i),
        None => {
            output::error(&format!("App '{}' is not installed.", name));
            std::process::exit(1);
        }
    };

    // Delete .desktop file
    let desktop_file_path = get_desktop_file_path(&entry.slug, &share_dir);
    if desktop_file_path.exists() {
        if dry_run {
            output::dry_run(&format!("would remove: {}", desktop_file_path.display()));
        } else {
            std::fs::remove_file(&desktop_file_path)?;
            output::info(&format!(
                "Removed desktop file: {}",
                desktop_file_path.display()
            ));
        }
    }

    // Delete icon only if it lives inside share_dir/icons/ (i.e. managed by tack)
    let icons_dir = share_dir.join("icons");
    let icon_path = Path::new(&entry.icon_path);
    if icon_path.exists() {
        if icon_path.starts_with(&icons_dir) {
            if dry_run {
                output::dry_run(&format!("would remove: {}", entry.icon_path));
            } else {
                std::fs::remove_file(icon_path)?;
                output::info(&format!("Removed icon: {}", entry.icon_path));
            }
        } else {
            output::info(&format!("Skipping user-supplied icon: {}", entry.icon_path));
        }
    }

    save_manifest(&manifest_path, &entries, dry_run)?;
    if dry_run {
        return Ok(());
    }
    output::info("Manifest updated.");

    output::success(&format!("✓ {} removed successfully!", name));
    Ok(())
}
