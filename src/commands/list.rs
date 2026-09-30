use std::error::Error;
use std::path::Path;

use crate::manifest::{get_manifest_path, load_manifest};
use crate::output;

pub fn list_apps(
    share_dir: &Path,
    query: Option<&str>,
    json: bool,
    names: bool,
) -> Result<(), Box<dyn Error>> {
    let mut entries = load_manifest(&get_manifest_path(share_dir))?;
    let total = entries.len();
    if let Some(query) = query {
        let query = query.to_lowercase();
        entries.retain(|entry| {
            entry.name.to_lowercase().contains(&query)
                || entry.url.to_lowercase().contains(&query)
                || entry.slug.contains(&query)
        });
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }
    if names {
        for entry in &entries {
            println!("{}", entry.name);
        }
        return Ok(());
    }
    if entries.is_empty() {
        output::info(if query.is_some() {
            "No apps match your search."
        } else {
            "No apps installed yet."
        });
        return Ok(());
    }

    let name_width = entries
        .iter()
        .map(|e| e.name.chars().count())
        .max()
        .unwrap_or(4)
        .clamp(4, 24);
    let url_width = entries
        .iter()
        .map(|e| e.url.chars().count())
        .max()
        .unwrap_or(3)
        .clamp(3, 48);
    output::info(&format!("{:<name_width$}  {:<url_width$}", "Name", "URL"));
    output::info(&format!(
        "{}  {}",
        "─".repeat(name_width),
        "─".repeat(url_width)
    ));
    for entry in &entries {
        output::info(&format!(
            "{:<name_width$}  {:<url_width$}",
            cell(&entry.name, name_width),
            cell(&entry.url, url_width),
        ));
        output::verbose(&format!(
            "Name: {}\n  Slug: {}\n  URL: {}\n  Browser: {}\n  Icon: {}",
            entry.name.escape_debug(),
            entry.slug.escape_debug(),
            entry.url.escape_debug(),
            entry.browser.escape_debug(),
            entry.icon_path.escape_debug(),
        ));
    }
    if query.is_some() {
        output::info(&format!("\n{} of {} app(s) match.", entries.len(), total));
    } else {
        output::info(&format!("\n{} app(s) installed.", total));
    }
    Ok(())
}

fn cell(value: &str, width: usize) -> String {
    let value: String = value
        .chars()
        .map(|c| if c.is_control() { '�' } else { c })
        .collect();
    if value.chars().count() <= width {
        value
    } else {
        format!("{}…", value.chars().take(width - 1).collect::<String>())
    }
}
