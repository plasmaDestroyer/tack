use std::error::Error;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::blocking::{Client, Response};

use crate::ico;
use crate::output;

pub const DEFAULT_ICON: &[u8] = include_bytes!("../assets/default.png");

#[derive(Debug)]
pub enum ImageFormat {
    Png,
    Svg,
    Ico,
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let (_, mut rest) = tag.split_once(char::is_whitespace)?;
    loop {
        rest = rest.trim_start();
        let end = rest
            .find(|c: char| c.is_whitespace() || matches!(c, '=' | '>' | '/'))
            .unwrap_or(rest.len());
        if end == 0 {
            return None;
        }
        let key = &rest[..end];
        rest = rest[end..].trim_start();
        let Some(value) = rest.strip_prefix('=') else {
            continue;
        };
        rest = value.trim_start();
        let value = if rest.starts_with(['\'', '"']) {
            let quote = rest.chars().next()?;
            rest = &rest[1..];
            let end = rest.find(quote)?;
            let value = &rest[..end];
            rest = &rest[end + 1..];
            value
        } else {
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '>')
                .unwrap_or(rest.len());
            let value = &rest[..end];
            rest = &rest[end..];
            value
        };
        if key.eq_ignore_ascii_case(name) {
            return Some(value);
        }
    }
}

fn find_icon_in_html(html: &str) -> Option<String> {
    // ponytail: common link markup only; use an HTML parser if malformed tags matter.
    let lower = html.to_ascii_lowercase();
    let mut best = None;
    let mut best_score = 0;
    for (start, _) in lower.match_indices("<link") {
        if !lower
            .as_bytes()
            .get(start + 5)
            .is_some_and(u8::is_ascii_whitespace)
        {
            continue;
        }
        let Some(end) = html[start..].find('>') else {
            continue;
        };
        let tag = &html[start..start + end + 1];
        let Some(rel) = attribute(tag, "rel") else {
            continue;
        };
        let has = |kind: &str| {
            rel.split_ascii_whitespace()
                .any(|token| token.eq_ignore_ascii_case(kind))
        };
        let score = if has("apple-touch-icon") {
            3
        } else if has("icon") {
            if has("shortcut") { 1 } else { 2 }
        } else {
            0
        };
        if score > best_score
            && let Some(href) = attribute(tag, "href").filter(|href| !href.is_empty())
        {
            best = Some(href.replace("&amp;", "&").replace("&#38;", "&"));
            best_score = score;
        }
    }
    best
}

fn read_body(response: Response) -> Option<Vec<u8>> {
    // ponytail: cap network bodies at 5 MiB; raise if real icon sources need more.
    const MAX_BYTES: u64 = 5 * 1024 * 1024;
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|size| size > MAX_BYTES)
    {
        return None;
    }
    let mut bytes = Vec::new();
    response.take(MAX_BYTES + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= MAX_BYTES).then_some(bytes)
}

fn fetch_icon(client: &Client, url: &str) -> Option<Vec<u8>> {
    let bytes = read_body(client.get(url).send().ok()?)?;
    match detect_format(&bytes)? {
        ImageFormat::Ico => ico::ico_to_png(&bytes).ok(),
        _ => Some(bytes),
    }
}

fn fetch_svgl_icon(url: &str, client: &Client) -> Option<Vec<u8>> {
    let parsed_target = reqwest::Url::parse(url).ok()?;
    let target_host = parsed_target.host_str()?.trim_start_matches("www.");

    let bytes = read_body(client.get("https://api.svgl.app").send().ok()?)?;
    let json: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let entries = json.as_array()?;

    for entry in entries {
        if let Some(entry_url_str) = entry.get("url").and_then(|u| u.as_str())
            && let Ok(parsed_entry) = reqwest::Url::parse(entry_url_str)
            && let Some(entry_host) = parsed_entry.host_str()
        {
            let entry_host_clean = entry_host.trim_start_matches("www.");
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
                    && let Some(bytes) = fetch_icon(client, dl_url)
                {
                    return Some(bytes);
                }
            }
        }
    }
    None
}

pub fn fetch_favicon(url: &str) -> Option<Vec<u8>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .ok()?;

    // 0. Try fetching from svgl.app first
    if let Some(svgl_bytes) = fetch_svgl_icon(url, &client) {
        return Some(svgl_bytes);
    }

    let mut parsed_url = reqwest::Url::parse(url).ok()?;

    if let Ok(response) = client.get(url).send() {
        parsed_url = response.url().clone();
        if let Some(bytes) = read_body(response)
            && let Some(href) = find_icon_in_html(&String::from_utf8_lossy(&bytes))
            && let Ok(icon_url) = parsed_url.join(&href)
            && let Some(icon) = fetch_icon(&client, icon_url.as_str())
        {
            return Some(icon);
        }
    }
    if let Ok(favicon_url) = parsed_url.join("/favicon.ico")
        && let Some(icon) = fetch_icon(&client, favicon_url.as_str())
    {
        return Some(icon);
    }
    let host = parsed_url.host_str()?;
    let google_api_url = format!("https://www.google.com/s2/favicons?domain={host}&sz=128");
    fetch_icon(&client, &google_api_url)
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

    crate::util::atomic_write(&icon_path, &final_bytes)?;
    Ok(icon_path)
}

pub fn cleanup_app_icons(slug: &str, keep: Option<&Path>, share_dir: &Path, dry_run: bool) {
    if slug.is_empty() || crate::util::slugify(slug) != slug {
        return;
    }
    for extension in ["png", "svg"] {
        let path = share_dir.join("icons").join(format!("{slug}.{extension}"));
        if keep == Some(path.as_path()) || !path.exists() {
            continue;
        }
        if dry_run {
            output::dry_run(&format!("would remove unused icon: {}", path.display()));
        } else if let Err(error) = std::fs::remove_file(&path) {
            output::warn(&format!(
                "Could not remove unused icon {}: {error}",
                path.display()
            ));
        }
    }
}

pub fn detect_format(bytes: &[u8]) -> Option<ImageFormat> {
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some(ImageFormat::Png)
    } else if std::str::from_utf8(bytes).is_ok_and(|text| {
        let text = text.trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
        text.starts_with("<svg") || (text.starts_with("<?xml") && text.contains("<svg"))
    }) {
        Some(ImageFormat::Svg)
    } else if ico::is_ico(bytes) {
        Some(ImageFormat::Ico)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_links_handle_common_attribute_layouts() {
        for (html, href) in [
            ("<LINK\nREL = 'ICON' HREF = /logo.png>", "/logo.png"),
            (
                "<link rel='icon' data-href='wrong.png' href=\"/good.svg?a=1&amp;b=2\">",
                "/good.svg?a=1&b=2",
            ),
            (
                "<link rel=icon href=small.png><link rel='apple-touch-icon' href=large.png>",
                "large.png",
            ),
        ] {
            assert_eq!(find_icon_in_html(html).as_deref(), Some(href));
        }
        assert!(find_icon_in_html("<linker rel=icon href=wrong.png>").is_none());
        assert!(find_icon_in_html("<link rel=stylesheet href=style.css>").is_none());
    }

    #[test]
    fn cleanup_rejects_slugs_that_escape_the_icons_directory() {
        let root = std::env::temp_dir().join(format!(
            "tack-icon-cleanup-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("icons")).unwrap();
        let outside = root.join("outside.png");
        std::fs::write(&outside, DEFAULT_ICON).unwrap();
        cleanup_app_icons("../outside", None, &root, false);
        assert!(outside.exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
