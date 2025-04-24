// Lyrics Caching Logic will go here
use std::{
    collections::BTreeMap, // Use BTreeMap for ordered iteration (needed for LRU)
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const CACHE_DIR_NAME: &str = ".lyricrs_cache";
const INDEX_FILE_NAME: &str = "index.json";
const MAX_CACHE_ENTRIES: usize = 500; // Limit cache size

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CacheEntry {
    filename: String,
    last_accessed: u64, // Unix timestamp (seconds)
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct CacheIndex {
    // Key: Unique identifier for the song (e.g., hash of "artist1,artist2 - title")
    // Value: CacheEntry
    entries: BTreeMap<String, CacheEntry>,
}

// --- Cache State ---
// Using a simple Mutex for now. For heavy concurrency, RwLock might be better.
static CACHE_INDEX: Mutex<Option<CacheIndex>> = Mutex::new(None);
static CACHE_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

// --- Helper Functions ---

fn get_cache_dir() -> Result<PathBuf, io::Error> {
    let mut cache_dir_guard = CACHE_DIR.lock().unwrap();
    if let Some(ref path) = *cache_dir_guard {
        return Ok(path.clone());
    }

    // Try to get user's cache directory or fallback to project dir
    let base_path = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".")); // Fallback to current dir if system cache dir fails

    let path = base_path.join(CACHE_DIR_NAME);
    fs::create_dir_all(&path)?; // Ensure directory exists
    *cache_dir_guard = Some(path.clone());
    Ok(path)
}


fn get_index_path() -> Result<PathBuf, io::Error> {
    Ok(get_cache_dir()?.join(INDEX_FILE_NAME))
}

fn generate_key(artists: &[String], title: &str) -> String {
    let combined = format!("{} - {}", artists.join(", "), title);
    let mut hasher = Sha256::new();
    hasher.update(combined.as_bytes());
    let result = hasher.finalize();
    // Use hex encoding for the key (more readable than raw bytes)
    hex::encode(result)
}

fn generate_filename(key: &str) -> String {
    format!("{}.txt", key)
}

fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn load_index() -> Result<CacheIndex, io::Error> {
    let index_path = get_index_path()?;
    if !index_path.exists() {
        return Ok(CacheIndex::default()); // Return empty index if file doesn't exist
    }

    let content = fs::read_to_string(&index_path)?;
    serde_json::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn save_index(index: &CacheIndex) -> Result<(), io::Error> {
    let index_path = get_index_path()?;
    let content = serde_json::to_string_pretty(index)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let mut file = fs::File::create(&index_path)?;
    file.write_all(content.as_bytes())
}

// --- Public Cache API ---

pub fn init_cache() -> Result<(), io::Error> {
    println!("Initializing lyrics cache...");
    let mut index_guard = CACHE_INDEX.lock().unwrap();
    if index_guard.is_some() {
        println!("Cache already initialized.");
        return Ok(());
    }
    // Ensure cache dir exists (called implicitly by load_index via get_index_path)
    let index = load_index()?;
    println!("Loaded {} cache entries.", index.entries.len());
    *index_guard = Some(index);
    Ok(())
}

pub fn get_lyrics_from_cache(artists: &[String], title: &str) -> Option<String> {
    let key = generate_key(artists, title);
    let mut index_guard = CACHE_INDEX.lock().unwrap();

    if let Some(ref mut index) = *index_guard {
        if let Some(entry) = index.entries.get_mut(&key) {
            println!("Cache hit for: {} - {}", artists.join(", "), title);
            // Update access time
            entry.last_accessed = get_current_timestamp();
            let filename = entry.filename.clone(); // Clone filename before saving index

            // Save index immediately after updating timestamp
            if let Err(e) = save_index(index) {
                eprintln!("Error saving cache index after timestamp update: {}", e);
                // Continue anyway, try to read the file
            }

            // Read lyrics file
            match get_cache_dir() {
                Ok(cache_dir) => {
                    let file_path = cache_dir.join(filename);
                    match fs::read_to_string(&file_path) {
                        Ok(lyrics) => Some(lyrics),
                        Err(e) => {
                            eprintln!("Cache index points to file '{}', but failed to read it: {}", file_path.display(), e);
                            // Consider removing the invalid entry here?
                            None
                        }
                    }
                }
                Err(e) => {
                     eprintln!("Failed to get cache directory while reading lyrics: {}", e);
                     None
                }
            }
        } else {
            println!("Cache miss for: {} - {}", artists.join(", "), title);
            None // Not found in index
        }
    } else {
        eprintln!("Cache not initialized, cannot get lyrics.");
        None // Cache not initialized
    }
}

pub fn store_lyrics_to_cache(artists: &[String], title: &str, lyrics: &str) {
    let key = generate_key(artists, title);
    let filename = generate_filename(&key);
    let timestamp = get_current_timestamp();

    let mut index_guard = CACHE_INDEX.lock().unwrap();

    if let Some(ref mut index) = *index_guard {
         // Write the lyrics file first
         match get_cache_dir() {
            Ok(cache_dir) => {
                let file_path = cache_dir.join(&filename);
                match fs::write(&file_path, lyrics) {
                    Ok(_) => {
                         println!("Successfully wrote lyrics to cache file: {}", file_path.display());
                         // Now update the index
                         let new_entry = CacheEntry {
                            filename,
                            last_accessed: timestamp,
                         };
                         index.entries.insert(key, new_entry);

                         // --- LRU Eviction ---
                         if index.entries.len() > MAX_CACHE_ENTRIES {
                            // BTreeMap iterates in sorted key order, but we need LRU (oldest timestamp)
                            if let Some((evict_key, _)) = index.entries.iter().min_by_key(|(_, entry)| entry.last_accessed) {
                                let evict_key = evict_key.clone(); // Clone key to remove later
                                println!("Cache limit reached. Evicting oldest entry: {}", evict_key);
                                if let Some(evicted_entry) = index.entries.remove(&evict_key) {
                                     // Delete the associated lyrics file
                                     let evict_file_path = cache_dir.join(evicted_entry.filename);
                                     if let Err(e) = fs::remove_file(&evict_file_path) {
                                         eprintln!("Failed to delete evicted cache file '{}': {}", evict_file_path.display(), e);
                                     }
                                }
                            }
                         }
                         // --- End LRU Eviction ---

                         // Save the updated index
                         if let Err(e) = save_index(index) {
                            eprintln!("Error saving cache index after storing lyrics: {}", e);
                         }
                    }
                    Err(e) => {
                        eprintln!("Failed to write lyrics to cache file '{}': {}", file_path.display(), e);
                    }
                }
            }
            Err(e) => {
                 eprintln!("Failed to get cache directory while storing lyrics: {}", e);
            }
         }
    } else {
        eprintln!("Cache not initialized, cannot store lyrics.");
    }
}