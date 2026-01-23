use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WoodeyeConfig {
    pub custom_script_path: Option<String>,
}

/// Get the path to the config file (~/.config/woodeye/config.json)
pub fn get_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".config").join("woodeye").join("config.json"))
}

/// Load config from disk, returning default if file doesn't exist
pub fn load_config() -> Result<WoodeyeConfig, String> {
    let config_path = get_config_path().ok_or("Could not determine home directory")?;

    if !config_path.exists() {
        return Ok(WoodeyeConfig::default());
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse config file: {}", e))
}

/// Save config to disk, creating directories if needed
pub fn save_config(config: &WoodeyeConfig) -> Result<(), String> {
    let config_path = get_config_path().ok_or("Could not determine home directory")?;

    // Create parent directories if they don't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&config_path, content).map_err(|e| format!("Failed to write config file: {}", e))
}

/// Expand ~ to home directory in paths
pub fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]).to_string_lossy().to_string();
        }
    }
    path.to_string()
}
