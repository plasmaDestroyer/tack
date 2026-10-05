use std::collections::HashSet;
use std::error::Error;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::desktop::{create_desktop_file, get_desktop_file_path};
use crate::icon::{
    DEFAULT_ICON, ImageFormat, cleanup_app_icons, detect_format, fetch_favicon, save_icon,
};
use crate::manifest::{
    AppEntry, add_or_update_app, find_app_index, get_manifest_path, load_manifest, lock_manifest,
};
use crate::output;
use crate::util::{
    detect_browser, get_share_dir, normalize_url, resolve_app_browser, slugify, validate_name,
    validate_url,
};

pub enum IconSource {
    File(String),
    Bytes {
        bytes: Vec<u8>,
        format: ImageFormat,
        user_supplied: bool,
    },
    Default,
}

pub fn install_app(
    url: &str,
    name: &str,
    force: bool,
    icon_arg: Option<IconSource>,
    browser_arg: Option<String>,
    // Some("") waits for the browser's Install app; Some(id) adopts an installed web app.
    web_app: Option<String>,
    dry_run: bool,
) -> Result<(), Box<dyn Error>> {
    let url = normalize_url(url);

    // Validate URL early (#23)
    validate_url(&url)?;
    validate_name(name)?;

    let share_dir = get_share_dir()?;
    let slug = slugify(name);

    let desktop_file_path = get_desktop_file_path(&slug, &share_dir)?;
    if !force && desktop_file_path.exists() {
        return Err(format!(
            "{} is already installed. Use `tack update {}` to modify it.",
            name, name
        )
        .into());
    }
    if dry_run {
        output::info(&format!("Previewing installation of {} from {}", name, url));
    } else {
        output::info(&format!("Installing {} from {}", name, url));
    }

    let config = crate::config::load_config();
    let browser_name = browser_arg
        .or(config.browser)
        .or_else(detect_browser)
        .ok_or("No supported browser found on PATH. Install a Chromium-based browser or use --browser.")?;
    let resolved = resolve_app_browser(&browser_name)?;
    if resolved != browser_name {
        output::info(&format!(
            "Resolved browser '{}' to '{}'",
            browser_name, resolved
        ));
    }
    let browser_name = resolved;

    output::verbose(&format!("Browser: {}", browser_name));

    let manifest_path = get_manifest_path(&share_dir);
    let _lock = if dry_run {
        None
    } else {
        Some(lock_manifest(&manifest_path)?)
    };
    let entries = load_manifest(&manifest_path)?;
    if find_app_index(&entries, name).is_some_and(|index| !force || entries[index].slug != slug) {
        return Err(format!("App '{}' is already installed.", name).into());
    }
    let existing = entries.iter().find(|entry| entry.slug == slug);

    let (app_id, profile) = match web_app {
        Some(id) => match adopt_web_app(&browser_name, &url, &id, &share_dir, dry_run)? {
            Some((id, profile)) => (Some(id), Some(profile)),
            None => return Ok(()),
        },
        None => (None, None),
    };
    let web_app = app_id.as_deref().zip(profile.as_deref());

    let mut user_supplied_icon = false;

    let icon_path = match icon_arg {
        Some(IconSource::File(icon_path_str)) => {
            let icon_path_buf = std::path::PathBuf::from(&icon_path_str);
            if icon_path_buf.exists() {
                output::info(&format!("Using custom icon: {}", icon_path_str));
                let bytes = std::fs::read(&icon_path_buf)?;
                let format = detect_format(&bytes)
                    .ok_or("Unsupported icon format (expected PNG, SVG, or ICO)")?;
                output::verbose(&format!("Detected icon format: {:?}", format_name(&format)));
                user_supplied_icon = true;
                save_icon(&slug, &bytes, format, &share_dir, dry_run)?
            } else {
                return Err(format!("Icon file not found: {}", icon_path_str).into());
            }
        }
        Some(IconSource::Bytes {
            bytes,
            format,
            user_supplied,
        }) => {
            user_supplied_icon = user_supplied;
            save_icon(&slug, &bytes, format, &share_dir, dry_run)?
        }
        Some(IconSource::Default) => {
            save_icon(&slug, DEFAULT_ICON, ImageFormat::Png, &share_dir, dry_run)?
        }
        None => {
            let icons_dir = share_dir.join("icons");
            let cached_png = icons_dir.join(format!("{}.png", slug));
            let cached_svg = icons_dir.join(format!("{}.svg", slug));

            if let Some(existing) = existing
                && std::path::Path::new(&existing.icon_path).is_file()
            {
                output::info(&format!("Found installed icon: {}", existing.icon_path));
                user_supplied_icon = existing.user_supplied_icon;
                std::path::PathBuf::from(&existing.icon_path)
            } else if cached_png.exists() {
                output::info(&format!("Found cached icon: {}", cached_png.display()));
                cached_png
            } else if cached_svg.exists() {
                output::info(&format!("Found cached icon: {}", cached_svg.display()));
                cached_svg
            } else {
                output::info(&format!("Fetching favicon for {}...", url));
                if let Some(bytes) = fetch_favicon(&url) {
                    if let Some(icon_format) = detect_format(&bytes) {
                        output::verbose(&format!(
                            "Favicon fetched — format: {}",
                            format_name(&icon_format)
                        ));
                        output::success("Favicon fetched successfully!");
                        save_icon(&slug, &bytes, icon_format, &share_dir, dry_run)?
                    } else {
                        output::warn("Wrong image format — installing with default icon.");
                        save_icon(&slug, DEFAULT_ICON, ImageFormat::Png, &share_dir, dry_run)?
                    }
                } else {
                    output::warn("Favicon not found — installing with default icon.");
                    save_icon(&slug, DEFAULT_ICON, ImageFormat::Png, &share_dir, dry_run)?
                }
            }
        }
    };

    output::verbose(&format!("Icon path: {}", icon_path.display()));

    create_desktop_file(
        name,
        &icon_path,
        &url,
        &browser_name,
        web_app,
        config.categories.as_deref(),
        &desktop_file_path,
        dry_run,
    )?;
    if !dry_run {
        output::info(&format!(
            "Desktop file created at: {}",
            desktop_file_path.display()
        ));
    }

    let entry = AppEntry {
        name: name.to_string(),
        slug: slug.clone(),
        url,
        browser: browser_name,
        icon_path: icon_path.display().to_string(),
        installed_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        user_supplied_icon,
        app_id,
        profile,
    };
    add_or_update_app(&manifest_path, entry, dry_run)?;
    cleanup_app_icons(&slug, Some(&icon_path), &share_dir, dry_run);
    if !dry_run {
        output::info(&format!("Manifest updated at: {}", manifest_path.display()));
        output::success(&format!("✓ {} installed successfully!", name));
    }

    Ok(())
}

fn is_app_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| (b'a'..=b'p').contains(&b))
}

/// Browser launchers are named `<browser>-<app id>-<profile>.desktop`.
fn browser_launchers(dir: &Path) -> HashSet<(String, std::path::PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return HashSet::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let stem = path.file_name()?.to_str()?.strip_suffix(".desktop")?;
            let id = stem.split('-').find(|part| is_app_id(part))?;
            Some((id.to_string(), path))
        })
        .collect()
}

/// Read `--profile-directory=` from a browser launcher's Exec line.
fn launcher_profile(path: &Path) -> Option<String> {
    let contents = std::fs::read_to_string(path).ok()?;
    let exec = contents
        .lines()
        .find_map(|line| line.strip_prefix("Exec="))?;
    let start = exec.find("--profile-directory=")?;
    let rest = &exec[start + "--profile-directory=".len()..];
    let quoted = exec[..start].ends_with('"');
    let end = rest
        .find(|c: char| if quoted { c == '"' } else { c.is_whitespace() })
        .unwrap_or(rest.len());
    Some(rest[..end].to_string()).filter(|p| !p.is_empty() && !p.contains(char::is_control))
}

/// Find the browser's launcher for a web app, waiting for a new install when `id` is empty.
/// Removes the browser's launcher so the app shows once. Returns None after a dry-run preview.
fn adopt_web_app(
    browser: &str,
    url: &str,
    id: &str,
    share_dir: &Path,
    dry_run: bool,
) -> Result<Option<(String, String)>, Box<dyn Error>> {
    let dir = share_dir.join("applications");
    let found = if id.is_empty() {
        if dry_run {
            output::dry_run(&format!(
                "would open {url} in {browser} and wait for Install app"
            ));
            return Ok(None);
        }
        let before = browser_launchers(&dir);
        unsafe {
            std::process::Command::new(browser)
                .arg(url)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .pre_exec(|| {
                    libc::setsid();
                    Ok(())
                })
                .spawn()?;
        }
        output::info(
            "In the browser, select Install app (address bar or menu). Waiting up to 5 minutes...",
        );
        let deadline = Instant::now() + Duration::from_secs(300);
        loop {
            // ponytail: 1s polling, use inotify if the wait ever matters.
            std::thread::sleep(Duration::from_secs(1));
            if let Some(new) = browser_launchers(&dir).difference(&before).next() {
                break new.clone();
            }
            if Instant::now() > deadline {
                return Err("No installed web app found. If the site is already installed, pass its id: --web-app APP_ID (from the browser's launcher name <browser>-APP_ID-<profile>.desktop).".into());
            }
        }
    } else {
        if !is_app_id(id) {
            return Err(format!("Invalid web app id '{id}': expected 32 letters a-p.").into());
        }
        browser_launchers(&dir)
            .into_iter()
            .find(|(found, _)| found == id)
            .unwrap_or((id.to_string(), std::path::PathBuf::new()))
    };
    let (id, launcher) = found;
    let profile = launcher_profile(&launcher).unwrap_or_else(|| "Default".into());
    output::verbose(&format!("Web app: {id} (profile {profile})"));
    if launcher.is_file() {
        if dry_run {
            output::dry_run(&format!(
                "would remove browser launcher: {}",
                launcher.display()
            ));
        } else {
            std::fs::remove_file(&launcher)?;
            output::info(&format!(
                "Replaced browser launcher: {}",
                launcher.display()
            ));
        }
    }
    Ok(Some((id, profile)))
}

fn format_name(fmt: &ImageFormat) -> &'static str {
    match fmt {
        ImageFormat::Png => "PNG",
        ImageFormat::Svg => "SVG",
        ImageFormat::Ico => "ICO",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_browser_launcher_id_and_quoted_profile() {
        let dir = std::env::temp_dir().join(format!("tack-webapp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let id = "pjibgclleladliembfgfagdaldikeohf";
        let launcher = dir.join(format!("brave-{id}-Profile_1.desktop"));
        std::fs::write(
            &launcher,
            format!(
                "[Desktop Entry]\nExec=/opt/brave \"--profile-directory=Profile 1\" --app-id={id}\n"
            ),
        )
        .unwrap();
        std::fs::write(dir.join("spotify.desktop"), "").unwrap();

        let found: Vec<_> = browser_launchers(&dir).into_iter().collect();
        assert_eq!(found, vec![(id.to_string(), launcher.clone())]);
        assert_eq!(launcher_profile(&launcher).as_deref(), Some("Profile 1"));
        assert!(!is_app_id("pjibgclleladliembfgfagdaldikeohz"));

        let desktop = dir.join("app.desktop");
        crate::desktop::create_desktop_file(
            "App",
            Path::new("/tmp/i.png"),
            "https://x.com",
            "brave",
            Some((id, "Profile 1")),
            None,
            &desktop,
            false,
        )
        .unwrap();
        let contents = std::fs::read_to_string(&desktop).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(contents.contains(&format!(
            "Exec=\"brave\" \"--profile-directory=Profile 1\" \"--app-id={id}\""
        )));
        assert!(contents.contains(&format!("StartupWMClass=crx_{id}")));
    }
}
