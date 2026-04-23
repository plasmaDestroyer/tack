use std::error::Error;
use std::path::{Path, PathBuf};

use crate::ico;

pub const DEFAULT_ICON: &[u8] = include_bytes!("../assets/default.png");

pub enum ImageFormat {
    Png,
    Svg,
    Ico,
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

    // If the source is ICO, convert to PNG first.
    let (final_bytes, extension) = match format {
        ImageFormat::Ico => {
            let png_bytes = ico::ico_to_png(bytes)?;
            (png_bytes, "png")
        }
        ImageFormat::Png => (bytes.to_vec(), "png"),
        ImageFormat::Svg => (bytes.to_vec(), "svg"),
    };

    let icon_path = icons_dir.join(format!("{}.{}", slug, extension));
    std::fs::write(&icon_path, &final_bytes)?;
    Ok(icon_path)
}

pub fn detect_format(bytes: &[u8]) -> Option<ImageFormat> {
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        Some(ImageFormat::Png)
    } else if bytes.starts_with(b"<svg") || bytes.starts_with(b"<?xml") {
        Some(ImageFormat::Svg)
    } else if ico::is_ico(bytes) {
        Some(ImageFormat::Ico)
    } else {
        None
    }
}
