use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenersConfig {
    pub image: String,
    pub pdf: String,
    pub text: String,
    pub default: String,
}

impl OpenersConfig {
    fn editor_default() -> String {
        std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub start_dir: PathBuf,
    pub show_hidden: bool,
    pub theme_name: String,
    #[serde(default)]
    pub openers: OpenersConfig,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            start_dir: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
            show_hidden: false,
            theme_name: "default".to_string(),
            openers: OpenersConfig {
                image: "xdg-open".to_string(),
                pdf: "xdg-open".to_string(),
                text: OpenersConfig::editor_default(),
                default: "xdg-open".to_string(),
            },
        }
    }
}

impl Settings {
    pub fn config_dir() -> PathBuf {
        if let Ok(path) = std::env::var("TFSV_CONFIG_DIR") {
            return PathBuf::from(path);
        }

        if let Some(path) = dirs::config_dir() {
            path.join("tfsv")
        } else {
            PathBuf::from("/tmp")
        }
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("settings.toml")
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if !path.exists() {
            let settings = Self::default();
            settings.save()?;
            return Ok(settings);
        }
        let content = fs::read_to_string(path)?;
        let settings: Self = toml::from_str(&content)?;
        Ok(settings)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir)?;
        let content = toml::to_string_pretty(self)?;
        fs::write(Self::config_path(), content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Settings;

    #[test]
    fn config_dir_honors_environment_override() {
        let key = "TFSV_CONFIG_DIR";
        let previous = std::env::var(key).ok();
        let target = std::env::temp_dir().join(format!("tfsv-config-override-{}", std::process::id()));

        unsafe {
            std::env::set_var(key, &target);
        }
        assert_eq!(Settings::config_dir(), target);

        unsafe {
            match previous {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}
