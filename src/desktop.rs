use std::error::Error;
use std::path::{Path, PathBuf};

use crate::{output, util::validate_app_browser};

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

pub fn get_desktop_file_path(slug: &str, share_dir: &Path) -> PathBuf {
    share_dir
        .join("applications")
        .join(format!("{}.desktop", slug))
}

pub fn create_desktop_file(
    name: &str,
    icon_path: &Path,
    url: &str,
    browser: &str,
    categories: Option<&str>,
    desktop_file_path: &Path,
    dry_run: bool,
) -> Result<(), Box<dyn Error>> {
    validate_app_browser(browser)?;
    let applications_dir = &desktop_file_path
        .parent()
        .ok_or("Invalid desktop file path")?;

    let exec_args = format!(
        "{} {}",
        exec_arg(browser),
        exec_arg(&format!("--app={url}"))
    );

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
Categories={};",
        desktop_value(name),
        exec_args,
        desktop_value(&icon_path.display().to_string()),
        desktop_value(categories_str)
    );

    if dry_run {
        output::dry_run(&format!("would create: {}", desktop_file_path.display()));
        output::verbose(&format!("contents:\n{}", contents));
        return Ok(());
    }

    std::fs::create_dir_all(applications_dir)?;
    std::fs::write(desktop_file_path, contents)?;

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
