use clipboard_rs::{
    common::RustImage, Clipboard, ClipboardContext, ClipboardHandler, ClipboardWatcher, ClipboardWatcherContext, ContentFormat
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
                                total_size += files.iter().map(|f| f.as_bytes().len() as u64).sum::<u64>();
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
            timestamp,
            byte_size: total_size,
            formats,
        }
    }
}

impl ClipboardHandler for Manager {
    fn on_clipboard_change(&mut self) {
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
