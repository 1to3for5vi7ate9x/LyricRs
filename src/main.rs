#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::{egui, NativeOptions};
use dotenv::dotenv; // Import dotenv

// Declare modules
mod app;
mod lyrics;
mod spotify;
mod cache; // Declare cache module

#[tokio::main] // Make main async
async fn main() -> Result<(), Box<dyn std::error::Error>> { // Return Box<dyn Error>
    // Load environment variables from .env file
    dotenv().ok();

    println!("Starting Spotify Lyrics Overlay...");

    // Initialize the Spotify client (await the async function)
    spotify::init_client().await?; // Use .await and ?

    // Configure viewport settings (size, always_on_top, transparency)
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([400.0, 600.0])
        .with_always_on_top() // Keep always on top
        .with_transparent(true); // Use egui's transparency setting

    let options = NativeOptions {
        viewport,
        // Remove window_builder for now
        ..Default::default()
    };

    // Run the eframe application defined in app.rs
    eframe::run_native(
        "Spotify Lyrics Overlay", // Window title
        options,
        Box::new(|cc| Box::new(app::LyricsApp::new(cc))),
    )?; // Use ? to propagate eframe errors

    Ok(()) // Return Ok if everything ran successfully
}
