use std::error::Error;
use std::path::{Path, PathBuf};

use crate::ico;
use crate::output;

pub const DEFAULT_ICON: &[u8] = include_bytes!("../assets/default.png");

pub enum ImageFormat {
    Png,
    Svg,
    Ico,
}

fn get_href(original_tag: &str) -> Option<String> {
    let tag_lower = original_tag.to_ascii_lowercase();
    if let Some(href_idx) = tag_lower.find("href=\"") {
        let start = href_idx + 6;
        if let Some(end) = tag_lower[start..].find('"') {
            return Some(original_tag[start..start + end].to_string());
        }
    }
    if let Some(href_idx) = tag_lower.find("href='") {
        let start = href_idx + 6;
        if let Some(end) = tag_lower[start..].find('\'') {
            return Some(original_tag[start..start + end].to_string());
        }
    }
    None
}

fn find_icon_in_html(html: &str) -> Option<String> {
    let html_lower = html.to_ascii_lowercase();
    let mut best_href = None;
    let mut best_score = 0;

    let mut search_start = 0;
    while let Some(idx) = html_lower[search_start..].find("<link ") {
        let tag_start = search_start + idx;
        let tag_end_idx = html_lower[tag_start..].find('>');
        if let Some(end_offset) = tag_end_idx {
            let tag_lower = &html_lower[tag_start..tag_start + end_offset + 1];

            let mut score = 0;
            if tag_lower.contains("rel=\"apple-touch-icon\"")
                || tag_lower.contains("rel='apple-touch-icon'")
            {
                score = 3;
            } else if tag_lower.contains("rel=\"icon\"") || tag_lower.contains("rel='icon'") {
                score = 2;
            } else if tag_lower.contains("rel=\"shortcut icon\"")
                || tag_lower.contains("rel='shortcut icon'")
            {
                score = 1;
            }

            if score > best_score {
                let original_tag = &html[tag_start..tag_start + end_offset + 1];
                if let Some(href) = get_href(original_tag) {
                    best_score = score;
                    best_href = Some(href);
                }
            }
            search_start = tag_start + end_offset;
        } else {
            break;
        }
    }
    best_href
}

pub fn fetch_svgl_icon(url: &str) -> Option<Vec<u8>> {
    let parsed_target = reqwest::Url::parse(url).ok()?;
    let target_host = parsed_target.host_str()?.replace("www.", "");

    let response = reqwest::blocking::get("https://api.svgl.app").ok()?;
    if !response.status().is_success() {
        return None;
    }

    let text = response.text().ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    let entries = json.as_array()?;

    for entry in entries {
        if let Some(entry_url_str) = entry.get("url").and_then(|u| u.as_str())
            && let Ok(parsed_entry) = reqwest::Url::parse(entry_url_str)
            && let Some(entry_host) = parsed_entry.host_str()
        {
            let entry_host_clean = entry_host.replace("www.", "");
            if entry_host_clean == target_host {
                let route = entry.get("route");
                let svg_url = if let Some(r) = route.and_then(|r| r.as_str()) {
                    Some(r)
                } else if let Some(obj) = route.and_then(|r| r.as_object()) {
                    obj.get("light")
                        .or_else(|| obj.get("dark"))
                        .and_then(|v| v.as_str())
                } else {
                    None
                };

                if let Some(dl_url) = svg_url
                    && let Ok(r) = reqwest::blocking::get(dl_url)
                    && r.status().is_success()
                    && let Ok(bytes) = r.bytes()
                {
                    return Some(bytes.to_vec());
                }
            }
        }
    }
    None
}

pub fn fetch_favicon(url: &str) -> Option<Vec<u8>> {
    // 0. Try fetching from svgl.app first
    if let Some(svgl_bytes) = fetch_svgl_icon(url) {
        return Some(svgl_bytes);
    }

    let parsed_url = reqwest::Url::parse(url).ok()?;

    // 1. Try fetching the HTML to find icon tags
    if let Ok(response) = reqwest::blocking::get(url)
        && response.status().is_success()
        && let Ok(html) = response.text()
        && let Some(href) = find_icon_in_html(&html)
        && let Ok(icon_url) = parsed_url.join(&href)
    {
        let icon_response = reqwest::blocking::get(icon_url).ok();
        if let Some(r) = icon_response
            && r.status().is_success()
            && let Ok(bytes) = r.bytes()
        {
            return Some(bytes.to_vec());
        }
    }

    // 2. Fallback to /favicon.ico directly
    if let Ok(favicon_url) = parsed_url.join("/favicon.ico") {
        let direct_bytes = reqwest::blocking::get(favicon_url.clone())
            .ok()
            .and_then(|r| {
                if r.status().is_success() {
                    r.bytes().ok()
                } else {
                    None
                }
            })
            .map(|b| b.to_vec());

        if direct_bytes.is_some() {
            return direct_bytes;
        }
    }

    // 3. Fallback to Google Favicon API
    if let Some(host) = parsed_url.host_str() {
        let google_api_url = format!("https://www.google.com/s2/favicons?domain={}&sz=128", host);
        let google_bytes = reqwest::blocking::get(&google_api_url)
            .ok()
            .and_then(|r| {
                if r.status().is_success() {
                    r.bytes().ok()
                } else {
                    None
                }
            })
            .map(|b| b.to_vec());

        if google_bytes.is_some() {
            return google_bytes;
        }
    }

    None
}

pub fn save_icon(
    slug: &str,
    bytes: &[u8],
    format: ImageFormat,
    share_dir: &Path,
    dry_run: bool,
) -> Result<PathBuf, Box<dyn Error>> {
    let icons_dir = share_dir.join("icons");

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

    if dry_run {
        output::dry_run(&format!("would save icon: {}", icon_path.display()));
        return Ok(icon_path);
    }

    std::fs::create_dir_all(&icons_dir)?;
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
