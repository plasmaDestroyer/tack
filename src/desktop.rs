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
    desktop_file_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let applications_dir = &desktop_file_path
        .parent()
        .ok_or("Invalid desktop file path")?;
    std::fs::create_dir_all(applications_dir)?;

    let contents = format!(
        "[Desktop Entry]
Name={}
Exec={} --app={}
Icon={}
Type=Application
Terminal=false
Categories=Network;",
        name,
        browser,
        url,
        icon_path.display()
    );

    std::fs::write(desktop_file_path, contents)?;

    Ok(())
}
