use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
use tauri_plugin_shell::ShellExt;
use tokio::sync::Mutex;
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};

mod accessibility;
mod paste;
mod sidecar;
mod visibility;

// Shared state for tray menu clipboard items
type TrayClipboardItems = Arc<Mutex<Vec<(String, String)>>>; // (id, summary)

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// Command to unregister the main shortcut (Ctrl+P)
#[tauri::command]
fn unregister_main_shortcut(app: tauri::AppHandle) -> Result<(), String> {
    let main_shortcut = Shortcut::new(Some(Modifiers::CONTROL), Code::KeyP);
    if let Err(e) = app.global_shortcut().unregister(main_shortcut) {
        return Err(format!("Failed to unregister shortcut: {}", e));
    }
    println!("Ctrl+P shortcut unregistered");
    Ok(())
}

// Command to register the main shortcut (Ctrl+P)
#[tauri::command]
fn register_main_shortcut(app: tauri::AppHandle) -> Result<(), String> {
    let main_shortcut = Shortcut::new(Some(Modifiers::CONTROL), Code::KeyP);
    if let Err(e) = app.global_shortcut().register(main_shortcut) {
        return Err(format!("Failed to register shortcut: {}", e));
    }
    println!("Ctrl+P shortcut registered");
    Ok(())
}

// Tauri command to open settings window (callable from frontend)
#[tauri::command]
fn open_settings(app: tauri::AppHandle) -> Result<(), String> {
    open_settings_window(app).map_err(|e| e.to_string())
}

// Function to open settings window
fn open_settings_window(app_handle: tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::{Manager, WebviewWindowBuilder};

    // Show dock icon when opening settings
    #[cfg(target_os = "macos")]
    {
        app_handle.set_activation_policy(tauri::ActivationPolicy::Regular)?;
    }

    // Check if settings window already exists
    if let Some(settings_window) = app_handle.get_webview_window("settings") {
        // Settings window already exists, just show it and hide main
        if let Some(_main_window) = app_handle.get_webview_window("main") {
            visibility::hide(&app_handle).ok();
        }
        settings_window.set_focus()?;
        settings_window.show()?;
        return Ok(());
    }

    // Hide main window first
    if let Some(_main_window) = app_handle.get_webview_window("main") {
        visibility::hide(&app_handle).ok();
    }

    // Create new settings window
    let settings_window = WebviewWindowBuilder::new(
        &app_handle,
        "settings",
        tauri::WebviewUrl::App("/settings".into()),
    )
    .title("Clippy Settings")
    .inner_size(400.0, 450.0)
    .resizable(false)
    .minimizable(true)
    .maximizable(false)
    .visible(true)
    .focused(true)
    .build()?;

    // Ensure the settings window is shown and focused
    settings_window.show()?;
    settings_window.set_focus()?;

    // Apply vibrancy to settings window on macOS (must run on main thread)
    #[cfg(target_os = "macos")]
    {
        let app_handle_clone = app_handle.clone();
        app_handle.run_on_main_thread(move || {
            if let Some(settings_window) = app_handle_clone.get_webview_window("settings") {
                apply_vibrancy(
                    &settings_window,
                    NSVisualEffectMaterial::HudWindow,
                    None,
                    None,
                )
                .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");
            }
        })?;
    }

    Ok(())
}

// Command to show preview for an item
// Command to show preview for an item
#[tauri::command]
fn preview_item(app: tauri::AppHandle, id: String) -> Result<(), String> {
    use tauri::{LogicalSize, Manager};

    let preview_window = app
        .get_webview_window("preview")
        .ok_or("Failed to get preview window")?;

    let main_window = app
        .get_webview_window("main")
        .ok_or("Failed to get main window")?;

    // Apply vibrancy

    #[cfg(target_os = "macos")]
    {
        apply_vibrancy(
            &preview_window,
            NSVisualEffectMaterial::HudWindow,
            None,
            None,
        )
        .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS")
    }

    // Get main window position and size
    let main_pos = main_window
        .outer_position()
        .map_err(|e| format!("Failed to get main window position: {}", e))?;
    let main_size = main_window
        .outer_size()
        .map_err(|e| format!("Failed to get main window size: {}", e))?;

    // Calculate preview position (right of main window)
    // We use physical coordinates for set_position
    let gap = 10; // Gap between windows
    let preview_x = main_pos.x + main_size.width as i32 + gap;
    let preview_y = main_pos.y;

    preview_window
        .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: preview_x,
            y: preview_y,
        }))
        .map_err(|e| format!("Failed to set preview position: {}", e))?;

    // Make window non-focusable to prevent focus stealing
    preview_window
        .set_focusable(false)
        .map_err(|e| format!("Failed to set focusable: {}", e))?;

    // Show the window without focusing it
    // Note: set_focusable(false) should prevent focus on show, but we can also use show() safely now.
    preview_window
        .show()
        .map_err(|e| format!("Failed to show preview window: {}", e))?;

    println!("Showing preview for item: {}", id);

    // Emit event to preview window
    preview_window
        .emit("preview-item", id)
        .map_err(|e| format!("Failed to emit preview event: {}", e))?;

    println!("Emitted preview-item event");

    Ok(())
}

// Command to fetch preview content
#[tauri::command]
async fn get_preview_content(id: String) -> Result<serde_json::Value, String> {
    let url = format!(
        "http://localhost:3016/item/{}/preview?interactive=false",
        id
    );
    let client = reqwest::Client::new();

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch preview: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let json = response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    Ok(json)
}

// Command to open item in dashboard
#[tauri::command]
fn open_in_dashboard(app: tauri::AppHandle, id: String) -> Result<(), String> {
    use tauri::Manager;
    
    // Open URL in default browser
    let url = format!("http://localhost:3016/dashboard?item={}", id);
    app.shell().open(&url, None)
        .map_err(|e| format!("Failed to open URL: {}", e))?;

    // Hide preview window
    if let Some(window) = app.get_webview_window("preview") {
        window.hide()
            .map_err(|e| format!("Failed to hide preview window: {}", e))?;
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_macos_permissions::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .on_window_event(|window, event| {
            // Handle settings window close/destroy events
            if window.label() == "settings" {
                if let tauri::WindowEvent::Destroyed = event {
                    println!("Settings window destroyed, restoring dock state");
                    #[cfg(target_os = "macos")]
                    {
                        let _ = window
                            .app_handle()
                            .set_activation_policy(tauri::ActivationPolicy::Accessory);
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            unregister_main_shortcut,
            register_main_shortcut,
            open_settings,
            preview_item,
            get_preview_content,
            open_in_dashboard,
            sidecar::init_service,
            sidecar::stop_service,
            sidecar::get_service_status,
            sidecar::get_history,
            sidecar::get_mtime,
            sidecar::copy_item,
            sidecar::paste_item,
            sidecar::delete_item,
            sidecar::configure_data_dir,
            sidecar::db_get_count,
            sidecar::db_get_size,
            sidecar::db_export_all,
            sidecar::db_import_all,
            sidecar::db_delete_all,
            sidecar::get_sidecar_dir,
            sidecar::set_sidecar_dir,
            sidecar::get_app_data_dir,
            paste::simulate_system_paste,
            visibility::is_visible,
            visibility::hide_app,
            visibility::show_app,
            accessibility::check_permissions,
            accessibility::request_permissions,
        ])
        .setup(|app| {
            use tauri_plugin_global_shortcut::{Code, ShortcutState};

            let main_shortcut = Shortcut::new(Some(Modifiers::CONTROL), Code::KeyP);

            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let app_handle = app.handle().clone();
            let window = app_handle.get_webview_window("main").unwrap();

            // Shared state for clipboard items in tray
            let tray_items: TrayClipboardItems = Arc::new(Mutex::new(Vec::new()));

            // Create system tray menu with dynamic stats
            let show_item = MenuItemBuilder::with_id("show", "Show Clippy").build(app)?;
            let dashboard_item =
                MenuItemBuilder::with_id("dashboard", "Show dashboard").build(app)?;
            // Add Cmd+, accelerator to Settings menu item (only works when tray menu is open)
            let settings_item = MenuItemBuilder::with_id("settings", "Settings")
                .accelerator("CmdOrCtrl+,")
                .build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

            // Create initial stats item
            let stats_item =
                MenuItemBuilder::with_id("stats", "clippy v0.1.0 - Running").build(app)?;

            // Create separator
            let separator = PredefinedMenuItem::separator(app)?;

            // Create clipboard item menu entries (initially empty, will be populated)
            let clip_items: Vec<tauri::menu::MenuItem<tauri::Wry>> = (0..10)
                .map(|i| {
                    let key = if i == 9 {
                        "0".to_string()
                    } else {
                        (i + 1).to_string()
                    };
                    MenuItemBuilder::with_id(format!("clip_{}", i), format!("{}. (empty)", key))
                        .accelerator(format!("CmdOrCtrl+{}", key))
                        .enabled(false)
                        .build(app)
                        .unwrap()
                })
                .collect();

            // Build menu with all items
            let mut menu_builder = MenuBuilder::new(app)
                .item(&stats_item)
                .item(&show_item)
                .item(&dashboard_item)
                .item(&settings_item)
                .item(&separator);

            // Add clipboard items
            for clip_item in &clip_items {
                menu_builder = menu_builder.item(clip_item);
            }

            let menu = menu_builder
                .item(&PredefinedMenuItem::separator(app)?)
                .item(&quit_item)
                .build()?;

            // Spawn stats and clipboard items updater for Tray
            let stats_item_handle = stats_item.clone();
            let clip_items_handles: Vec<_> = clip_items.iter().cloned().collect();
            let tray_items_clone = tray_items.clone();

            tauri::async_runtime::spawn(async move {
                let client = reqwest::Client::new();
                let stats_url = "http://localhost:3016/stats";
                let items_url = "http://localhost:3016/items?count=10";

                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                    // Update stats
                    if let Ok(response) = client.get(stats_url).send().await {
                        if let Ok(json) = response.json::<serde_json::Value>().await {
                            let count = json["totalItems"].as_u64().unwrap_or(0);
                            let size = json["totalSize"].as_u64().unwrap_or(0);

                            let size_str = if size < 1024 {
                                format!("{}b", size)
                            } else if size < 1024 * 1024 {
                                format!("{:.0}kb", size as f64 / 1024.0)
                            } else {
                                format!("{:.1}mb", size as f64 / (1024.0 * 1024.0))
                            };

                            let text = format!("clippy v0.1.0 - {} items {}", count, size_str);
                            let _ = stats_item_handle.set_text(text);
                        }
                    }

                    // Update clipboard items in tray
                    if let Ok(response) = client.get(items_url).send().await {
                        if let Ok(items) = response.json::<Vec<serde_json::Value>>().await {
                            let mut tray_items_lock = tray_items_clone.lock().await;
                            tray_items_lock.clear();

                            for (i, item) in items.iter().take(10).enumerate() {
                                let id = item["hash"]
                                    .as_str()
                                    .or(item["id"].as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let summary = item["summary"].as_str().unwrap_or("").to_string();

                                // Truncate summary for menu display
                                let display_summary = if summary.len() > 40 {
                                    format!("{}...", &summary[..37])
                                } else {
                                    summary.clone()
                                };

                                // Update menu item text
                                let key = if i == 9 {
                                    "0".to_string()
                                } else {
                                    (i + 1).to_string()
                                };
                                let menu_text =
                                    format!("{}. {}", key, display_summary.replace('\n', " "));

                                if let Some(menu_item) = clip_items_handles.get(i) {
                                    let _ = menu_item.set_text(&menu_text);
                                    let _ = menu_item.set_enabled(true);
                                }

                                tray_items_lock.push((id, summary));
                            }

                            // Disable unused menu items
                            for i in items.len()..10 {
                                if let Some(menu_item) = clip_items_handles.get(i) {
                                    let key = if i == 9 {
                                        "0".to_string()
                                    } else {
                                        (i + 1).to_string()
                                    };
                                    let _ = menu_item.set_text(&format!("{}. (empty)", key));
                                    let _ = menu_item.set_enabled(false);
                                }
                            }
                        }
                    }
                }
            });

            // Store tray items in app state for menu event handler
            app.manage(tray_items.clone());

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(move |app, event| {
                    let event_id = event.id().as_ref();
                    match event_id {
                        "show" => {
                            if let Some(_window) = app.get_webview_window("main") {
                                let _ = visibility::show(app.clone());
                            }
                        }
                        "dashboard" => {
                            let _ = app.shell().open("http://localhost:3016/dashboard", None);
                        }
                        "settings" => {
                            if let Err(e) = open_settings_window(app.clone()) {
                                eprintln!("Failed to open settings: {}", e);
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        id if id.starts_with("clip_") => {
                            // Handle clipboard item selection
                            if let Ok(index) =
                                id.strip_prefix("clip_").unwrap_or("").parse::<usize>()
                            {
                                let app_clone = app.clone();
                                tauri::async_runtime::spawn(async move {
                                    let tray_items: tauri::State<'_, TrayClipboardItems> =
                                        app_clone.state();
                                    let items = tray_items.lock().await;
                                    if let Some((id, _)) = items.get(index) {
                                        // Paste the item
                                        if let Err(e) =
                                            sidecar::paste_item(app_clone.clone(), id.clone()).await
                                        {
                                            eprintln!("Failed to paste item from tray: {}", e);
                                        }
                                    }
                                });
                            }
                        }
                        _ => (),
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = visibility::hide(app);
                            } else {
                                let _ = visibility::show(app.clone());
                            }
                        }
                    }
                })
                .build(app)?;

            /* Shortcut - Only register Ctrl+P as global shortcut */
            /* Cmd+Comma is now handled by frontend and tray menu accelerator */
            app_handle.plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler({
                        move |app_handle, shortcut, event| {
                            if shortcut == &main_shortcut && event.state() == ShortcutState::Pressed
                            {
                                println!("Ctrl+P pressed - showing window");
                                if let Err(e) = visibility::show(app_handle.clone()) {
                                    eprintln!("Failed to show window: {}", e);
                                }
                            }
                        }
                    })
                    .build(),
            )?;
            app.global_shortcut().register(main_shortcut)?;
            // NOTE: We no longer register settings_shortcut globally
            // This allows other apps to receive Cmd+Comma for their preferences

            #[cfg(target_os = "macos")]
            {
                apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, None)
                    .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");
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
                                if let Err(e) =
                                    accessibility::show_permissions_alert(app_handle_clone.clone())
                                        .await
                                {
                                    eprintln!("Failed to show permissions alert: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to check accessibility permissions: {}", e);
                            // Show alert anyway to guide user
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

            // Configure sidecar and start watcher service
            let app_handle_clone = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                println!("Initializing clipboard watcher service...");
                if let Err(e) = sidecar::init_service(app_handle_clone.clone()).await {
                    eprintln!("Failed to initialize service: {}", e);
                } else {
                    println!("Clipboard watcher service initialized successfully");
                }
            });

            // Start API server sidecar (child process)
            let app_handle_clone = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                println!("Starting API sidecar...");
                let sidecar = app_handle_clone.shell().sidecar("get_clipboard");
                if let Ok(cmd) = sidecar {
                    // We spawn it and let it run.
                    // The process will persist as long as the main app runs (or until stopped).
                    let result = cmd.args(["api", "--port", "3016"]).spawn();
                    match result {
                        Ok((_rx, _child)) => println!("API sidecar started successfully"),
                        Err(e) => eprintln!("Failed to spawn API sidecar: {}", e),
                    }
                } else {
                    eprintln!("Failed to find sidecar");
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
