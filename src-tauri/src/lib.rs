use std::sync::Arc;
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
use tauri_plugin_shell::ShellExt;
use tokio::sync::Mutex;
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};

mod accessibility;
mod api;
mod clipboard;
mod paste;
mod polling;
mod preview;
mod settings;
mod sidecar;
mod tray;
mod visibility;
mod windows;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn unregister_main_shortcut(app: tauri::AppHandle) -> Result<(), String> {
    let shortcut_str = settings::get_configured_shortcut(app.clone())?;
    let main_shortcut = settings::parse_shortcut(&shortcut_str)?;
    if let Err(e) = app.global_shortcut().unregister(main_shortcut) {
        return Err(format!("Failed to unregister shortcut: {}", e));
    }
    println!("{} shortcut unregistered", shortcut_str);
    Ok(())
}

#[tauri::command]
fn register_main_shortcut(app: tauri::AppHandle) -> Result<(), String> {
    let shortcut_str = settings::get_configured_shortcut(app.clone())?;
    let main_shortcut = settings::parse_shortcut(&shortcut_str)?;
    if let Err(e) = app.global_shortcut().register(main_shortcut) {
        return Err(format!("Failed to register shortcut: {}", e));
    }
    println!("{} shortcut registered", shortcut_str);
    Ok(())
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle) -> Result<(), String> {
    windows::open_settings_window(app).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_macos_permissions::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .on_window_event(|window, event| {
            if window.label() == "settings" || window.label() == "welcome" {
                if let tauri::WindowEvent::Destroyed = event {
                    windows::handle_window_destroyed(window);
                }
            }
            if window.label() == "main" {
                match event {
                    tauri::WindowEvent::Focused(false) => {
                        windows::handle_main_window_focus_lost(window);
                    }
                    tauri::WindowEvent::Destroyed => {
                        windows::handle_main_window_destroyed(window);
                    }
                    _ => {}
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            unregister_main_shortcut,
            register_main_shortcut,
            open_settings,
            preview::preview_item,
            preview::hide_preview,
            preview::focus_preview,
            preview::is_preview_visible,
            preview::get_preview_content,
            preview::get_item_data,
            preview::get_item_text,
            preview::open_in_dashboard,
            sidecar::init_service,
            sidecar::stop_service,
            sidecar::get_service_status,
            sidecar::get_history,
            sidecar::get_mtime,
            sidecar::copy_item,
            sidecar::paste_item,
            sidecar::paste_item_plain_text,
            sidecar::delete_item,
            sidecar::configure_data_dir,
            clipboard::write_to_clipboard,
            sidecar::db_get_count,
            sidecar::db_get_size,
            sidecar::db_export_all,
            sidecar::db_import_all,
            sidecar::db_delete_all,
            sidecar::get_sidecar_dir,
            sidecar::set_sidecar_dir,
            sidecar::restart_api,
            sidecar::get_app_data_dir,
            sidecar::edit_item,
            paste::simulate_system_paste,
            visibility::is_visible,
            visibility::hide_app,
            visibility::show_app,
            accessibility::check_permissions,
            accessibility::request_permissions,
            settings::get_settings,
            settings::set_settings,
            settings::check_first_run,
            settings::check_welcome_shown,
            settings::get_configured_shortcut,
            settings::add_cli_to_path,
        ])
        .setup(|app| {
            use tauri_plugin_global_shortcut::ShortcutState;

            let app_handle = app.handle().clone();

            let shortcut_str = settings::get_configured_shortcut(app_handle.clone())
                .unwrap_or_else(|_| settings::AppSettings::default_shortcut());
            let main_shortcut = settings::parse_shortcut(&shortcut_str)
                .unwrap_or_else(|_| Shortcut::new(Some(Modifiers::CONTROL), Code::KeyP));
            println!("Configured shortcut: {}", shortcut_str);

            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let window = app_handle
                .get_webview_window("main")
                .ok_or("Failed to get main window during setup")?;

            let tray_items: tray::TrayClipboardItems = Arc::new(Mutex::new(Vec::new()));

            tray::setup_tray(app, tray_items, windows::open_settings_window)?;

            app_handle.plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler({
                        move |app_handle, _shortcut, event| {
                            if event.state() == ShortcutState::Pressed {
                                println!("Shortcut pressed - showing window");
                                if let Err(e) = visibility::show(app_handle.clone()) {
                                    eprintln!("Failed to show window: {}", e);
                                }
                            }
                        }
                    })
                    .build(),
            )?;
            app.global_shortcut().register(main_shortcut)?;

            #[cfg(target_os = "macos")]
            {
                apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, None)
                    .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");
            }

            #[cfg(target_os = "macos")]
            {
                let app_handle_clone = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    println!("Checking accessibility permissions on startup...");
                    match accessibility::ensure_accessibility_permissions().await {
                        Ok(has_permissions) => {
                            if !has_permissions {
                                println!("Accessibility permissions not granted, showing alert...");
                                if let Err(e) =
                                    accessibility::show_permissions_alert(app_handle_clone.clone())
                                        .await
                                {
                                    eprintln!("Failed to show permissions alert: {}", e);
                                }
                            } else {
                                match settings::check_welcome_shown(app_handle_clone.clone()) {
                                    Ok(shown) => {
                                        if !shown {
                                            println!(
                                                "Welcome not yet shown, opening welcome window..."
                                            );
                                            if let Err(e) =
                                                windows::open_welcome_window(app_handle_clone.clone())
                                            {
                                                eprintln!("Failed to open welcome window: {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to check welcome_shown: {}", e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to check accessibility permissions: {}", e);
                            if let Err(e) =
                                accessibility::show_permissions_alert(app_handle_clone.clone())
                                    .await
                            {
                                eprintln!("Failed to show permissions alert: {}", e);
                            }
                        }
                    }
                });
            }

            let app_handle_clone = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                println!("Initializing clipboard watcher service...");
                if let Err(e) = sidecar::init_service(app_handle_clone.clone()).await {
                    eprintln!("Failed to initialize service: {}", e);
                } else {
                    println!("Clipboard watcher service initialized successfully");
                }
            });

            let app_handle_clone = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                println!("Starting API sidecar...");
                let sidecar = app_handle_clone.shell().sidecar("get_clipboard");
                if let Ok(cmd) = sidecar {
                    let result = cmd
                        .args(["api", "--port", &api::API_PORT.to_string()])
                        .spawn();
                    match result {
                        Ok((_rx, _child)) => println!("API sidecar started successfully"),
                        Err(e) => eprintln!("Failed to spawn API sidecar: {}", e),
                    }
                } else {
                    eprintln!("Failed to find sidecar");
                }
            });

            polling::start_mtime_polling(app_handle);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
