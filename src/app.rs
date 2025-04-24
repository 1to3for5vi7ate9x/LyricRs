use eframe::egui;
use egui::FontFamily::Proportional;
use egui::{Color32, FontData, FontDefinitions}; // Re-added Color32
use std::{sync::{Arc, Mutex}, time::Duration};

// Import functions/structs from our other modules
use crate::spotify::{self, SpotifyInfo};
use crate::lyrics;
use crate::cache; // Import cache module

// --- Application State ---

#[derive(Clone, Debug)] // Removed Default, will init manually
pub struct AppState {
    pub current_info: Option<SpotifyInfo>,
    pub lyrics: String,
    pub status: String,
    pub opacity: f32, // Opacity level (0.0 to 1.0)
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_info: None,
            lyrics: String::from(""),
            status: String::from("Initializing..."),
            opacity: 1.0, // Default to fully opaque
        }
    }
}


// --- GUI Application ---

pub struct LyricsApp {
    state: Arc<Mutex<AppState>>,
}

impl LyricsApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // --- Font Configuration ---
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "noto_sans_kr".to_owned(),
            FontData::from_static(include_bytes!("../../assets/NotoSansKR-VariableFont_wght.ttf"))
        );
        fonts
            .families
            .entry(Proportional)
            .or_default()
            .insert(0, "noto_sans_kr".to_owned());
        cc.egui_ctx.set_fonts(fonts);
        // --- End Font Configuration ---

        // Apply initial visuals (including opacity)
        let initial_state = AppState::default();
        let initial_opacity = initial_state.opacity;
        Self::apply_opacity(&cc.egui_ctx, initial_opacity);


        let state = Arc::new(Mutex::new(initial_state));


        // --- Background Thread ---
        let state_clone = Arc::clone(&state);
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime in background thread");

            // Initialize the cache (synchronous call)
            if let Err(e) = cache::init_cache() {
                eprintln!("Failed to initialize lyrics cache: {}", e);
                // Application can continue, but caching won't work
            }

            let mut last_song_title: Option<String> = None; // Track only title to detect changes

            loop {
                rt.block_on(async {
                    let mut current_state = state_clone.lock().unwrap();
                    current_state.status = "Checking Spotify...".to_string();
                    drop(current_state);

                    // Await the async function call
                    match spotify::get_current_info().await {
                        Ok(Some(info)) => {
                            let song_changed = last_song_title.as_ref() != Some(&info.title);
                            last_song_title = Some(info.title.clone());

                            // Store the latest info (including playback state)
                            let mut current_state = state_clone.lock().unwrap();
                            current_state.current_info = Some(info.clone());
                            drop(current_state);

                            if song_changed {
                                let artists_str = info.artists.join(", "); // For display/logging
                                println!("New song detected: {} - {}", artists_str, info.title);
                                let mut current_state = state_clone.lock().unwrap();
                                current_state.lyrics = "".to_string(); // Clear lyrics immediately
                                current_state.status = format!("Looking for lyrics for {} - {}...", artists_str, info.title);
                                drop(current_state);

                                // --- Check Cache First ---
                                let cached_lyrics = cache::get_lyrics_from_cache(&info.artists, &info.title);

                                if let Some(lyrics) = cached_lyrics {
                                     // Found in cache
                                     let mut current_state = state_clone.lock().unwrap();
                                     current_state.lyrics = lyrics;
                                     current_state.status = format!("Showing lyrics for {} - {} (Cached)", artists_str, info.title);
                                } else {
                                     // Not in cache, fetch from Genius
                                     current_state = state_clone.lock().unwrap(); // Re-acquire lock
                                     current_state.status = format!("Fetching lyrics for {} - {} (Web)...", artists_str, info.title);
                                     drop(current_state);

                                     match lyrics::fetch_and_parse_lyrics(&info.artists, &info.title).await {
                                        Ok(cleaned_lyrics) => {
                                            // Store in cache *before* updating UI state
                                            cache::store_lyrics_to_cache(&info.artists, &info.title, &cleaned_lyrics);

                                            let mut current_state = state_clone.lock().unwrap();
                                            current_state.lyrics = cleaned_lyrics;
                                            current_state.status = format!("Showing lyrics for {} - {}", artists_str, info.title);
                                        }
                                        Err(e) => {
                                            println!("Lyrics fetch/parse error: {}", e); // Log error
                                            let mut current_state = state_clone.lock().unwrap();
                                            current_state.lyrics = format!("Error fetching/parsing lyrics:\n{}", e); // Show error in GUI
                                            current_state.status = "Error".to_string();
                                        }
                                     }
                                }
                            } else {
                                // Song unchanged, update status based on actual playback state
                                let mut current_state = state_clone.lock().unwrap();
                                let is_playing = current_state.current_info.as_ref().map_or(false, |info| info.is_playing);

                                if is_playing && !current_state.status.starts_with("Showing lyrics") && !current_state.status.starts_with("Error") {
                                     current_state.status = "Song unchanged.".to_string();
                                } else if !is_playing && current_state.current_info.is_some() { // Check if info exists before declaring paused
                                     current_state.status = "Spotify paused.".to_string();
                                }
                                // TODO: Could update a progress bar here
                            }
                        }
                        Ok(None) => { // Nothing playing according to API
                            if last_song_title.is_some() {
                                println!("Spotify stopped or nothing playing.");
                                last_song_title = None;
                                let mut current_state = state_clone.lock().unwrap();
                                current_state.current_info = None;
                                current_state.lyrics = "".to_string();
                                current_state.status = "Spotify stopped or nothing playing.".to_string();
                            } else {
                                 let mut current_state = state_clone.lock().unwrap();
                                 if current_state.current_info.is_some() || current_state.status != "Spotify stopped or nothing playing." {
                                     current_state.current_info = None;
                                     current_state.lyrics = "".to_string();
                                     current_state.status = "Spotify stopped or nothing playing.".to_string();
                                 }
                            }
                        }
                        Err(e) => { // Error getting info from Spotify API
                             println!("Error checking Spotify: {}", e);
                             last_song_title = None;
                             let mut current_state = state_clone.lock().unwrap();
                             current_state.current_info = None;
                             current_state.lyrics = "".to_string();
                             current_state.status = format!("Spotify API Error: {}", e);
                        }
                    }
                }); // End block_on

                // Poll interval (can be adjusted)
                std::thread::sleep(Duration::from_secs(3));
            }
        }); // End background thread spawn

        Self { state }
    }

    // Helper to apply transparency based on opacity
    fn apply_opacity(ctx: &egui::Context, opacity: f32) {
        let mut visuals = ctx.style().visuals.clone();
        // Convert f32 (0.0-1.0) to u8 (0-255) for alpha
        let alpha = (opacity.clamp(0.0, 1.0) * 255.0) as u8;

        // Set window and panel fill to have the calculated alpha
        // Create a new Color32 with the desired alpha
        let base_color = visuals.window_fill; // Get the original color
        visuals.window_fill = Color32::from_rgba_unmultiplied(base_color.r(), base_color.g(), base_color.b(), alpha);
        visuals.panel_fill = visuals.window_fill; // Make panel match window background

        // Ensure text and other widgets remain fully opaque regardless of background
        visuals.override_text_color = None; // Use default text color calculation
        let opaque_color = |c: Color32| Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 255);
        visuals.widgets.noninteractive.fg_stroke.color = opaque_color(visuals.widgets.noninteractive.fg_stroke.color);
        visuals.widgets.inactive.fg_stroke.color = opaque_color(visuals.widgets.inactive.fg_stroke.color);
        visuals.widgets.hovered.fg_stroke.color = opaque_color(visuals.widgets.hovered.fg_stroke.color);
        visuals.widgets.active.fg_stroke.color = opaque_color(visuals.widgets.active.fg_stroke.color);
        visuals.widgets.open.fg_stroke.color = opaque_color(visuals.widgets.open.fg_stroke.color);
        // Keep widget backgrounds opaque too? (Optional)
        // visuals.widgets.noninteractive.bg_fill = opaque_color(visuals.widgets.noninteractive.bg_fill);
        // ... etc ...

        ctx.set_visuals(visuals);
    }
}

impl eframe::App for LyricsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(500)); // Keep requesting repaints

        let mut current_state = self.state.lock().unwrap(); // Lock state for read/write

        // --- Opacity Slider ---
        // Place it before the main panel to potentially put it in a top bar later
        let mut new_opacity = current_state.opacity; // Copy value for slider
        egui::TopBottomPanel::top("config_panel").show(ctx, |ui| {
             ui.horizontal(|ui| {
                ui.label("Opacity:");
                // Use a slider to change the opacity value
                if ui.add(egui::Slider::new(&mut new_opacity, 0.0..=1.0).step_by(0.05)).changed() {
                    current_state.opacity = new_opacity; // Update state if slider moved
                    // Apply the new opacity immediately
                    Self::apply_opacity(ctx, new_opacity);
                }
             });
        });


        // --- Main Content Panel ---
        egui::CentralPanel::default().show(ctx, |ui| {
            // Display current song title and artists
            if let Some(info) = &current_state.current_info {
                 let artists_str = info.artists.join(", ");
                 ui.heading(format!("{} - {}", artists_str, info.title));
                 // TODO: Add playback progress bar here later
                 ui.separator();
            } else {
                 ui.heading("No song playing");
                 ui.separator();
            }

            // Display lyrics
            egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                ui.label(egui::RichText::new(&current_state.lyrics).size(14.0));
            });

             // Footer area for status
             ui.separator();
             ui.label(&current_state.status);
        });
    }
}