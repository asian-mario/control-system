use anyhow::{anyhow, Result};
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
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let github_token = env::var("GITHUB_TOKEN").ok();
        
        let github_user = env::var("GITHUB_USER")
            .map_err(|_| anyhow!("GITHUB_USER environment variable is required"))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_path_fallback() {
        let path = Config::determine_cache_path();
        assert!(path.to_string_lossy().contains("cache.json"));
    }
}
