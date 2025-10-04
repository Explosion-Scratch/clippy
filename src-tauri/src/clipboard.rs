use clipboard_rs::{
    common::{RustImage, ClipboardContent}, Clipboard, ClipboardContext, ClipboardHandler, ClipboardWatcher, ClipboardWatcherContext, ContentFormat
};
use crate::structs::{ClipboardItem, ClipboardFormats, ClipboardChangeEvent};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};
use base64::{Engine as _, engine::general_purpose};

// Global state to track if clipboard listening is active
static IS_LISTENING: AtomicBool = AtomicBool::new(false);

struct Manager {
    ctx: ClipboardContext,
    app_handle: AppHandle,
}

impl Manager {
    pub fn new(app_handle: AppHandle) -> Self {
        let ctx = ClipboardContext::new().expect("Failed to create clipboard context");
        Manager { ctx, app_handle }
    }

    fn extract_clipboard_item(&mut self) -> ClipboardItem {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut formats = ClipboardFormats::default();
        let mut total_size: u64 = 0;
        let mut searchable_text = None;

        // Get available formats
        let available_formats = match self.ctx.available_formats() {
            Ok(formats) => formats,
            Err(e) => {
                eprintln!("Failed to get available formats: {}", e);
                vec![]
            }
        };

        // Extract text content
        if self.ctx.has(ContentFormat::Text) {
            if let Ok(text) = self.ctx.get_text() {
                formats.txt = Some(text.clone());
                searchable_text = Some(text.clone());
                total_size += text.len() as u64; // Size in bytes (assuming UTF-8)
            }
        }

        // Extract HTML content
        if self.ctx.has(ContentFormat::Html) {
            if let Ok(html) = self.ctx.get_html() {
                total_size += html.len() as u64;
                formats.html = Some(html);
            }
        }

        // Extract RTF content
        if self.ctx.has(ContentFormat::Rtf) {
            if let Ok(rtf) = self.ctx.get_rich_text() {
                total_size += rtf.len() as u64;
                formats.rtf = Some(rtf);
            }
        }

        // Extract image data
        if self.ctx.has(ContentFormat::Image) {
            if let Ok(_image) = self.ctx.get_image() {
                if let Ok(png) = _image.to_png() {
                    let bytes = png.get_bytes();
                    let base64_string = general_purpose::STANDARD.encode(bytes);
                    let full_data_uri = format!("data:{};base64,{}", "image/png", base64_string);
                    formats.image_data = Some(full_data_uri.clone());
                    total_size += full_data_uri.len() as u64; // Use actual stored size (including base64 overhead)
                }
            }
        }

        // Handle custom formats
        let mut custom_formats = std::collections::HashMap::new();
        for format in available_formats {
            match format.as_str() {
                // Skip formats we've already handled
                "public.text" | "public.utf8-plain-text" | "public.html" | "public.rtf" | "public.png" | "public.tiff" => continue,

                // Handle file lists
                "public.file-url" => {
                    if let Ok(buffer) = self.ctx.get_buffer(&format) {
                        if let Ok(file_urls) = String::from_utf8(buffer) {
                            // Parse file URLs and convert to paths
                            let files: Vec<String> = file_urls
                                .split('\n')
                                .filter(|s| !s.is_empty())
                                .filter_map(|s| {
                                    // Remove file:// prefix
                                    s.trim_start_matches("file://")
                                        .trim_start_matches("file:/")
                                        .to_string().into()
                                })
                                .collect();
                            if !files.is_empty() {
                                // Count the file path strings themselves (not file content size)
                                total_size += files.iter().map(|f| f.len() as u64).sum::<u64>();
                                formats.files = Some(files);
                            }
                        }
                    }
                }
                _ => {
                    // Handle other custom formats
                    if let Ok(buffer) = self.ctx.get_buffer(&format) {
                        // Try to convert to string, otherwise store as base64
                        if let Ok(text) = String::from_utf8(buffer.clone()) {
                            total_size += text.len() as u64;
                            custom_formats.insert(format.clone(), text);
                        } else {
                            let base64_data = general_purpose::STANDARD.encode(&buffer);
                            total_size += base64_data.len() as u64; // Use actual stored size
                            custom_formats.insert(format, base64_data);
                        }
                    }
                }
            }
        }

        if !custom_formats.is_empty() {
            formats.custom_formats = Some(custom_formats);
        }

        ClipboardItem {
            id: 0, // Will be assigned by database
            text: searchable_text,
            timestamp, // Last copied timestamp
            first_copied: timestamp, // Initially same as timestamp
            copies: 1, // Initially 1 copy
            byte_size: total_size,
            formats,
        }
    }
}

impl ClipboardHandler for Manager {
    fn on_clipboard_change(&mut self) {
        if !IS_LISTENING.load(Ordering::Relaxed) {
            return
        }
        let item = self.extract_clipboard_item();
        let event = ClipboardChangeEvent { item };

        if let Err(e) = self.app_handle.emit("change-clipboard", &event) {
            eprintln!("Failed to emit clipboard change event: {}", e);
        }
    }
}

pub fn start_listen(app_handle: AppHandle) -> Result<(), String> {
    // Check if already listening
    if IS_LISTENING.load(Ordering::Relaxed) {
        return Ok(());
    }

    // Set listening flag
    IS_LISTENING.store(true, Ordering::Relaxed);

    // Create manager and watcher
    let manager = Manager::new(app_handle);
    let mut watcher = ClipboardWatcherContext::new()
        .map_err(|e| format!("Failed to create clipboard watcher: {}", e))?;

    watcher.add_handler(manager);

    // Start watching in a separate thread
    std::thread::spawn(move || {
        println!("Starting clipboard watcher");
        watcher.start_watch();
        println!("Clipboard watcher stopped");
        IS_LISTENING.store(false, Ordering::Relaxed);
    });

    println!("Clipboard listener started");
    Ok(())
}

pub fn stop_listen() -> Result<(), String> {
    if !IS_LISTENING.load(Ordering::Relaxed) {
        return Err("No clipboard watcher is currently running".to_string());
    }

    IS_LISTENING.store(false, Ordering::Relaxed);
    println!("Clipboard watcher will stop on next iteration");
    Ok(())
}

pub fn is_listening() -> bool {
    IS_LISTENING.load(Ordering::Relaxed)
}

// Tauri commands for manual control
#[tauri::command]
pub fn start_clipboard_listener(app_handle: AppHandle) -> Result<String, String> {
    match start_listen(app_handle) {
        Ok(()) => Ok("Clipboard listener started".to_string()),
        Err(e) => Err(format!("Failed to start clipboard listener: {}", e)),
    }
}

#[tauri::command]
pub fn stop_clipboard_listener() -> Result<String, String> {
    match stop_listen() {
        Ok(()) => Ok("Clipboard listener stopped".to_string()),
        Err(e) => Err(format!("Failed to stop clipboard listener: {}", e)),
    }
}

#[tauri::command]
pub fn get_clipboard_status() -> bool {
    is_listening()
}

/// Inject clipboard item by ID - sets clipboard and pastes, with listening control
#[tauri::command]
pub fn inject_item(app_handle: AppHandle, id: u64) -> Result<String, String> {
    println!("=== INJECTING CLIPBOARD ITEM BY ID ===");
    println!("Item ID: {}", id);

    // Check if we're currently listening and pause if needed
    let was_listening = is_listening();
    if was_listening {
        println!("Pausing clipboard listener for injection");
        if let Err(e) = stop_listen() {
            eprintln!("Failed to pause clipboard listener: {}", e);
        }
    }

    // Hide the window and app before setting clipboard
    if let Err(e) = crate::visibility::hide(&app_handle) {
        eprintln!("Failed to hide window: {}", e);
    }

    // Set the clipboard content
    let set_result = set_clipboard_item_internal(app_handle.clone(), id);

    // If clipboard was set successfully, increment copies and simulate paste
    match set_result {
        Ok(_) => {
            println!("Clipboard set successfully, incrementing copies counter");
            
            // Increment the copies counter for this item
            if let Err(e) = crate::db::db_increment_copies(app_handle.clone(), id) {
                eprintln!("Failed to increment copies counter: {}", e);
                // Don't fail the operation, just log the error
            }
            
            println!("Waiting 0.2s before simulating paste");
            std::thread::sleep(std::time::Duration::from_millis(200));
            if let Err(e) = crate::paste::simulate_system_paste_internal(&app_handle) {
                eprintln!("Failed to simulate paste: {}", e);
                return Err(format!("Failed to simulate paste: {}", e));
            }
        }
        Err(e) => {
            eprintln!("Failed to set clipboard: {}", e);
            return Err(e);
        }
    }

    // Resume listening if it was active before (with 1s delay)
    if was_listening {
        println!("Resuming clipboard listener after injection with 1s delay");
        let app_handle_clone = app_handle.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(1));
            if let Err(e) = start_listen(app_handle_clone) {
                eprintln!("Failed to resume clipboard listener: {}", e);
            }
        });
    }

    println!("Item injection completed successfully");
    Ok("Item injected successfully".to_string())
}

/// Reconstruct and set clipboard content from a stored ClipboardItem by ID
#[tauri::command]
pub fn set_clipboard_item(app_handle: AppHandle, id: u64) -> Result<String, String> {
    set_clipboard_item_internal(app_handle, id)
}

/// Internal function to set clipboard content without listening control
fn set_clipboard_item_internal(app_handle: AppHandle, id: u64) -> Result<String, String> {
    println!("=== SETTING CLIPBOARD FROM DATABASE ITEM BY ID ===");
    println!("Item ID: {}", id);

    // Fetch the item from database
    let item = match crate::db::db_get_item_by_id(app_handle, id) {
        Ok(item) => {
            println!("Successfully fetched item from database");
            println!("Item timestamp: {}", item.timestamp);
            item
        },
        Err(e) => {
            println!("Failed to fetch item from database: {}", e);
            return Err(format!("Failed to fetch item from database: {}", e));
        }
    };

    let ctx = ClipboardContext::new()
        .map_err(|e| format!("Failed to create clipboard context: {}", e))?;

    let mut contents = Vec::new();

    // Set text content if available
    if let Some(text) = &item.formats.txt {
        contents.push(ClipboardContent::Text(text.clone()));
        println!("Added text content: {} chars", text.len());
    }

    // Set HTML content if available
    if let Some(html) = &item.formats.html {
        contents.push(ClipboardContent::Html(html.clone()));
        println!("Added HTML content: {} chars", html.len());
    }

    // Set RTF content if available
    if let Some(rtf) = &item.formats.rtf {
        contents.push(ClipboardContent::Rtf(rtf.clone()));
        println!("Added RTF content: {} chars", rtf.len());
    }

    // Set image content if available
    if let Some(image_data_uri) = &item.formats.image_data {
        // Parse data URI: data:image/png;base64,xxxxx
        if let Some(base64_data) = image_data_uri.strip_prefix("data:image/png;base64,") {
            match base64::engine::general_purpose::STANDARD.decode(base64_data) {
                Ok(image_bytes) => {
                    // Create a temporary image file to load with clipboard-rs
                    use std::io::Write;
                    let temp_path = std::env::temp_dir().join("clippy_temp_image.png");
                    {
                        let mut temp_file = std::fs::File::create(&temp_path)
                            .map_err(|e| format!("Failed to create temp image file: {}", e))?;
                        temp_file.write_all(&image_bytes)
                            .map_err(|e| format!("Failed to write temp image file: {}", e))?;
                    }

                    // Load image using clipboard-rs
                    match clipboard_rs::common::RustImageData::from_path(temp_path.to_str().ok_or("Invalid temp file path")?) {
                        Ok(image_data) => {
                            contents.push(ClipboardContent::Image(image_data));
                            println!("Added image content: {} bytes", image_bytes.len());
                        }
                        Err(e) => {
                            println!("Failed to load image for clipboard: {}", e);
                            // Clean up temp file
                            let _ = std::fs::remove_file(&temp_path);
                            return Err(format!("Failed to load image for clipboard: {}", e));
                        }
                    }

                    // Clean up temp file
                    let _ = std::fs::remove_file(&temp_path);
                }
                Err(e) => {
                    println!("Failed to decode base64 image data: {}", e);
                    return Err(format!("Failed to decode base64 image data: {}", e));
                }
            }
        } else {
            println!("Invalid image data URI format");
            return Err("Invalid image data URI format".to_string());
        }
    }

    // Set files if available
    if let Some(files) = &item.formats.files {
        // Convert file paths to file URLs for clipboard
        let file_urls: Vec<String> = files.iter()
            .map(|path| {
                if path.starts_with("file://") {
                    path.clone()
                } else {
                    format!("file://{}", path)
                }
            })
            .collect();
        contents.push(ClipboardContent::Files(file_urls));
        println!("Added files: {} items", files.len());
    }

    // Set custom formats if available
    if let Some(custom_formats) = &item.formats.custom_formats {
        for (format_name, data) in custom_formats {
            // Try to detect if data is base64 encoded by checking if it fails to decode as UTF-8
            let decoded_data = match base64::engine::general_purpose::STANDARD.decode(data) {
                Ok(bytes) => {
                    // Successfully decoded as base64, use the bytes
                    bytes
                }
                Err(_) => {
                    // Not base64, treat as UTF-8 string
                    data.as_bytes().to_vec()
                }
            };
            let data_len = decoded_data.len();
            contents.push(ClipboardContent::Other(format_name.clone(), decoded_data));
            println!("Added custom format '{}': {} bytes", format_name, data_len);
        }
    }

    if contents.is_empty() {
        return Err("No content to set to clipboard".to_string());
    }

    // Clear clipboard first
    ctx.clear()
        .map_err(|e| format!("Failed to clear clipboard: {}", e))?;

    // Set all content at once
    ctx.set(contents)
        .map_err(|e| format!("Failed to set clipboard content: {}", e))?;

    println!("Clipboard set successfully with {} format(s)",
             if item.formats.txt.is_some() { 1 } else { 0 } +
             if item.formats.html.is_some() { 1 } else { 0 } +
             if item.formats.rtf.is_some() { 1 } else { 0 } +
             if item.formats.image_data.is_some() { 1 } else { 0 } +
             if item.formats.files.is_some() { 1 } else { 0 } +
             item.formats.custom_formats.as_ref().map_or(0, |f| f.len()));

    Ok("Clipboard item set successfully".to_string())
}
