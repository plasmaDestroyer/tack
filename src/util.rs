use std::error::Error;
use std::path::PathBuf;

pub fn get_share_dir() -> Result<PathBuf, Box<dyn Error>> {
    if let Ok(home_directory) = std::env::var("XDG_DATA_HOME") {
        Ok(PathBuf::from(home_directory))
    } else if let Ok(home_directory) = std::env::var("HOME") {
        Ok(PathBuf::from(home_directory).join(".local/share/"))
    } else {
        Err("Could not find home directory!".into())
    }
}

pub fn slugify(name: &str) -> String {
    name.to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|x| !x.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub fn normalize_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        String::from(url)
    } else {
        format!("https://{url}")
    }
}

pub fn detect_browser() -> Option<String> {
    let browsers = [
        "chromium",
        "brave-browser",
        "google-chrome-stable",
        "ungoogled-chromium",
        "vivaldi",
        "microsoft-edge-stable",
        "zen-browser",
        "firefox",
    ];

    if let Ok(path) = std::env::var("PATH") {
        for browser in browsers.iter() {
            for dir in std::env::split_paths(&path) {
                let p = dir.join(browser);
                if p.is_file() {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = p.metadata()
                        && metadata.permissions().mode() & 0o111 != 0
                    {
                        return Some(browser.to_string());
                    }
                }
            }
        }
    }
    None
}
