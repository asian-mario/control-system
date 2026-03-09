use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tracing::{error, info, warn};

use super::auth::SpotifyAuth;
use super::client::SpotifyClient;
use super::state::{AlbumArt, PlayerState, SpotifyState};

/// Commands that can be sent to the Spotify poller
#[derive(Debug, Clone)]
pub enum SpotifyCommand {
    TogglePlayback,
    NextTrack,
    PrevTrack,
    Stop,
}

pub struct SpotifyPoller;

impl SpotifyPoller {
    /// Start the Spotify polling loop.
    /// Returns a watch receiver for state updates and a command sender.
    pub fn start(
        poll_interval: Duration,
    ) -> (watch::Receiver<SpotifyState>, mpsc::Sender<SpotifyCommand>) {
        let (state_tx, state_rx) = watch::channel(SpotifyState::default());
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<SpotifyCommand>(16);

        tokio::spawn(async move {
            let tokens = match SpotifyAuth::load_tokens() {
                Some(t) => t,
                None => {
                    let _ = state_tx.send(SpotifyState {
                        connected: false,
                        error: Some("Not authenticated".to_string()),
                        ..Default::default()
                    });
                    return;
                }
            };

            let mut client = SpotifyClient::new(tokens);
            let http_client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .connect_timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());
            let mut interval = tokio::time::interval(poll_interval);
            let mut cached_art_url: Option<String> = None;
            let mut cached_art: Option<AlbumArt> = None;

            info!("Spotify poller started");

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        match client.get_playback().await {
                            Ok(Some(pb)) => {
                                // Fetch album art if URL changed
                                let art_url = pb.album_art_url.clone();
                                if art_url != cached_art_url {
                                    cached_art = match &art_url {
                                        Some(url) => fetch_album_art(&http_client, url).await,
                                        None => None,
                                    };
                                    cached_art_url = art_url;
                                }

                                // Fetch queue
                                let queue = client.get_queue().await.unwrap_or_default()
                                    .into_iter()
                                    .map(|q| super::state::QueueItem { name: q.name, artist: q.artist })
                                    .collect();

                                let _ = state_tx.send(SpotifyState {
                                    player: PlayerState {
                                        is_playing: pb.is_playing,
                                        track_name: pb.track_name,
                                        artist_name: pb.artist_name,
                                        album_name: pb.album_name,
                                        progress_ms: pb.progress_ms,
                                        duration_ms: pb.duration_ms,
                                        album_art_url: pb.album_art_url,
                                        album_art: cached_art.clone(),
                                        last_updated: Some(chrono::Utc::now()),
                                        queue,
                                    },
                                    connected: true,
                                    error: None,
                                });
                            }
                            Ok(None) => {
                                let _ = state_tx.send(SpotifyState {
                                    player: PlayerState::default(),
                                    connected: true,
                                    error: None,
                                });
                            }
                            Err(e) => {
                                warn!("Spotify poll error: {}", e);
                                let _ = state_tx.send(SpotifyState {
                                    connected: false,
                                    error: Some(e.to_string()),
                                    ..state_tx.borrow().clone()
                                });
                            }
                        }
                    }
                    Some(cmd) = cmd_rx.recv() => {
                        match cmd {
                            SpotifyCommand::TogglePlayback => {
                                let is_playing = state_tx.borrow().player.is_playing;
                                if let Err(e) = client.toggle_playback(is_playing).await {
                                    error!("Spotify toggle error: {}", e);
                                }
                            }
                            SpotifyCommand::NextTrack => {
                                if let Err(e) = client.next_track().await {
                                    error!("Spotify next error: {}", e);
                                }
                            }
                            SpotifyCommand::PrevTrack => {
                                if let Err(e) = client.prev_track().await {
                                    error!("Spotify prev error: {}", e);
                                }
                            }
                            SpotifyCommand::Stop => {
                                info!("Spotify poller stopping");
                                return;
                            }
                        }
                        // After a command, fetch updated state quickly
                        tokio::time::sleep(Duration::from_millis(300)).await;
                        if let Ok(Some(pb)) = client.get_playback().await {
                            // Fetch album art if URL changed
                            let art_url = pb.album_art_url.clone();
                            if art_url != cached_art_url {
                                cached_art = match &art_url {
                                    Some(url) => fetch_album_art(&http_client, url).await,
                                    None => None,
                                };
                                cached_art_url = art_url;
                            }

                            // Clone queue from previous state BEFORE calling send() to
                            // avoid a deadlock: borrow() holds a read lock and
                            // send() needs a write lock on the same RwLock.
                            let prev_queue = state_tx.borrow().player.queue.clone();
                            let _ = state_tx.send(SpotifyState {
                                player: PlayerState {
                                    is_playing: pb.is_playing,
                                    track_name: pb.track_name,
                                    artist_name: pb.artist_name,
                                    album_name: pb.album_name,
                                    progress_ms: pb.progress_ms,
                                    duration_ms: pb.duration_ms,
                                    album_art_url: pb.album_art_url,
                                    album_art: cached_art.clone(),
                                    last_updated: Some(chrono::Utc::now()),
                                    queue: prev_queue,
                                },
                                connected: true,
                                error: None,
                            });
                        }
                    }
                }
            }
        });

        (state_rx, cmd_tx)
    }
}

/// Fetch album art from a URL and decode it into RGBA pixel data
async fn fetch_album_art(client: &reqwest::Client, url: &str) -> Option<AlbumArt> {
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        warn!("Failed to fetch album art: {}", resp.status());
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();

    Some(AlbumArt {
        width: w,
        height: h,
        rgba_data: rgba.into_raw(),
        source_url: url.to_string(),
    })
}
