use anyhow::Result;
use chrono::Utc;
use octocrab::Octocrab;
use tracing::{debug, error, info, warn};

use crate::config::Config;

use super::models::{
    FetchStatus, GithubEvent, GithubEventType, GithubProfile, GithubRepo, GithubState, RateLimit,
};

/// GitHub API client wrapper
pub struct GithubClient {
    client: Octocrab,
    username: String,
}

impl GithubClient {
    /// Create a new GitHub client
    pub fn new(config: &Config) -> Result<Self> {
        let builder = if let Some(ref token) = config.github_token {
            Octocrab::builder().personal_token(token.clone())
        } else {
            Octocrab::builder()
        };

        let client = builder.build()?;

        Ok(Self {
            client,
            username: config.github_user.clone(),
        })
    }

    /// Fetch all GitHub data and return updated state
    pub async fn fetch_all(&self, current_state: &GithubState) -> GithubState {
        let mut state = GithubState {
            status: FetchStatus::Fetching,
            ..current_state.clone()
        };

        info!("Fetching GitHub data for user: {}", self.username);

        // Fetch profile
        match self.fetch_profile().await {
            Ok(profile) => {
                debug!("Fetched profile for {}", profile.login);
                state.profile = Some(profile);
            }
            Err(e) => {
                error!("Failed to fetch profile: {}", e);
                state.status = FetchStatus::Error(format!("Profile fetch failed: {}", e));
                return state;
            }
        }

        // Fetch repositories
        match self.fetch_repos().await {
            Ok(repos) => {
                debug!("Fetched {} repositories", repos.len());
                state.repos = repos;
            }
            Err(e) => {
                error!("Failed to fetch repos: {}", e);
                state.status = FetchStatus::Error(format!("Repos fetch failed: {}", e));
                return state;
            }
        }

        // Fetch events
        let existing_event_ids: std::collections::HashSet<_> =
            current_state.events.iter().map(|e| e.id.clone()).collect();

        match self.fetch_events(&existing_event_ids).await {
            Ok(events) => {
                debug!("Fetched {} events", events.len());
                state.events = events;
            }
            Err(e) => {
                warn!("Failed to fetch events: {}", e);
                // Don't fail completely for events
            }
        }

        // Fetch rate limit
        match self.fetch_rate_limit().await {
            Ok(rate_limit) => {
                debug!(
                    "Rate limit: {}/{}",
                    rate_limit.remaining, rate_limit.limit
                );
                state.rate_limit = rate_limit;
            }
            Err(e) => {
                warn!("Failed to fetch rate limit: {}", e);
            }
        }

        // Compute stats
        state.compute_stats();
        state.last_updated = Some(Utc::now());
        state.status = FetchStatus::Success;

        info!(
            "GitHub fetch complete: {} repos, {} stars total",
            state.stats.total_repos, state.stats.total_stars
        );

        state
    }

    /// Fetch user profile
    async fn fetch_profile(&self) -> Result<GithubProfile> {
        let user = self.client.users(&self.username).profile().await?;

        Ok(GithubProfile {
            login: user.login,
            name: user.name,
            avatar_url: user.avatar_url.to_string(),
            bio: user.bio,
            public_repos: user.public_repos as u32,
            public_gists: user.public_gists as u32,
            followers: user.followers as u32,
            following: user.following as u32,
            created_at: Some(user.created_at),
        })
    }

    /// Fetch user repositories
    async fn fetch_repos(&self) -> Result<Vec<GithubRepo>> {
        let mut all_repos = Vec::new();
        let mut page = 1u32;
        let per_page = 100u8;
        let max_repos = 200; // Cap to avoid too many API calls

        loop {
            let repos = self
                .client
                .users(&self.username)
                .repos()
                .per_page(per_page)
                .page(page)
                .send()
                .await?;

            if repos.items.is_empty() {
                break;
            }

            for repo in repos.items {
                all_repos.push(GithubRepo {
                    name: repo.name,
                    full_name: repo.full_name.unwrap_or_default(),
                    description: repo.description,
                    html_url: repo.html_url.map(|u| u.to_string()).unwrap_or_default(),
                    stargazers_count: repo.stargazers_count.unwrap_or(0) as u32,
                    forks_count: repo.forks_count.unwrap_or(0) as u32,
                    watchers_count: repo.watchers_count.unwrap_or(0) as u32,
                    language: repo.language.and_then(|v| v.as_str().map(|s| s.to_string())),
                    updated_at: repo.updated_at,
                    pushed_at: repo.pushed_at,
                    open_issues_count: repo.open_issues_count.unwrap_or(0) as u32,
                    fork: repo.fork.unwrap_or(false),
                });

                if all_repos.len() >= max_repos {
                    break;
                }
            }

            if all_repos.len() >= max_repos {
                break;
            }

            page += 1;
            if page > 10 {
                // Safety limit
                break;
            }
        }

        Ok(all_repos)
    }

    /// Fetch user events
    async fn fetch_events(
        &self,
        existing_ids: &std::collections::HashSet<String>,
    ) -> Result<Vec<GithubEvent>> {
        // Use the activity API to get user events
        let url = format!("/users/{}/events?per_page=50", self.username);
        let response: Vec<serde_json::Value> = self.client.get(&url, None::<&()>).await?;

        let mut events = Vec::new();

        for event in response {
            if let (Some(id), Some(event_type), Some(repo), Some(created_at)) = (
                event.get("id").and_then(|v| v.as_str()),
                event.get("type").and_then(|v| v.as_str()),
                event.get("repo").and_then(|v| v.get("name")).and_then(|v| v.as_str()),
                event.get("created_at").and_then(|v| v.as_str()),
            ) {
                let is_new = !existing_ids.contains(id);
                
                if let Ok(created_at) = chrono::DateTime::parse_from_rfc3339(created_at) {
                    events.push(GithubEvent {
                        id: id.to_string(),
                        event_type: GithubEventType::from_str(event_type),
                        repo_name: repo.to_string(),
                        created_at: created_at.with_timezone(&Utc),
                        is_new,
                    });
                }
            }
        }

        Ok(events)
    }

    /// Fetch rate limit information
    async fn fetch_rate_limit(&self) -> Result<RateLimit> {
        let rate_limit = self.client.ratelimit().get().await?;
        
        Ok(RateLimit {
            limit: rate_limit.rate.limit as u32,
            remaining: rate_limit.rate.remaining as u32,
            reset_at: Some(chrono::DateTime::from_timestamp(rate_limit.rate.reset as i64, 0)
                .unwrap_or_else(|| Utc::now())),
        })
    }
}
