use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

/// Application configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct Config {
    /// GitHub personal access token (recommended for higher rate limits)
    pub github_token: Option<String>,
    /// GitHub username (required)
    pub github_user: String,
    /// Refresh interval in seconds (default: 60)
    pub refresh_secs: u64,
    /// Whether to reduce/disable animations
    pub reduced_motion: bool,
    /// Path to cache file
    pub cache_path: PathBuf,
}

impl Config {
    /// Load configuration from environment variables.
    /// Returns Ok(None) if GITHUB_USER is missing from both env and saved settings.
    pub fn from_env_optional() -> Result<Option<Self>> {
        let github_user = match env::var("GITHUB_USER") {
            Ok(u) if !u.is_empty() => u,
            _ => {
                // Try loading from saved settings
                match AppSettings::load() {
                    Some(settings) if !settings.github_user.is_empty() => settings.github_user,
                    _ => return Ok(None),
                }
            }
        };
        Ok(Some(Self::build_with_user(github_user)?))
    }

    /// Build config with a known github_user
    pub fn build_with_user(github_user: String) -> Result<Self> {
        let github_token = env::var("GITHUB_TOKEN").ok();

        let refresh_secs = env::var("CONTROL_SYSTEM_REFRESH_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60);

        let reduced_motion = env::var("CONTROL_SYSTEM_REDUCED_MOTION")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        let cache_path = Self::determine_cache_path();

        Ok(Config {
            github_token,
            github_user,
            refresh_secs,
            reduced_motion,
            cache_path,
        })
    }

    /// Load configuration from environment variables (requires GITHUB_USER)
    pub fn from_env() -> Result<Self> {
        match Self::from_env_optional()? {
            Some(config) => Ok(config),
            None => Err(anyhow!("GITHUB_USER environment variable is required")),
        }
    }

    /// Determine the cache file path
    fn determine_cache_path() -> PathBuf {
        // Try ~/.config/control-system/cache.json first
        if let Some(config_dir) = dirs::config_dir() {
            let app_dir = config_dir.join("control-system");
            if std::fs::create_dir_all(&app_dir).is_ok() {
                return app_dir.join("cache.json");
            }
        }

        // Fallback to current directory
        PathBuf::from("./control-system-cache.json")
    }

    /// Check if we have a GitHub token configured
    pub fn has_token(&self) -> bool {
        self.github_token.is_some()
    }
}

/// Persistent app settings saved to disk
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSettings {
    pub github_user: String,
}

impl AppSettings {
    /// Path to the settings file
    fn path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            let app_dir = config_dir.join("control-system");
            let _ = std::fs::create_dir_all(&app_dir);
            app_dir.join("settings.json")
        } else {
            PathBuf::from("./control-system-settings.json")
        }
    }

    /// Load settings from disk
    pub fn load() -> Option<Self> {
        let data = std::fs::read_to_string(Self::path()).ok()?;
        serde_json::from_str(&data).ok()
    }

    /// Save settings to disk
    pub fn save(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(Self::path(), data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_path_fallback() {
        let path = Config::determine_cache_path();
        assert!(path.to_string_lossy().contains("cache.json"));
    }
}
