# LyricRs - Spotify Lyrics Overlay (macOS)

LyricRs is a simple macOS desktop application that displays the lyrics for the currently playing song on Spotify in an always-on-top, transparent window. It fetches song information using the Spotify Web API and scrapes lyrics from Genius.com.

## Features

*   **Spotify Integration:** Connects to the Spotify Web API using OAuth (PKCE flow) to get the current song title, artists, and playback status. Caches authentication tokens for seamless subsequent runs.
*   **Lyrics Display:** Fetches lyrics by scraping Genius.com based on the detected song.
*   **Always-on-Top:** The lyrics window stays visible above other applications.
*   **Transparency Control:** An opacity slider allows adjusting the window's background transparency.
*   **Multi-language Support:** Includes Noto Sans KR font to render characters for languages like Korean correctly.
*   **Modular Code:** Organized into separate modules for Spotify interaction, lyrics fetching, and the GUI application logic.

## Setup

### Prerequisites

1.  **Rust:** Ensure you have a working Rust development environment installed (via [rustup](https://rustup.rs/)).
2.  **Spotify Account:** A Spotify account (Free or Premium).
3.  **Spotify Developer Application:**
    *   Go to the [Spotify Developer Dashboard](https://developer.spotify.com/dashboard/).
    *   Create a new application or use an existing one.
    *   Note down your **Client ID** and **Client Secret**.
    *   In the application settings, add the following **Redirect URI**: `http://localhost:8888/callback`
    *   Save the changes to your Spotify application settings.

### Installation & Configuration

1.  **Clone the Repository:** (Assuming the code is hosted on Git)
    ```bash
    git clone <repository-url>
    cd spotify_lyrics_overlay
    ```
    (If not using Git, ensure you have the project files in a directory named `spotify_lyrics_overlay`).

2.  **Create `.env` File:**
    In the root of the `spotify_lyrics_overlay` directory, create a file named `.env` and add your Spotify credentials:
    ```dotenv
    RSPOTIFY_CLIENT_ID=YOUR_CLIENT_ID
    RSPOTIFY_CLIENT_SECRET=YOUR_CLIENT_SECRET
    RSPOTIFY_REDIRECT_URI=http://localhost:8888/callback
    ```
    Replace `YOUR_CLIENT_ID` and `YOUR_CLIENT_SECRET` with the actual values from your Spotify Developer Dashboard. **Important:** This file contains secrets and should *not* be committed to version control (it's included in `.gitignore`).

3.  **Build & Run:**
    Navigate to the `spotify_lyrics_overlay` directory in your terminal and run:
    ```bash
    cargo run
    ```

4.  **First-Time Authorization:**
    *   The first time you run the application, it will print a message and open your default web browser to a Spotify authorization page.
    *   Log in to Spotify and click "Agree" to grant the application permission to read your playback state.
    *   Spotify will redirect your browser to `http://localhost:8888/callback?code=...`. You might see a "connection refused" or similar error in the browser – this is normal.
    *   **Copy the entire URL** from your browser's address bar (the one starting with `http://localhost:8888/callback?...`).
    *   **Paste this URL** back into the terminal where the application is waiting.
    *   The application should then authenticate successfully and launch the GUI. This authorization process only needs to be done once (unless the token cache is deleted or expires).

## Usage

*   Run the application using `cargo run` from the project directory.
*   Ensure Spotify is running and playing music.
*   The lyrics for the current song will appear in the overlay window.
*   Use the slider at the top of the window to adjust the background transparency.
*   The window will stay on top of other applications.
*   Close the window or press `Ctrl+C` in the terminal to stop the application.

## Limitations & Disclaimers

*   **Scraping Fragility:** This application relies on scraping Genius.com. If Genius changes its website structure, the lyrics fetching will likely break until the scraping code (`src/lyrics.rs`) is updated.
*   **Missing Lyrics:** Lyrics may not be available on Genius.com for all songs, or the generated URL might not match the one used by Genius, resulting in a "404 Not Found" error displayed in the app.
*   **Incorrect Formatting:** While basic cleaning is performed, some non-lyric text or incorrect formatting might occasionally appear depending on the specific Genius page structure.
*   **Genius.com Terms of Service:** This tool scrapes Genius.com. Please use it responsibly and respect their Terms of Service. Avoid making excessive requests. This tool is intended for personal, non-commercial use.

## Future Development Ideas

*   **Lyrics Synchronization:** Attempt to highlight or scroll lyrics based on the current song playback position (`progress_ms`).
*   **Configuration:** Add UI options for font size, text color, and background color. Persist settings to a config file.
*   **Fallback Lyrics Sources:** Implement scraping for other lyrics websites (e.g., AZLyrics, Musixmatch) as fallbacks if Genius fails.
*   **Error Handling:** Improve error messages and potentially add retry logic for network issues.
*   **UI Enhancements:** Add a progress bar for song playback, improve layout.
*   **Packaging:** Create distributable application bundles for macOS.
*   **Cross-Platform Support:** Investigate using platform-specific APIs or alternative methods for Spotify integration and window management on Windows/Linux.