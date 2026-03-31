use std::error::Error;
use std::path::{Path, PathBuf};

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

fn save_icon(slug: &str, bytes: &[u8], format: ImageFormat, share_dir: &Path) -> Option<PathBuf> {
    let icons_dir = share_dir.join("icons");
    std::fs::create_dir_all(&icons_dir)
        .unwrap_or_else(|_| panic!("Error making directory: {}!", icons_dir.display()));

    let extension = match format {
        ImageFormat::Png | ImageFormat::Unknown => "png",
        ImageFormat::Svg => "svg",
    };

    let icon_path = icons_dir.join(format!("{}.{}", slug, extension));
    match std::fs::write(&icon_path, bytes) {
        Ok(()) => Some(icon_path),
        Err(_) => {
            println!("Error writing image bytes to file!");
            None
        }
    }
}

fn create_desktop_file(name: &str, slug: &str, icon_path: &Path, url: &str, share_dir: &Path) -> Option<PathBuf> {
    let applications_dir = share_dir.join("applications");
    std::fs::create_dir_all(&applications_dir)
        .unwrap_or_else(|_| panic!("Error making directory: {}!", applications_dir.display()));

    let desktop_file_path = applications_dir.join(format!("{}.desktop", slug));
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
    match std::fs::write(&desktop_file_path, contents) {
        Ok(()) => Some(desktop_file_path),
        Err(_) => {
            println!("Error writing desktop file!");
            None
        },
    }
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
    Unknown,
}

fn detect_format(bytes: &[u8]) -> ImageFormat {
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        ImageFormat::Png
    } else if bytes.starts_with(b"<svg") || bytes.starts_with(b"<?xml") {
        ImageFormat::Svg
    } else {
        ImageFormat::Unknown
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: tack <url> <name>");
        std::process::exit(1);
    }

    let url = normalize_url(&args[1]);
    let name = &args[2];

    println!("Installing {} from {}", name, url);

    let share_dir = get_share_dir()?;
    let slug = slugify(name);

    println!("Fetching favicon for {}...", url);
    let icon_path = if let Some(bytes) = fetch_favicon(&url) {
        let icon_format = detect_format(&bytes);
        match icon_format {
            ImageFormat::Png | ImageFormat::Svg => {
                println!("Favicon fetched successfully!");
                save_icon(&slug, &bytes, icon_format, &share_dir)
            }
            ImageFormat::Unknown => {
                println!("Wrong image format ... Installing with Default icon.");
                save_icon(&slug, DEFAULT_ICON, icon_format, &share_dir)
            }
        }
    } else {
        println!("Favicon not found ... Installing with Default icon.");
        save_icon(&slug, DEFAULT_ICON, ImageFormat::Png, &share_dir)
    }
    .ok_or("Failed to save icon :(")?;

    println!("Icon saved at: {}", icon_path.display());

    let desktop_file_path = create_desktop_file(name, &slug, &icon_path, &url, &share_dir).ok_or("Failed to create Desktop file :(")?;
    println!("Desktop file created at: {}", desktop_file_path.display());

    println!("✓ {} installed successfully!", name);

    Ok(())
}
