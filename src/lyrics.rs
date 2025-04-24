use reqwest;
use scraper::{Html, Selector, Node};
use regex::Regex;

// --- Genius URL Formatting ---

// Formats a single component (artist name or title) for the Genius URL path.
fn format_genius_path_component(input: &str) -> String {
    let mut processed = input.to_lowercase();

    // Remove specific unwanted parentheticals like (feat.), (with), (explicit)
    let re_unwanted_paren = Regex::new(r"\s*\((feat|ft|with|explicit)[^)]*\)\s*").unwrap();
    processed = re_unwanted_paren.replace_all(&processed, "").to_string();

    // Remove specific common suffixes like "- radio edit", "- live version"
    let re_suffix = Regex::new(r"\s+-\s+(radio edit|live|acoustic|version|edit|mix)\b.*").unwrap();
    processed = re_suffix.replace_all(&processed, "").to_string();

    // Handle ampersands - replace with "and" before general replacement
    processed = processed.replace(" & ", "-and-");

    // Replace remaining non-alphanumeric characters (allow letters, numbers) with a single hyphen
    let re_non_alpha = Regex::new(r"[^a-z0-9]+").unwrap();
    processed = re_non_alpha.replace_all(&processed, "-").to_string();

    // Trim leading/trailing hyphens
    let re_trim_hyphens = Regex::new(r"^-+|-+$").unwrap();
    processed = re_trim_hyphens.replace_all(&processed, "").to_string();

    // Collapse multiple consecutive hyphens into one
    let re_collapse_hyphens = Regex::new(r"-{2,}").unwrap();
    processed = re_collapse_hyphens.replace_all(&processed, "-").to_string();

    processed
}

// Builds the Genius URL using a list of artists.
fn build_genius_url(artists: &[String], title: &str) -> String {
    // Format each artist name individually
    let formatted_artist_names: Vec<String> = artists
        .iter()
        .map(|a| format_genius_path_component(a))
        .collect();

    // Join artist names with "-and-" for the URL path (common Genius pattern)
    let joined_artists = formatted_artist_names.join("-and-");

    // Format the title
    let formatted_title = format_genius_path_component(title);

    // Combine for the final URL
    format!("https://genius.com/{}-{}-lyrics", joined_artists, formatted_title)
}

// --- HTML Fetching & Parsing --- (Keep fetch_lyrics_html and parse_and_extract_genius_lyrics as they are)

async fn fetch_lyrics_html(url: &str) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/100.0.4896.88 Safari/537.36")
        .build()?;
    let response = client.get(url).timeout(std::time::Duration::from_secs(15)).send().await?;
    response.error_for_status()?.text().await
}

fn parse_and_extract_genius_lyrics(html: &str) -> Result<String, String> {
    let html_doc = Html::parse_document(html);
    let selector_str = "div[data-lyrics-container='true']";
    let selector = Selector::parse(selector_str)
        .map_err(|e| format!("Invalid CSS selector '{}': {:?}", selector_str, e))?;

    let mut raw_lyrics = String::new();
    let lyrics_containers = html_doc.select(&selector);
    let mut container_count = 0;

    for container in lyrics_containers {
        container_count += 1;
        for node in container.children() {
            match node.value() {
                Node::Text(text) => {
                    raw_lyrics.push_str(&text);
                }
                Node::Element(element) => {
                    match element.name() {
                        "br" => raw_lyrics.push('\n'),
                        "a" => { // Handle Genius annotations/links
                            if let Some(a_ref) = scraper::ElementRef::wrap(node) {
                                for text_node in a_ref.text() {
                                     raw_lyrics.push_str(text_node);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        raw_lyrics.push('\n');
    }
    raw_lyrics = raw_lyrics.trim_end().to_string();

    if container_count == 0 {
        Err(format!("Could not find lyrics container matching selector '{}'. Website structure may have changed.", selector_str))
    } else if raw_lyrics.trim().is_empty() {
        Err(format!("Found lyrics container matching selector '{}', but it contained no text.", selector_str))
    } else {
        Ok(raw_lyrics)
    }
}

// --- Lyrics Cleaning --- (Keep clean_genius_lyrics as it is)

fn clean_genius_lyrics(raw_lyrics: &str) -> String {
    let re_headers = Regex::new(r"\s*\[.*?\]\s*\n?").unwrap();
    let no_headers = re_headers.replace_all(raw_lyrics, "");
    let re_newlines = Regex::new(r"\n{2,}").unwrap();
    let collapsed_newlines = re_newlines.replace_all(&no_headers, "\n");
    collapsed_newlines.trim().to_string()
}

// --- Public API ---

/// Fetches lyrics from Genius for the given artists and title.
/// Returns the cleaned lyrics or an error string.
pub async fn fetch_and_parse_lyrics(artists: &[String], title: &str) -> Result<String, String> {
    // Check if artist list is empty, which shouldn't happen with valid Spotify data
    if artists.is_empty() {
        return Err("Cannot fetch lyrics: Artist list is empty.".to_string());
    }

    let url = build_genius_url(artists, title);
    println!("Attempting to fetch lyrics from: {}", url);

    match fetch_lyrics_html(&url).await {
        Ok(html) => {
            println!("Successfully fetched HTML ({} bytes)", html.len());
            match parse_and_extract_genius_lyrics(&html) {
                Ok(raw_lyrics) => {
                    let cleaned = clean_genius_lyrics(&raw_lyrics);
                    if cleaned.is_empty() {
                         Err("Extracted lyrics were empty after cleaning.".to_string())
                    } else {
                        Ok(cleaned)
                    }
                }
                Err(e) => Err(format!("Parsing error: {}", e)),
            }
        }
        Err(e) => {
            let mut error_msg = format!("Network error fetching {}: {}", url, e);
             if let Some(status) = e.status() {
                if status == reqwest::StatusCode::NOT_FOUND {
                    error_msg.push_str("\nHint: Lyrics page not found (404). URL format might be wrong or song not on Genius.");
                } else if status.is_client_error() || status.is_server_error() {
                     error_msg.push_str(&format!("\nHint: Received HTTP error {}. Genius might be blocking requests or the URL is wrong.", status));
                }
            } else if e.is_timeout() {
                 error_msg.push_str("\nHint: Request timed out.");
            }
            Err(error_msg)
        }
    }
}