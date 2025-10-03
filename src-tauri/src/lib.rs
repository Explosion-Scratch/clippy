use tauri::Listener;

mod structs;
mod clipboard;
mod db;

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
            clipboard::get_clipboard_status,
            db::db_save_item,
            db::db_recent_items,
            db::db_search,
            db::db_delete_item,
            db::db_get_count
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

            // Listen for change-clipboard events to log clipboard changes and save to database
            app.listen("change-clipboard", move |event| {
                if let Ok(payload) = serde_json::from_str::<serde_json::Value>(event.payload()) {
                    if let Some(item) = payload.get("item") {
                        let timestamp = item.get("timestamp")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let byte_size = item.get("byteSize")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize;

                        // Extract searchable text (prioritize plain text)
                        let searchable_text = item.get("formats")
                            .and_then(|f| f.get("txt"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        // Create ClipboardItem for database storage
                        if let Ok(formats) = serde_json::from_value::<structs::ClipboardFormats>(
                            item.get("formats").unwrap_or(&serde_json::Value::Object(Default::default())).clone()
                        ) {
                            let clipboard_item = structs::ClipboardItem {
                                id: 0, // Will be assigned by database
                                text: searchable_text,
                                timestamp,
                                byte_size,
                                formats,
                            };

                            // Save to database asynchronously
                            let app_handle_clone = app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) = db::db_save_item(app_handle_clone, clipboard_item) {
                                    eprintln!("Failed to save clipboard item to database: {}", e);
                                }
                            });
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
