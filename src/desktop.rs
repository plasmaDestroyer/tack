use std::error::Error;
use std::path::{Path, PathBuf};

use crate::{
    output,
    util::{atomic_write, validate_app_browser, validate_slug},
};

fn desktop_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn exec_arg(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\\\\\")
            .replace('"', "\\\\\"")
            .replace('$', "\\\\$")
            .replace('`', "\\\\`")
            .replace('%', "%%")
    )
}

/// Browser arguments that open the app window.
pub fn launch_args(url: &str, web_app: Option<(&str, &str)>) -> Vec<String> {
    match web_app {
        Some((id, profile)) => vec![
            format!("--profile-directory={profile}"),
            format!("--app-id={id}"),
        ],
        None => vec![format!("--app={url}")],
    }
}

pub fn get_desktop_file_path(slug: &str, share_dir: &Path) -> Result<PathBuf, String> {
    validate_slug(slug)?;
    Ok(share_dir
        .join("applications")
        .join(format!("{}.desktop", slug)))
}

#[allow(clippy::too_many_arguments)]
pub fn create_desktop_file(
    name: &str,
    icon_path: &Path,
    url: &str,
    browser: &str,
    web_app: Option<(&str, &str)>,
    categories: Option<&str>,
    desktop_file_path: &Path,
    dry_run: bool,
) -> Result<(), Box<dyn Error>> {
    validate_app_browser(browser)?;
    let exec_args = std::iter::once(browser.to_string())
        .chain(launch_args(url, web_app))
        .map(|arg| exec_arg(&arg))
        .collect::<Vec<_>>()
        .join(" ");
    // Window class lets docks match the window to this launcher.
    let wm_class = web_app
        .map(|(id, _)| format!("\nStartupWMClass=crx_{id}"))
        .unwrap_or_default();

    let categories_str = categories.unwrap_or("Network").trim_end_matches(';');
    let categories_str = if categories_str.is_empty() {
        "Network"
    } else {
        categories_str
    };

    let contents = format!(
        "[Desktop Entry]
Name={}
Exec={}
Icon={}
Type=Application
Terminal=false
Categories={};{}",
        desktop_value(name),
        exec_args,
        desktop_value(&icon_path.display().to_string()),
        desktop_value(categories_str),
        wm_class
    );

    if dry_run {
        output::dry_run(&format!("would create: {}", desktop_file_path.display()));
        output::verbose(&format!("contents:\n{}", contents));
        return Ok(());
    }

    atomic_write(desktop_file_path, contents.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_desktop_fields_and_exec_arguments() {
        let path =
            std::env::temp_dir().join(format!("tack-desktop-{}.desktop", std::process::id()));
        create_desktop_file(
            "Bad\nHidden=true",
            Path::new("/tmp/icon with space.png"),
            "https://example.com/search?q=a&b=2/%20",
            "/tmp/browser with space",
            None,
            Some("Network;Utility;"),
            &path,
            false,
        )
        .unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(path).unwrap();

        let lines: Vec<_> = contents.lines().collect();
        assert!(lines.contains(&"Name=Bad\\nHidden=true"));
        assert!(!lines.contains(&"Hidden=true"));
        assert!(lines.contains(
            &"Exec=\"/tmp/browser with space\" \"--app=https://example.com/search?q=a&b=2/%%20\""
        ));
        assert!(lines.contains(&"Icon=/tmp/icon with space.png"));
        assert!(lines.contains(&"Categories=Network;Utility;"));
    }
}
