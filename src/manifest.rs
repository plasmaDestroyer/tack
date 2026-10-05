use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

use crate::output;
use crate::util::{atomic_write, slugify};

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
    /// Browser-installed web app (`--app-id`) instead of a plain `--app=URL` window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

impl AppEntry {
    pub fn web_app(&self) -> Option<(&str, &str)> {
        Some((
            self.app_id.as_deref()?,
            self.profile.as_deref().unwrap_or("Default"),
        ))
    }
}

pub fn get_manifest_path(share_dir: &Path) -> PathBuf {
    share_dir.join("tack").join("apps.json")
}

pub fn lock_manifest(path: &Path) -> Result<File, Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.with_extension("lock"))?;
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(file)
}

pub fn load_manifest(path: &Path) -> Result<Vec<AppEntry>, Box<dyn Error>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = std::fs::read_to_string(path)?;
    let entries: Vec<AppEntry> = serde_json::from_str(&contents)?;
    Ok(entries)
}

pub fn find_app_index(entries: &[AppEntry], name: &str) -> Option<usize> {
    let slug = slugify(name);
    entries
        .iter()
        .position(|entry| entry.slug == slug || slugify(&entry.name) == slug)
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
    atomic_write(path, &serde_json::to_vec_pretty(entries)?)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn manifest_write_replaces_file_and_cleans_failed_temporary_file() {
        let root = std::env::temp_dir().join(format!(
            "tack-manifest-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("apps.json");
        std::fs::write(&path, "[]").unwrap();
        let entries = [AppEntry {
            name: "Demo".into(),
            slug: "demo".into(),
            url: "https://example.com".into(),
            browser: "chromium".into(),
            icon_path: "/tmp/demo.png".into(),
            installed_at: 0,
            user_supplied_icon: false,
            app_id: None,
            profile: None,
        }];
        save_manifest(&path, &entries, false).unwrap();
        assert_eq!(load_manifest(&path).unwrap()[0].name, "Demo");

        let blocked = root.join("blocked.json");
        std::fs::create_dir(&blocked).unwrap();
        assert!(save_manifest(&blocked, &entries, false).is_err());
        assert!(blocked.is_dir());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
        std::fs::remove_dir_all(root).unwrap();
    }
}
