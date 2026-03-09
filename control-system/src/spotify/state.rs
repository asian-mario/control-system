use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Spotify playback state
#[derive(Debug, Clone, Default)]
pub struct SpotifyState {
    pub player: PlayerState,
    pub connected: bool,
    pub error: Option<String>,
}

/// Current player state from Spotify
#[derive(Debug, Clone, Default)]
pub struct PlayerState {
    pub is_playing: bool,
    pub track_name: String,
    pub artist_name: String,
    pub album_name: String,
    pub progress_ms: u64,
    pub duration_ms: u64,
    pub album_art_url: Option<String>,
    pub last_updated: Option<DateTime<Utc>>,
    /// Cached album art as raw RGBA pixels (width, height, data)
    pub album_art: Option<AlbumArt>,
    /// Up next queue (up to 5 items)
    pub queue: Vec<QueueItem>,
}

/// A track/episode in the queue (display only)
#[derive(Debug, Clone, Default)]
pub struct QueueItem {
    pub name: String,
    pub artist: String,
}

/// Cached album art image data
#[derive(Debug, Clone)]
pub struct AlbumArt {
    pub width: u32,
    pub height: u32,
    pub rgba_data: Vec<u8>,
    /// The URL this art was fetched from (for cache invalidation)
    pub source_url: String,
}

impl PlayerState {
    /// Progress as a fraction (0.0 - 1.0)
    pub fn progress_fraction(&self) -> f64 {
        if self.duration_ms == 0 {
            return 0.0;
        }
        self.progress_ms as f64 / self.duration_ms as f64
    }

    /// Format time as M:SS
    pub fn format_time(ms: u64) -> String {
        let secs = ms / 1000;
        let mins = secs / 60;
        let remaining = secs % 60;
        format!("{}:{:02}", mins, remaining)
    }
}

/// Spotify token data (persisted to disk)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
    pub client_id: String,
}

impl SpotifyTokens {
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }
}
