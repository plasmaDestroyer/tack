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

fn save_icon(slug: &str, bytes: &[u8], share_dir: &Path) {
    let icons_dir = share_dir.join("icons");
    std::fs::create_dir_all(&icons_dir)
        .unwrap_or_else(|_| panic!("Error making directory: {}!", icons_dir.display()));

    let icon_path = icons_dir.join(format!("{}.png", slug));
    std::fs::write(icon_path, bytes).expect("Error writing image bytes to file!");
}

fn create_desktop_file(name: &str, slug: &str, url: &str, share_dir: &Path) {
    let applications_dir = share_dir.join("applications");
    std::fs::create_dir_all(&applications_dir)
        .unwrap_or_else(|_| panic!("Error making directory: {}!", applications_dir.display()));

    let icon_path = share_dir.join("icons").join(format!("{}.png", slug));
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
    std::fs::write(desktop_file_path, contents).expect("Error writing desktop file!");
}

fn normalize_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        String::from(url)
    } else {
        format!("https://{url}")
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
    if let Some(bytes) = fetch_favicon(&url) {
        println!("Favicon fetched successfully!");
        save_icon(&slug, &bytes, &share_dir);
        println!("Icon saved.")
    } else {
        println!("Favicon not found :( ... Installing with Default icon.");
        save_icon(&slug, DEFAULT_ICON, &share_dir);
    }

    create_desktop_file(name, &slug, &url, &share_dir);
    println!("Desktop file created.");

    println!("✓ {} installed successfully!", name);

    Ok(())
}
