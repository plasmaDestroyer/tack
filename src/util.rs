use std::error::Error;
use std::path::{Path, PathBuf};

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

pub fn validate_name(name: &str) -> Result<(), String> {
    if slugify(name).is_empty() {
        Err("App name must contain an ASCII letter or digit.".to_string())
    } else {
        Ok(())
    }
}

pub fn normalize_url(url: &str) -> String {
    if url.split_once("://").is_some_and(|(scheme, _)| {
        !scheme.is_empty()
            && scheme
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
    }) {
        String::from(url)
    } else {
        format!("https://{url}")
    }
}

/// Validate an HTTP(S) URL after normalization.
pub fn validate_url(url: &str) -> Result<(), String> {
    if url.chars().any(char::is_whitespace) {
        return Err(format!("Invalid URL: '{}' contains whitespace.", url));
    }

    let parsed = reqwest::Url::parse(url).map_err(|e| format!("Invalid URL '{}': {e}.", url))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(format!("Invalid URL '{}': use http:// or https://.", url));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| format!("Invalid URL '{}': missing host.", url))?;
    if host != "localhost"
        && !host.contains('.')
        && host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_err()
    {
        return Err(format!(
            "Invalid URL: host '{}' doesn't look like a valid domain (missing '.').",
            host
        ));
    }

    Ok(())
}

const KNOWN_BROWSERS: &[&str] = &[
    "chromium",
    "brave-browser",
    "brave",
    "google-chrome-stable",
    "google-chrome-beta",
    "google-chrome-dev",
    "ungoogled-chromium",
    "vivaldi",
    "microsoft-edge-stable",
];

pub fn validate_app_browser(browser: &str) -> Result<(), String> {
    let name = Path::new(browser)
        .file_name()
        .and_then(|name| name.to_str());
    if name.is_some_and(|name| {
        name.starts_with("firefox") || name == "zen" || name.starts_with("zen-browser")
    }) {
        return Err(format!(
            "Browser '{}' cannot open standalone app windows on Linux. Use a Chromium-based browser.",
            browser
        ));
    }
    Ok(())
}

pub fn detect_browser() -> Option<String> {
    detect_browsers().into_iter().next()
}

/// Return all installed browsers found on PATH, in preference order.
pub fn detect_browsers() -> Vec<String> {
    let mut found = Vec::new();

    if let Ok(path) = std::env::var("PATH") {
        for browser in KNOWN_BROWSERS.iter() {
            for dir in std::env::split_paths(&path) {
                let p = dir.join(browser);
                if p.is_file() {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = p.metadata()
                        && metadata.permissions().mode() & 0o111 != 0
                    {
                        found.push(browser.to_string());
                        break; // found this browser, move to next
                    }
                }
            }
        }
    }
    found
}

fn is_on_path(name: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        std::env::split_paths(&path).any(|dir| dir.join(name).is_file())
    } else {
        false
    }
}

/// Resolve a browser name to an executable present on PATH.
/// Tries an exact match first, then a prefix match against known browsers
/// (e.g. "brave" -> "brave-browser").
pub fn resolve_browser(name: &str) -> Option<String> {
    if is_on_path(name) {
        return Some(name.to_string());
    }
    KNOWN_BROWSERS
        .iter()
        .find(|b| b.starts_with(name) && is_on_path(b))
        .map(|b| b.to_string())
}
