use std::error::Error;
use std::path::Path;

use crate::manifest::{get_manifest_path, load_manifest};

pub fn list_apps(share_dir: &Path) -> Result<(), Box<dyn Error>> {
    let manifest_path = get_manifest_path(share_dir);
    let entries = load_manifest(&manifest_path)?;

    if entries.is_empty() {
        println!("No apps installed yet.");
        return Ok(());
    }

    // Calculate column widths
    let name_width = entries
        .iter()
        .map(|e| e.name.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let url_width = entries
        .iter()
        .map(|e| e.url.len())
        .max()
        .unwrap_or(3)
        .max(3);
    let browser_width = entries
        .iter()
        .map(|e| e.browser.len())
        .max()
        .unwrap_or(7)
        .max(7);
    let icon_width = entries
        .iter()
        .map(|e| e.icon_path.len())
        .max()
        .unwrap_or(4)
        .max(4);

    // Header
    println!(
        "{:<name_width$}  {:<url_width$}  {:<browser_width$}  {:<icon_width$}",
        "Name", "URL", "Browser", "Icon",
    );
    println!(
        "{:<name_width$}  {:<url_width$}  {:<browser_width$}  {:<icon_width$}",
        "─".repeat(name_width),
        "─".repeat(url_width),
        "─".repeat(browser_width),
        "─".repeat(icon_width),
    );

    // Rows
    for entry in &entries {
        println!(
            "{:<name_width$}  {:<url_width$}  {:<browser_width$}  {:<icon_width$}",
            entry.name, entry.url, entry.browser, entry.icon_path,
        );
    }

    println!("\n{} app(s) installed.", entries.len());
    Ok(())
}
