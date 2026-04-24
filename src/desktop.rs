use std::error::Error;
use std::path::{Path, PathBuf};

use crate::output;

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
    let applications_dir = &desktop_file_path
        .parent()
        .ok_or("Invalid desktop file path")?;

    let exec_args = if browser == "firefox"
        || browser == "zen-browser"
        || browser.contains("firefox")
        || browser.contains("zen")
    {
        format!("{} --ssb {}", browser, url)
    } else {
        format!("{} --app={}", browser, url)
    };

    let categories_str = categories.unwrap_or("Network");

    let contents = format!(
        "[Desktop Entry]
Name={}
Exec={}
Icon={}
Type=Application
Terminal=false
Categories={};",
        name,
        exec_args,
        icon_path.display(),
        categories_str
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
