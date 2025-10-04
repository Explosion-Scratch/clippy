use tauri::{Listener, Manager};
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};

mod structs;
mod clipboard;
mod db;
mod paste;
mod visibility;
mod accessibility;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_macos_permissions::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            clipboard::start_clipboard_listener,
            clipboard::stop_clipboard_listener,
            clipboard::get_clipboard_status,
            clipboard::set_clipboard_item,
            clipboard::inject_item,
            db::db_save_item,
            db::db_recent_items,
            db::db_search,
            db::db_delete_item,
            db::db_get_item_by_id,
            db::db_get_count,
            db::db_flush,
            paste::simulate_system_paste,
            visibility::is_visible,
            visibility::hide_app,
            visibility::show_app,
            accessibility::check_permissions,
            accessibility::request_permissions,
        ])
        .setup(|app| {
            use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

            let main_shortcut = Shortcut::new(Some(Modifiers::CONTROL), Code::KeyP);

            app.set_activation_policy(tauri::ActivationPolicy::Regular);

            let app_handle = app.handle().clone();
            let window = app_handle.get_webview_window("main").unwrap();

            /* Shorcut */
            app_handle.plugin(tauri_plugin_global_shortcut::Builder::new().with_handler({
                let _app_handle = app_handle.clone();
                move |app_handle, shortcut, event| {
                    if shortcut == &main_shortcut && event.state() == ShortcutState::Pressed {
                        println!("{:?}", shortcut);
                        println!("Show window here");
                        if let Err(e) = visibility::show(app_handle.clone()) {
                            eprintln!("Failed to show window: {}", e);
                        }
                    }
                }
            }).build())?;
            app.global_shortcut().register(main_shortcut)?;

            #[cfg(target_os = "macos")]
            {
                apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, None)
                    .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");

                // Hide app from dock
                if let Err(e) = visibility::hide(app.handle()) {
                    eprintln!("Failed to hide app on startup: {}", e);
                }
            }

            // Check accessibility permissions on startup
            #[cfg(target_os = "macos")]
            {
                let app_handle_clone = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    println!("Checking accessibility permissions on startup...");
                    match accessibility::ensure_accessibility_permissions().await {
                        Ok(has_permissions) => {
                            if !has_permissions {
                                println!("Accessibility permissions not granted, showing alert...");
                                if let Err(e) = accessibility::show_permissions_alert(app_handle_clone.clone()).await {
                                    eprintln!("Failed to show permissions alert: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to check accessibility permissions: {}", e);
                            // Show alert anyway to guide user
                            if let Err(e) = accessibility::show_permissions_alert(app_handle_clone.clone()).await {
                                eprintln!("Failed to show permissions alert: {}", e);
                            }
                        }
                    }
                });
            }

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

            // Listen for change-clipboard events to save items to database
            app.listen("change-clipboard", move |event| {
                println!("Clipboard changed - saving to database");

                if let Ok(payload) = serde_json::from_str::<serde_json::Value>(event.payload()) {
                    if let Some(item) = payload.get("item") {
                        // Parse the clipboard item from the event
                        if let Ok(clipboard_item) = serde_json::from_value::<structs::ClipboardItem>(item.clone()) {
                            // Save to database asynchronously
                            let app_handle_clone = app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                match db::db_save_item(app_handle_clone, clipboard_item) {
                                    Ok(result) => {
                                        if result.success {
                                            println!("Successfully saved clipboard item with ID: {:?}", result.id);
                                        } else {
                                            eprintln!("Failed to save clipboard item: {:?}", result.error);
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Error saving clipboard item: {}", e);
                                    }
                                }
                            });
                        } else {
                            eprintln!("Failed to parse clipboard item from event payload");
                        }
                    } else {
                        println!("Clipboard changed but no item data found in payload");
                    }
                } else {
                    eprintln!("Failed to parse clipboard change event payload");
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
