use std::error::Error;

use crate::desktop::get_desktop_file_path;
use crate::icon::cleanup_app_icons;
use crate::manifest::{
    find_app_index, get_manifest_path, load_manifest, lock_manifest, save_manifest,
};
use crate::output;
use crate::util::get_share_dir;

pub fn remove_app(name: &str, dry_run: bool) -> Result<(), Box<dyn Error>> {
    let share_dir = get_share_dir()?;
    let manifest_path = get_manifest_path(&share_dir);
    let _lock = if dry_run {
        None
    } else {
        Some(lock_manifest(&manifest_path)?)
    };

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

    save_manifest(&manifest_path, &entries, dry_run)?;
    cleanup_app_icons(&entry.slug, None, &share_dir, dry_run);
    if dry_run {
        return Ok(());
    }
    output::info("Manifest updated.");

    output::success(&format!("✓ {} removed successfully!", name));
    Ok(())
}
