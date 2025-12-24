use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// GitHub user profile information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GithubProfile {
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: String,
    pub bio: Option<String>,
    pub public_repos: u32,
    pub public_gists: u32,
    pub followers: u32,
    pub following: u32,
    pub created_at: Option<DateTime<Utc>>,
}

/// GitHub repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubRepo {
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub stargazers_count: u32,
    pub forks_count: u32,
    pub watchers_count: u32,
    pub language: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
    pub pushed_at: Option<DateTime<Utc>>,
    pub open_issues_count: u32,
    pub fork: bool,
}

/// GitHub event types we care about
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GithubEventType {
    PushEvent,
    CreateEvent,
    DeleteEvent,
    IssuesEvent,
    IssueCommentEvent,
    PullRequestEvent,
    PullRequestReviewEvent,
    WatchEvent,
    ForkEvent,
    ReleaseEvent,
    PublicEvent,
    MemberEvent,
    GollumEvent,
    CommitCommentEvent,
    Unknown(String),
}

impl GithubEventType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "PushEvent" => Self::PushEvent,
            "CreateEvent" => Self::CreateEvent,
            "DeleteEvent" => Self::DeleteEvent,
            "IssuesEvent" => Self::IssuesEvent,
            "IssueCommentEvent" => Self::IssueCommentEvent,
            "PullRequestEvent" => Self::PullRequestEvent,
            "PullRequestReviewEvent" => Self::PullRequestReviewEvent,
            "WatchEvent" => Self::WatchEvent,
            "ForkEvent" => Self::ForkEvent,
            "ReleaseEvent" => Self::ReleaseEvent,
            "PublicEvent" => Self::PublicEvent,
            "MemberEvent" => Self::MemberEvent,
            "GollumEvent" => Self::GollumEvent,
            "CommitCommentEvent" => Self::CommitCommentEvent,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::PushEvent => "[^]",
            Self::CreateEvent => "[+]",
            Self::DeleteEvent => "[-]",
            Self::IssuesEvent => "[!]",
            Self::IssueCommentEvent => "[#]",
            Self::PullRequestEvent => "[~]",
            Self::PullRequestReviewEvent => "[.]",
            Self::WatchEvent => "[*]",
            Self::ForkEvent => "[Y]",
            Self::ReleaseEvent => "[>]",
            Self::PublicEvent => "[@]",
            Self::MemberEvent => "[&]",
            Self::GollumEvent => "[W]",
            Self::CommitCommentEvent => "[C]",
            Self::Unknown(_) => "[?]",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::PushEvent => "pushed to",
            Self::CreateEvent => "created",
            Self::DeleteEvent => "deleted",
            Self::IssuesEvent => "opened issue in",
            Self::IssueCommentEvent => "commented on",
            Self::PullRequestEvent => "opened PR in",
            Self::PullRequestReviewEvent => "reviewed PR in",
            Self::WatchEvent => "starred",
            Self::ForkEvent => "forked",
            Self::ReleaseEvent => "released",
            Self::PublicEvent => "made public",
            Self::MemberEvent => "added member to",
            Self::GollumEvent => "updated wiki in",
            Self::CommitCommentEvent => "commented on commit in",
            Self::Unknown(_) => "did something in",
        }
    }
}

/// GitHub event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubEvent {
    pub id: String,
    pub event_type: GithubEventType,
    pub repo_name: String,
    pub created_at: DateTime<Utc>,
    pub is_new: bool,
}

/// Rate limit information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RateLimit {
    pub limit: u32,
    pub remaining: u32,
    pub reset_at: Option<DateTime<Utc>>,
}

impl RateLimit {
    pub fn usage_percentage(&self) -> f64 {
        if self.limit == 0 {
            return 0.0;
        }
        ((self.limit - self.remaining) as f64 / self.limit as f64) * 100.0
    }

    pub fn is_low(&self) -> bool {
        self.remaining < 10
    }
}

/// Computed statistics from GitHub data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GithubStats {
    pub total_stars: u32,
    pub total_forks: u32,
    pub total_repos: u32,
    pub total_watchers: u32,
}

/// Status of GitHub data fetching
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FetchStatus {
    #[default]
    Idle,
    Fetching,
    Success,
    Error(String),
}

impl FetchStatus {
    pub fn is_fetching(&self) -> bool {
        matches!(self, Self::Fetching)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }
}

/// Complete GitHub state
#[derive(Debug, Clone, Default)]
pub struct GithubState {
    pub profile: Option<GithubProfile>,
    pub repos: Vec<GithubRepo>,
    pub events: Vec<GithubEvent>,
    pub stats: GithubStats,
    pub rate_limit: RateLimit,
    pub last_updated: Option<DateTime<Utc>>,
    pub status: FetchStatus,
}

impl GithubState {
    /// Get top N repos by star count
    pub fn top_repos_by_stars(&self, n: usize) -> Vec<&GithubRepo> {
        let mut repos: Vec<_> = self.repos.iter().filter(|r| !r.fork).collect();
        repos.sort_by(|a, b| b.stargazers_count.cmp(&a.stargazers_count));
        repos.into_iter().take(n).collect()
    }

    /// Get recently updated repos
    pub fn recently_updated_repos(&self, n: usize) -> Vec<&GithubRepo> {
        let mut repos: Vec<_> = self.repos.iter().collect();
        repos.sort_by(|a, b| b.pushed_at.cmp(&a.pushed_at));
        repos.into_iter().take(n).collect()
    }

    /// Compute statistics from repos
    pub fn compute_stats(&mut self) {
        self.stats = GithubStats {
            total_stars: self.repos.iter().map(|r| r.stargazers_count).sum(),
            total_forks: self.repos.iter().map(|r| r.forks_count).sum(),
            total_repos: self.repos.len() as u32,
            total_watchers: self.repos.iter().map(|r| r.watchers_count).sum(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_from_str() {
        assert!(matches!(
            GithubEventType::from_str("PushEvent"),
            GithubEventType::PushEvent
        ));
        assert!(matches!(
            GithubEventType::from_str("UnknownType"),
            GithubEventType::Unknown(_)
        ));
    }

    #[test]
    fn test_rate_limit_percentage() {
        let rate_limit = RateLimit {
            limit: 100,
            remaining: 75,
            reset_at: None,
        };
        assert!((rate_limit.usage_percentage() - 25.0).abs() < 0.01);
    }
}
