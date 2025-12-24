use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tracing::{debug, error, info};

use crate::config::Config;

use super::cache::GithubCache;
use super::client::GithubClient;
use super::models::GithubState;

/// Commands that can be sent to the GitHub poller
#[derive(Debug, Clone)]
pub enum GithubCommand {
    /// Force an immediate refresh
    Refresh,
    /// Stop the poller
    Stop,
}

/// GitHub data poller that runs in the background
pub struct GithubPoller {
    client: Arc<GithubClient>,
    cache: Arc<GithubCache>,
    refresh_interval: Duration,
}

impl GithubPoller {
    /// Create a new GitHub poller
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let client = Arc::new(GithubClient::new(config)?);
        let cache = Arc::new(GithubCache::new(&config.cache_path));

        Ok(Self {
            client,
            cache,
            refresh_interval: Duration::from_secs(config.refresh_secs),
        })
    }

    /// Load initial state from cache
    pub async fn load_cached_state(&self) -> GithubState {
        match self.cache.load().await {
            Ok(Some(data)) => {
                info!("Loaded GitHub state from cache");
                data.to_github_state()
            }
            Ok(None) => {
                debug!("No cache found, starting fresh");
                GithubState::default()
            }
            Err(e) => {
                error!("Failed to load cache: {}", e);
                GithubState::default()
            }
        }
    }

    /// Start the poller task
    /// Returns a watch receiver for state updates and an mpsc sender for commands
    pub fn start(
        self,
        initial_state: GithubState,
    ) -> (
        watch::Receiver<GithubState>,
        mpsc::Sender<GithubCommand>,
    ) {
        let (state_tx, state_rx) = watch::channel(initial_state);
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<GithubCommand>(16);

        let client = self.client;
        let cache = self.cache;
        let refresh_interval = self.refresh_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(refresh_interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            // Do an initial fetch
            let current = state_tx.borrow().clone();
            let new_state = client.fetch_all(&current).await;
            let _ = state_tx.send(new_state.clone());
            if let Err(e) = cache.save(&new_state).await {
                error!("Failed to save cache: {}", e);
            }

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        debug!("Periodic GitHub refresh triggered");
                        let current = state_tx.borrow().clone();
                        let new_state = client.fetch_all(&current).await;
                        let _ = state_tx.send(new_state.clone());
                        if let Err(e) = cache.save(&new_state).await {
                            error!("Failed to save cache: {}", e);
                        }
                    }
                    Some(cmd) = cmd_rx.recv() => {
                        match cmd {
                            GithubCommand::Refresh => {
                                info!("Manual GitHub refresh triggered");
                                let current = state_tx.borrow().clone();
                                let new_state = client.fetch_all(&current).await;
                                let _ = state_tx.send(new_state.clone());
                                if let Err(e) = cache.save(&new_state).await {
                                    error!("Failed to save cache: {}", e);
                                }
                            }
                            GithubCommand::Stop => {
                                info!("GitHub poller stopping");
                                break;
                            }
                        }
                    }
                }
            }
        });

        (state_rx, cmd_tx)
    }
}
