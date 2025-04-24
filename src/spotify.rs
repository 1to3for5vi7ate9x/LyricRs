use rspotify::{
    prelude::*,
    scopes, // Needed for defining authorization scopes
    AuthCodePkceSpotify, // Use the PKCE client
    Credentials,
    OAuth, // Needed for defining scopes and cache path
    model::{PlayableItem},
    // Removed unused Token import
    Config, // Re-add Config
};
use std::sync::Mutex;
use std::path::PathBuf; // Re-add PathBuf
// Removed tokio::runtime::Handle import

// Structure to hold Spotify info (remains the same)
#[derive(Clone, Debug, PartialEq)]
pub struct SpotifyInfo {
    pub artists: Vec<String>,
    pub title: String,
    pub progress_ms: Option<u32>,
    pub duration_ms: Option<u32>,
    pub is_playing: bool,
}

// Removed static TOKIO_RUNTIME definition

// Store the PKCE client
static SPOTIFY_CLIENT: Mutex<Option<AuthCodePkceSpotify>> = Mutex::new(None);

// Initialize the Spotify client using PKCE flow (now async)
pub async fn init_client() -> Result<(), String> {
    let mut client_guard = SPOTIFY_CLIENT.lock().unwrap();
    if client_guard.is_some() {
        println!("Spotify client already initialized.");
        return Ok(());
    }

    println!("Initializing Spotify client (PKCE)...");

    // Load credentials from .env file
    let creds = Credentials::from_env().ok_or_else(|| {
        "Failed to load RSPOTIFY_CLIENT_ID and RSPOTIFY_CLIENT_SECRET from .env".to_string()
    })?;

    // Define required scopes
    let scopes = scopes!("user-read-playback-state");

    // Configure OAuth settings (scopes, redirect URI, cache path)
    let oauth = OAuth::from_env(scopes).ok_or_else(|| {
        "Failed to load RSPOTIFY_REDIRECT_URI from .env".to_string()
    })?;
    // Configure the client config, including the cache path
    let config = Config {
        token_cached: true, // Enable caching
        cache_path: PathBuf::from(".spotify_token_cache.json"), // Explicit path
        ..Default::default()
    };

    // Create the PKCE client with the config
    let mut spotify = AuthCodePkceSpotify::with_config(creds, oauth, config); // Use with_config

    // Generate the authorization URL (only needed if prompting)
    // let _auth_url = spotify.get_authorize_url(None) // Prefix with _ if unused now
    //     .map_err(|e| format!("Failed to get authorize URL: {}", e))?;

    // --- Simplest Token Handling ---
    // Rely entirely on prompt_for_token to check cache, prompt if needed, and manage internal state/cache.

    // Generate the authorization URL (needed for prompt_for_token)
    let auth_url = spotify.get_authorize_url(None)
        .map_err(|e| format!("Failed to get authorize URL: {}", e))?;

    // Call prompt_for_token.
    match spotify.prompt_for_token(&auth_url).await {
        Ok(_) => {
            println!("Spotify client authentication check/prompt successful.");
            // Store the client instance. Assume prompt_for_token handled caching and internal state.
            *client_guard = Some(spotify);
            Ok(())
        }
        Err(e) => Err(format!("Failed to authenticate Spotify client (PKCE): {}", e)),
    }
}

// Fetches current playback info using the authenticated PKCE client (now async)
pub async fn get_current_info() -> Result<Option<SpotifyInfo>, String> {
    let client_guard = SPOTIFY_CLIENT.lock().unwrap();
    let spotify = client_guard.as_ref().ok_or("Spotify client not initialized")?;

    // Fetch current playback state - await the async call directly
    match spotify.current_playback(None, None::<&[_]>).await {
        Ok(Some(context)) => {
            if let Some(PlayableItem::Track(track)) = context.item {
                 // track object in v0.13 likely has duration directly
                let artists = track.artists.iter().map(|a| a.name.clone()).collect();
                let duration_ms = track.duration.num_milliseconds().try_into().ok();

                Ok(Some(SpotifyInfo {
                    artists,
                    title: track.name,
                    // Convert progress from Option<TimeDelta> to Option<u32> milliseconds
                    progress_ms: context.progress.and_then(|p| p.num_milliseconds().try_into().ok()),
                    duration_ms,
                    is_playing: context.is_playing,
                }))
            } else {
                Ok(None) // Not a track
            }
        }
        Ok(None) => Ok(None), // Nothing playing
        // Simplify error handling - catch specific auth errors if needed later
        // Err(ClientError::InvalidToken) => { ... } // Example if needed
        Err(e) => Err(format!("Failed to get playback state: {}", e)), // Catch other errors
    }
}