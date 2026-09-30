use std::error::Error;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let target = match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => path.canonicalize()?,
        Ok(_) => path.to_path_buf(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => path.to_path_buf(),
        Err(error) => return Err(error.into()),
    };
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = target.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| {
        match target.metadata() {
            Ok(metadata) => file.set_permissions(metadata.permissions())?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        file.write_all(bytes)?;
        file.sync_all()?;
        std::fs::rename(&temporary, &target)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result?;
    Ok(())
}

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

pub fn validate_slug(slug: &str) -> Result<(), String> {
    if slug.is_empty() || slugify(slug) != slug {
        Err(format!(
            "Invalid stored app slug '{}'. Restore this app from a backup.",
            slug.escape_debug()
        ))
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
                if is_executable(&dir.join(browser)) {
                    found.push(browser.to_string());
                    break; // found this browser, move to next
                }
            }
        }
    }
    found
}

fn is_executable(path: &Path) -> bool {
    path.is_file()
        && path
            .metadata()
            .is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
}

fn is_on_path(name: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        std::env::split_paths(&path).any(|dir| is_executable(&dir.join(name)))
    } else {
        false
    }
}

/// Resolve a browser name to an executable present on PATH.
/// Tries an exact match first, then a prefix match against known browsers
/// (e.g. "brave" -> "brave-browser").
pub fn resolve_browser(name: &str) -> Option<String> {
    if name.trim().is_empty() {
        return None;
    }
    if is_on_path(name) {
        return Some(name.to_string());
    }
    KNOWN_BROWSERS
        .iter()
        .find(|b| b.starts_with(name) && is_on_path(b))
        .map(|b| b.to_string())
}

pub fn resolve_app_browser(name: &str) -> Result<String, String> {
    validate_app_browser(name)?;
    let browser = resolve_browser(name).ok_or_else(|| {
        format!("Browser '{name}' not found or not executable. Check its path and permissions.")
    })?;
    validate_app_browser(&browser)?;
    Ok(browser)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_writes_keep_permissions_symlinks_and_clean_failed_files() {
        let root = std::env::temp_dir().join(format!(
            "tack-atomic-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let target = root.join("config.toml");
        std::fs::write(&target, b"old").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600)).unwrap();
        let link = root.join("config-link.toml");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        atomic_write(&link, b"new").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
        assert_eq!(
            target.metadata().unwrap().permissions().mode() & 0o777,
            0o600
        );
        let blocked = root.join("blocked");
        std::fs::create_dir(&blocked).unwrap();
        assert!(atomic_write(&blocked, b"data").is_err());
        assert!(blocked.is_dir());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 3);
        std::fs::remove_dir_all(root).unwrap();
    }
}
