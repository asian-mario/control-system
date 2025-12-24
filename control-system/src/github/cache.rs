use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tracing::{debug, info, warn};

use super::models::{GithubEvent, GithubProfile, GithubRepo, GithubStats, RateLimit, GithubState};
use chrono::{DateTime, Utc};

/// Serializable cache data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheData {
    pub profile: Option<GithubProfile>,
    pub repos: Vec<GithubRepo>,
    pub events: Vec<GithubEvent>,
    pub stats: GithubStats,
    pub rate_limit: RateLimit,
    pub last_updated: Option<DateTime<Utc>>,
    pub cache_version: u32,
}

impl Default for CacheData {
    fn default() -> Self {
        Self {
            profile: None,
            repos: Vec::new(),
            events: Vec::new(),
            stats: GithubStats::default(),
            rate_limit: RateLimit::default(),
            last_updated: None,
            cache_version: 1,
        }
    }
}

impl From<&GithubState> for CacheData {
    fn from(state: &GithubState) -> Self {
        Self {
            profile: state.profile.clone(),
            repos: state.repos.clone(),
            events: state.events.clone(),
            stats: state.stats.clone(),
            rate_limit: state.rate_limit.clone(),
            last_updated: state.last_updated,
            cache_version: 1,
        }
    }
}

impl CacheData {
    pub fn to_github_state(&self) -> GithubState {
        GithubState {
            profile: self.profile.clone(),
            repos: self.repos.clone(),
            events: self.events.clone(),
            stats: self.stats.clone(),
            rate_limit: self.rate_limit.clone(),
            last_updated: self.last_updated,
            status: super::models::FetchStatus::Idle,
        }
    }
}

/// GitHub data cache manager
pub struct GithubCache {
    path: std::path::PathBuf,
}

impl GithubCache {
    /// Create a new cache manager
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    /// Load cached data from disk
    pub async fn load(&self) -> Result<Option<CacheData>> {
        if !self.path.exists() {
            debug!("Cache file does not exist: {:?}", self.path);
            return Ok(None);
        }

        info!("Loading cache from {:?}", self.path);
        
        let content = fs::read_to_string(&self.path).await?;
        let data: CacheData = serde_json::from_str(&content)?;
        
        // Check cache version
        if data.cache_version != 1 {
            warn!("Cache version mismatch, ignoring cache");
            return Ok(None);
        }

        debug!("Loaded cache with {} repos", data.repos.len());
        Ok(Some(data))
    }

    /// Save data to cache
    pub async fn save(&self, state: &GithubState) -> Result<()> {
        let data = CacheData::from(state);
        let content = serde_json::to_string_pretty(&data)?;
        
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&self.path, content).await?;
        info!("Saved cache to {:?}", self.path);
        
        Ok(())
    }

    /// Check if cache exists
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Delete cache file
    pub async fn clear(&self) -> Result<()> {
        if self.path.exists() {
            fs::remove_file(&self.path).await?;
            info!("Cleared cache at {:?}", self.path);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_cache_roundtrip() {
        let dir = tempdir().unwrap();
        let cache_path = dir.path().join("test-cache.json");
        let cache = GithubCache::new(&cache_path);

        let state = GithubState {
            profile: Some(GithubProfile {
                login: "testuser".to_string(),
                name: Some("Test User".to_string()),
                ..Default::default()
            }),
            repos: vec![GithubRepo {
                name: "test-repo".to_string(),
                full_name: "testuser/test-repo".to_string(),
                description: Some("A test repo".to_string()),
                html_url: "https://github.com/testuser/test-repo".to_string(),
                stargazers_count: 42,
                forks_count: 10,
                watchers_count: 42,
                language: Some("Rust".to_string()),
                updated_at: None,
                pushed_at: None,
                open_issues_count: 5,
                fork: false,
            }],
            events: Vec::new(),
            stats: GithubStats {
                total_stars: 42,
                total_forks: 10,
                total_repos: 1,
                total_watchers: 42,
            },
            rate_limit: RateLimit::default(),
            last_updated: Some(Utc::now()),
            status: super::super::models::FetchStatus::Success,
        };

        // Save
        cache.save(&state).await.unwrap();
        assert!(cache.exists());

        // Load
        let loaded = cache.load().await.unwrap().unwrap();
        assert_eq!(loaded.profile.as_ref().unwrap().login, "testuser");
        assert_eq!(loaded.repos.len(), 1);
        assert_eq!(loaded.repos[0].name, "test-repo");
        assert_eq!(loaded.stats.total_stars, 42);
    }
}
