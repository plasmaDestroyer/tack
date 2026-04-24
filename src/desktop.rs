use std::error::Error;
use std::path::{Path, PathBuf};

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
) -> Result<(), Box<dyn Error>> {
    let applications_dir = &desktop_file_path
        .parent()
        .ok_or("Invalid desktop file path")?;
    std::fs::create_dir_all(applications_dir)?;

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

    std::fs::write(desktop_file_path, contents)?;

    Ok(())
}
