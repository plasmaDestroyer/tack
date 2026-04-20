use std::error::Error;
use std::path::{Path, PathBuf};

pub const DEFAULT_ICON: &[u8] = include_bytes!("../assets/default.png");

pub enum ImageFormat {
    Png,
    Svg,
}

pub fn fetch_favicon(url: &str) -> Option<Vec<u8>> {
    let favicon_url = format!("{}/favicon.ico", url.trim_end_matches('/'));

    let response = reqwest::blocking::get(&favicon_url).ok()?;

    if response.status().is_success() {
        response.bytes().ok().map(|b| b.to_vec())
    } else {
        None
    }
}

pub fn save_icon(
    slug: &str,
    bytes: &[u8],
    format: ImageFormat,
    share_dir: &Path,
) -> Result<PathBuf, Box<dyn Error>> {
    let icons_dir = share_dir.join("icons");
    std::fs::create_dir_all(&icons_dir)?;

    let extension = match format {
        ImageFormat::Png => "png",
        ImageFormat::Svg => "svg",
    };

    let icon_path = icons_dir.join(format!("{}.{}", slug, extension));
    std::fs::write(&icon_path, bytes)?;
    Ok(icon_path)
}

pub fn detect_format(bytes: &[u8]) -> Option<ImageFormat> {
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        Some(ImageFormat::Png)
    } else if bytes.starts_with(b"<svg") || bytes.starts_with(b"<?xml") {
        Some(ImageFormat::Svg)
    } else {
        None
    }
}
