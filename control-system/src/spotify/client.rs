use anyhow::{anyhow, Result};
use tracing::{debug, warn};

use super::auth::SpotifyAuth;
use super::state::SpotifyTokens;

const API_BASE: &str = "https://api.spotify.com/v1";

/// Spotify Web API client
pub struct SpotifyClient {
    client: reqwest::Client,
    tokens: SpotifyTokens,
}

impl SpotifyClient {
    pub fn new(tokens: SpotifyTokens) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client, tokens }
    }

    /// Ensure we have a valid access token, refreshing if needed
    async fn ensure_token(&mut self) -> Result<()> {
        if self.tokens.is_expired() {
            debug!("Spotify token expired, refreshing...");
            self.tokens = SpotifyAuth::refresh_token(&self.tokens).await?;
        }
        Ok(())
    }

    /// Get current playback state
    pub async fn get_playback(&mut self) -> Result<Option<PlaybackResponse>> {
        self.ensure_token().await?;

        let resp = self
            .client
            .get(&format!(
                "{}/me/player?additional_types=track,episode",
                API_BASE
            ))
            .bearer_auth(&self.tokens.access_token)
            .send()
            .await?;

        if resp.status().as_u16() == 204 {
            // No active playback
            return Ok(None);
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            warn!("Spotify API error {}: {}", status, text);
            return Err(anyhow!("Spotify API error: {}", status));
        }

        let body: serde_json::Value = resp.json().await?;
        Ok(Some(parse_playback(&body)))
    }

    /// Toggle play/pause
    pub async fn toggle_playback(&mut self, currently_playing: bool) -> Result<()> {
        self.ensure_token().await?;

        let endpoint = if currently_playing {
            format!("{}/me/player/pause", API_BASE)
        } else {
            format!("{}/me/player/play", API_BASE)
        };

        let resp = self
            .client
            .put(&endpoint)
            .header("Content-Length", "0")
            .bearer_auth(&self.tokens.access_token)
            .body(reqwest::Body::from(vec![]))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            warn!("Spotify toggle failed ({}): {}", status, text);
            return Err(anyhow!("Spotify toggle failed: {} {}", status, text));
        }
        Ok(())
    }

    /// Skip to next track
    pub async fn next_track(&mut self) -> Result<()> {
        self.ensure_token().await?;

        let resp = self
            .client
            .post(&format!("{}/me/player/next", API_BASE))
            .header("Content-Length", "0")
            .bearer_auth(&self.tokens.access_token)
            .body(reqwest::Body::from(vec![]))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            warn!("Spotify next failed ({}): {}", status, text);
            return Err(anyhow!("Spotify next failed: {} {}", status, text));
        }
        Ok(())
    }

    /// Skip to previous track
    pub async fn prev_track(&mut self) -> Result<()> {
        self.ensure_token().await?;

        let resp = self
            .client
            .post(&format!("{}/me/player/previous", API_BASE))
            .header("Content-Length", "0")
            .bearer_auth(&self.tokens.access_token)
            .body(reqwest::Body::from(vec![]))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            warn!("Spotify prev failed ({}): {}", status, text);
            return Err(anyhow!("Spotify prev failed: {} {}", status, text));
        }
        Ok(())
    }

    /// Get the user's queue (up next)
    pub async fn get_queue(&mut self) -> Result<Vec<QueueItem>> {
        self.ensure_token().await?;

        let resp = self
            .client
            .get(&format!("{}/me/player/queue", API_BASE))
            .bearer_auth(&self.tokens.access_token)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let body: serde_json::Value = resp.json().await?;
        let queue = body["queue"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .take(8)
                    .map(|item| {
                        let name = item["name"].as_str().unwrap_or("").to_string();
                        let artists: Vec<String> = item["artists"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|x| x["name"].as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();
                        let artist = if artists.is_empty() {
                            item["show"]["name"].as_str().unwrap_or("").to_string()
                        } else {
                            artists.join(", ")
                        };
                        QueueItem { name, artist }
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(queue)
    }
}

/// A track/episode in the queue
#[derive(Debug, Clone, Default)]
pub struct QueueItem {
    pub name: String,
    pub artist: String,
}
/// Parsed playback response
#[derive(Debug, Clone)]
pub struct PlaybackResponse {
    pub is_playing: bool,
    pub track_name: String,
    pub artist_name: String,
    pub album_name: String,
    pub progress_ms: u64,
    pub duration_ms: u64,
    pub album_art_url: Option<String>,
}

fn parse_playback(body: &serde_json::Value) -> PlaybackResponse {
    let is_playing = body["is_playing"].as_bool().unwrap_or(false);
    let progress_ms = body["progress_ms"].as_u64().unwrap_or(0);

    // Handle null item (happens between tracks, during ads, etc.)
    let item = &body["item"];
    if item.is_null() {
        debug!(
            "Spotify item is null, currently_playing_type: {:?}",
            body["currently_playing_type"]
        );
        return PlaybackResponse {
            is_playing,
            track_name: String::new(),
            artist_name: String::new(),
            album_name: String::new(),
            progress_ms,
            duration_ms: 0,
            album_art_url: None,
        };
    }

    let track_name = item["name"].as_str().unwrap_or("").to_string();

    // Handle both tracks (artists array) and episodes (show.publisher)
    let artists: Vec<String> = item["artists"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a["name"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let artist_name = if artists.is_empty() {
        // Might be a podcast episode - try show name
        item["show"]["name"].as_str().unwrap_or("").to_string()
    } else {
        artists.join(", ")
    };

    // Handle both tracks (album) and episodes (show)
    let album_name = item["album"]["name"]
        .as_str()
        .or_else(|| item["show"]["name"].as_str())
        .unwrap_or("")
        .to_string();

    let duration_ms = item["duration_ms"].as_u64().unwrap_or(0);

    // Try album images first, then show images
    let album_art_url = item["album"]["images"]
        .as_array()
        .or_else(|| item["show"]["images"].as_array())
        .and_then(|imgs| imgs.first())
        .and_then(|img| img["url"].as_str())
        .map(|s| s.to_string());

    PlaybackResponse {
        is_playing,
        track_name,
        artist_name,
        album_name,
        progress_ms,
        duration_ms,
        album_art_url,
    }
}
