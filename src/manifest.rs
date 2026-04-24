use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::output;

#[derive(Serialize, Deserialize)]
pub struct AppEntry {
    pub name: String,
    pub slug: String,
    pub url: String,
    pub browser: String,
    pub icon_path: String,
    pub installed_at: u64,
    #[serde(default)]
    pub user_supplied_icon: bool,
}

pub fn get_manifest_path(share_dir: &Path) -> PathBuf {
    share_dir.join("tack").join("apps.json")
}

pub fn load_manifest(path: &Path) -> Result<Vec<AppEntry>, Box<dyn Error>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = std::fs::read_to_string(path)?;
    let entries: Vec<AppEntry> = serde_json::from_str(&contents)?;
    Ok(entries)
}

pub fn save_manifest(
    path: &Path,
    entries: &[AppEntry],
    dry_run: bool,
) -> Result<(), Box<dyn Error>> {
    if dry_run {
        output::dry_run(&format!("would update manifest: {}", path.display()));
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(entries)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn add_or_update_app(
    manifest_path: &Path,
    entry: AppEntry,
    dry_run: bool,
) -> Result<(), Box<dyn Error>> {
    let mut entries = load_manifest(manifest_path)?;
    if let Some(existing) = entries.iter_mut().find(|e| e.slug == entry.slug) {
        *existing = entry;
    } else {
        entries.push(entry);
    }
    save_manifest(manifest_path, &entries, dry_run)?;
    Ok(())
}
