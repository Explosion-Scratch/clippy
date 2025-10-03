mod clipboard;

use tauri::Listener;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            clipboard::start_clipboard_listener,
            clipboard::stop_clipboard_listener,
            clipboard::get_clipboard_status
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Automatically start clipboard listener on app load
            if let Err(e) = clipboard::start_listen(app_handle.clone()) {
                eprintln!("Failed to start clipboard listener on startup: {}", e);
            } else {
                println!("Clipboard listener started automatically on app startup");
            }

            // Listen for start-listen events from frontend
            app.listen("start-listen", {
                let app_handle = app_handle.clone();
                move |_event| {
                    println!("Received start-listen event from frontend");
                    if let Err(e) = clipboard::start_listen(app_handle.clone()) {
                        eprintln!("Failed to start clipboard listener: {}", e);
                    } else {
                        println!("Clipboard listener started via event");
                    }
                }
            });

            // Listen for stop-listen events from frontend
            app.listen("stop-listen", move |_event| {
                println!("Received stop-listen event from frontend");
                if let Err(e) = clipboard::stop_listen() {
                    eprintln!("Failed to stop clipboard listener: {}", e);
                } else {
                    println!("Clipboard listener stopped via event");
                }
            });

            // Listen for change-clipboard events to log clipboard changes
            app.listen("change-clipboard", move |event| {
                if let Ok(payload) = serde_json::from_str::<serde_json::Value>(event.payload()) {
                    if let Some(item) = payload.get("item") {
                        let timestamp = item.get("timestamp")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let byte_size = item.get("byteSize")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize;

                        println!("Clipboard item changed at timestamp {} ({} bytes)", timestamp, byte_size);

                        // Log available formats
                        if let Some(formats) = item.get("formats") {
                            if let Some(txt) = formats.get("txt").and_then(|v| v.as_str()) {
                                println!("  Text: {} chars", txt.len());
                            }
                            if let Some(html) = formats.get("html").and_then(|v| v.as_str()) {
                                println!("  HTML: {} chars", html.len());
                            }
                            if let Some(rtf) = formats.get("rtf").and_then(|v| v.as_str()) {
                                println!("  RTF: {} chars", rtf.len());
                            }
                            if let Some(image_data) = formats.get("imageData").and_then(|v| v.as_str()) {
                                println!("  Image: {} chars (base64)", image_data.len());
                            }
                            if let Some(files) = formats.get("files").and_then(|v| v.as_array()) {
                                println!("  Files: {} items", files.len());
                            }
                            if let Some(custom_formats) = formats.get("customFormats").and_then(|v| v.as_object()) {
                                println!("  Custom formats: {}", custom_formats.len());
                            }
                        }
                    } else {
                        println!("Clipboard changed (no item data found)");
                    }
                } else {
                    println!("Clipboard changed (unable to parse payload)");
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
