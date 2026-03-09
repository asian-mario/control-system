use anyhow::{anyhow, Result};
use base64::Engine;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tracing::info;

use super::state::SpotifyTokens;

/// Redirect URI - uses explicit IPv4 loopback per Spotify requirements.
/// `localhost` is not allowed; must use `127.0.0.1`. HTTP is permitted for loopback.
/// Nothing actually listens here; the user copies the URL from their browser.
const REDIRECT_URI: &str = "http://127.0.0.1:8585/callback";
const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const AUTH_URL: &str = "https://accounts.spotify.com/authorize";
const SCOPES: &str =
    "user-read-playback-state user-modify-playback-state user-read-currently-playing";

/// Spotify OAuth 2.0 with PKCE
pub struct SpotifyAuth;

impl SpotifyAuth {
    /// Path to stored tokens
    pub fn token_path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            let app_dir = config_dir.join("control-system");
            let _ = std::fs::create_dir_all(&app_dir);
            app_dir.join("spotify.json")
        } else {
            PathBuf::from("./spotify.json")
        }
    }

    /// Load tokens from disk
    pub fn load_tokens() -> Option<SpotifyTokens> {
        let path = Self::token_path();
        let data = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&data).ok()
    }

    /// Save tokens to disk
    pub fn save_tokens(tokens: &SpotifyTokens) -> Result<()> {
        let path = Self::token_path();
        let data = serde_json::to_string_pretty(tokens)?;
        std::fs::write(&path, data)?;
        Ok(())
    }

    /// Check if Spotify is configured (tokens exist)
    pub fn is_configured() -> bool {
        Self::load_tokens().is_some()
    }

    /// Generate PKCE code verifier (random 128 chars)
    fn generate_code_verifier() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..64).map(|_| rng.gen::<u8>()).collect();
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes)
    }

    /// Generate PKCE code challenge from verifier
    fn generate_code_challenge(verifier: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let result = hasher.finalize();
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(result)
    }

    /// Build the authorization URL
    pub fn build_auth_url(client_id: &str) -> (String, String) {
        let verifier = Self::generate_code_verifier();
        let challenge = Self::generate_code_challenge(&verifier);

        let url = format!(
            "{}?client_id={}&response_type=code&redirect_uri={}&scope={}&code_challenge_method=S256&code_challenge={}",
            AUTH_URL,
            urlencoding::encode(client_id),
            urlencoding::encode(REDIRECT_URI),
            urlencoding::encode(SCOPES),
            urlencoding::encode(&challenge),
        );

        (url, verifier)
    }

    /// Extract the authorization code from a pasted redirect URL.
    /// The URL looks like: https://localhost/callback?code=XXXXX
    pub fn extract_code_from_url(url_str: &str) -> Result<String> {
        let url_str = url_str.trim();
        let parsed = url::Url::parse(url_str).map_err(|_| {
            anyhow!("Invalid URL. Make sure you copied the full URL from the address bar.")
        })?;

        // Check for error parameter
        if let Some((_, err)) = parsed.query_pairs().find(|(k, _)| k == "error") {
            return Err(anyhow!("Authorization denied: {}", err));
        }

        parsed
            .query_pairs()
            .find(|(k, _)| k == "code")
            .map(|(_, v)| v.to_string())
            .ok_or_else(|| {
                anyhow!("No authorization code found in URL. Make sure you copied the full URL.")
            })
    }

    /// Get the redirect URI (for display in setup instructions)
    pub fn redirect_uri() -> &'static str {
        REDIRECT_URI
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code(
        client_id: &str,
        code: &str,
        verifier: &str,
    ) -> Result<SpotifyTokens> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", REDIRECT_URI),
            ("client_id", client_id),
            ("code_verifier", verifier),
        ];

        let resp = client.post(TOKEN_URL).form(&params).send().await?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Token exchange failed: {}", text));
        }

        let body: serde_json::Value = resp.json().await?;

        let access_token = body["access_token"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing access_token"))?
            .to_string();
        let refresh_token = body["refresh_token"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing refresh_token"))?
            .to_string();
        let expires_in = body["expires_in"].as_u64().unwrap_or(3600);

        let tokens = SpotifyTokens {
            access_token,
            refresh_token,
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64),
            client_id: client_id.to_string(),
        };

        Self::save_tokens(&tokens)?;
        Ok(tokens)
    }

    /// Refresh an expired access token
    pub async fn refresh_token(tokens: &SpotifyTokens) -> Result<SpotifyTokens> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", &tokens.refresh_token),
            ("client_id", &tokens.client_id),
        ];

        let resp = client.post(TOKEN_URL).form(&params).send().await?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Token refresh failed: {}", text));
        }

        let body: serde_json::Value = resp.json().await?;

        let access_token = body["access_token"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing access_token"))?
            .to_string();
        let refresh_token = body["refresh_token"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| tokens.refresh_token.clone());
        let expires_in = body["expires_in"].as_u64().unwrap_or(3600);

        let new_tokens = SpotifyTokens {
            access_token,
            refresh_token,
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64),
            client_id: tokens.client_id.clone(),
        };

        Self::save_tokens(&new_tokens)?;
        Ok(new_tokens)
    }
}
