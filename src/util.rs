use std::error::Error;
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

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

/// Validate a URL after normalization.
/// Checks: has scheme, has host, host has a dot, no spaces.
pub fn validate_url(url: &str) -> Result<(), String> {
    if url.contains(' ') {
        return Err(format!("Invalid URL: '{}' contains spaces.", url));
    }

    let after_scheme = if let Some(rest) = url.strip_prefix("https://") {
        rest
    } else if let Some(rest) = url.strip_prefix("http://") {
        rest
    } else {
        return Err(format!(
            "Invalid URL: '{}' is missing a scheme (http:// or https://).",
            url
        ));
    };

    // Extract host (everything before the first '/' or end)
    let host = after_scheme.split('/').next().unwrap_or("");

    // Strip port if present
    let host_no_port = if let Some(bracket_end) = host.find(']') {
        // IPv6: [::1]:port
        &host[..bracket_end + 1]
    } else {
        host.split(':').next().unwrap_or("")
    };

    if host_no_port.is_empty() {
        return Err(format!("Invalid URL: '{}' has an empty host.", url));
    }

    // Allow localhost without a dot
    if host_no_port != "localhost" && !host_no_port.contains('.') {
        return Err(format!(
            "Invalid URL: host '{}' doesn't look like a valid domain (missing '.').",
            host_no_port
        ));
    }

    Ok(())
}

/// Quick network connectivity check via TCP to Google DNS.
/// Returns true if online.
pub fn check_online() -> bool {
    let addr: SocketAddr = "8.8.8.8:53".parse().unwrap();
    TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok()
}

const KNOWN_BROWSERS: &[&str] = &[
    "chromium",
    "brave-browser",
    "google-chrome-stable",
    "ungoogled-chromium",
    "vivaldi",
    "microsoft-edge-stable",
    "zen-browser",
    "firefox",
];

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
