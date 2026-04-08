use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_ICON: &[u8] = include_bytes!("../assets/default.png");

fn fetch_favicon(url: &str) -> Option<Vec<u8>> {
    let favicon_url = format!("{}/favicon.ico", url.trim_end_matches('/'));

    let response = reqwest::blocking::get(&favicon_url).ok()?;

    if response.status().is_success() {
        response.bytes().ok().map(|b| b.to_vec())
    } else {
        None
    }
}

fn get_share_dir() -> Result<PathBuf, Box<dyn Error>> {
    if let Ok(home_directory) = std::env::var("XDG_DATA_HOME") {
        Ok(PathBuf::from(home_directory))
    } else if let Ok(home_directory) = std::env::var("HOME") {
        Ok(PathBuf::from(home_directory).join(".local/share/"))
    } else {
        Err("Could not find home directory!".into())
    }
}

fn slugify(name: &str) -> String {
    name.to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|x| !x.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn get_desktop_file_path(slug: &str, share_dir: &Path) -> PathBuf {
    share_dir
        .join("applications")
        .join(format!("{}.desktop", slug))
}

fn save_icon(
    slug: &str,
    bytes: &[u8],
    format: ImageFormat,
    share_dir: &Path,
) -> Result<PathBuf, Box<dyn Error>> {
    let icons_dir = share_dir.join("icons");
    std::fs::create_dir_all(&icons_dir)?;

    let extension = match format {
        ImageFormat::Png => "png",
        ImageFormat::Svg => "svg",
    };

    let icon_path = icons_dir.join(format!("{}.{}", slug, extension));
    std::fs::write(&icon_path, bytes)?;
    Ok(icon_path)
}

fn create_desktop_file(
    name: &str,
    icon_path: &Path,
    url: &str,
    desktop_file_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let applications_dir = &desktop_file_path
        .parent()
        .ok_or("Invalid desktop file path")?;
    std::fs::create_dir_all(applications_dir)?;

    let contents = format!(
        "[Desktop Entry]
Name={}
Exec=chromium --app={}
Icon={}
Type=Application
Terminal=false
Categories=Network;",
        name,
        url,
        icon_path.display()
    );

    std::fs::write(desktop_file_path, contents)?;

    Ok(())
}

fn normalize_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        String::from(url)
    } else {
        format!("https://{url}")
    }
}

enum ImageFormat {
    Png,
    Svg,
}

#[derive(Serialize, Deserialize)]
struct AppEntry {
    name: String,
    slug: String,
    url: String,
    browser: String,
    icon_path: String,
    installed_at: u64,
}

fn get_manifest_path(share_dir: &Path) -> PathBuf {
    share_dir.join("tack").join("apps.json")
}

fn load_manifest(path: &Path) -> Result<Vec<AppEntry>, Box<dyn Error>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = std::fs::read_to_string(path)?;
    let entries: Vec<AppEntry> = serde_json::from_str(&contents)?;
    Ok(entries)
}

fn save_manifest(path: &Path, entries: &[AppEntry]) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(entries)?;
    std::fs::write(path, json)?;
    Ok(())
}

fn add_or_update_app(manifest_path: &Path, entry: AppEntry) -> Result<(), Box<dyn Error>> {
    let mut entries = load_manifest(manifest_path)?;
    if let Some(existing) = entries.iter_mut().find(|e| e.slug == entry.slug) {
        *existing = entry;
    } else {
        entries.push(entry);
    }
    save_manifest(manifest_path, &entries)?;
    Ok(())
}

fn list_apps(share_dir: &Path) -> Result<(), Box<dyn Error>> {
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

fn remove_app(name: &str) -> Result<(), Box<dyn Error>> {
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

fn detect_format(bytes: &[u8]) -> Option<ImageFormat> {
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        Some(ImageFormat::Png)
    } else if bytes.starts_with(b"<svg") || bytes.starts_with(b"<?xml") {
        Some(ImageFormat::Svg)
    } else {
        None
    }
}

fn install_app(url: &str, name: &str) -> Result<(), Box<dyn Error>> {
    let url = normalize_url(url);

    let share_dir = get_share_dir()?;
    let slug = slugify(name);

    let desktop_file_path = get_desktop_file_path(&slug, &share_dir);
    if desktop_file_path.exists() {
        print!("{} is already installed. Overwrite? [y/N] ", name);
        Write::flush(&mut io::stdout()).ok();

        let mut buffer = String::new();
        let _ = io::stdin().read_line(&mut buffer);

        let input = buffer.trim();
        if input != "y" && input != "Y" {
            std::process::exit(0);
        }
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

    create_desktop_file(name, &icon_path, &url, &desktop_file_path)?;
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

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: tack <url> <name>");
        eprintln!("       tack list");
        eprintln!("       tack remove <name>");
        std::process::exit(1);
    }

    match args[1].as_str() {
        "list" => {
            let share_dir = get_share_dir()?;
            list_apps(&share_dir)?;
        }
        "remove" => {
            if args.len() < 3 {
                eprintln!("Usage: tack remove <name>");
                std::process::exit(1);
            }
            remove_app(&args[2])?;
        }
        _ => {
            if args.len() < 3 {
                eprintln!("Usage: tack <url> <name>");
                std::process::exit(1);
            }
            install_app(&args[1], &args[2])?;
        }
    }

    Ok(())
}
