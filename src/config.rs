use std::path::PathBuf;

#[derive(Default)]
pub struct TackConfig {
    pub browser: Option<String>,
    pub categories: Option<String>,
}

pub fn get_config_path() -> PathBuf {
    if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(config_home).join("tack/config.toml")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/tack/config.toml")
    } else {
        PathBuf::from(".config/tack/config.toml")
    }
}

pub fn load_config() -> TackConfig {
    let mut config = TackConfig::default();
    let config_path = get_config_path();

    if let Ok(contents) = std::fs::read_to_string(config_path) {
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
                match key {
                    "browser" => config.browser = Some(value.to_string()),
                    "categories" => config.categories = Some(value.to_string()),
                    _ => {}
                }
            }
        }
    }
    config
}
