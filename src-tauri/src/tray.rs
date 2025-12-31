use std::sync::Arc;
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex;

use crate::api;
use crate::sidecar;
use crate::visibility;

pub type TrayClipboardItems = Arc<Mutex<Vec<(String, String)>>>;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn setup_tray(
    app: &tauri::App,
    tray_items: TrayClipboardItems,
    open_settings_fn: fn(tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let show_item = MenuItemBuilder::with_id("show", "Show Clippy")
        .accelerator("CmdOrCtrl+Return")
        .build(app)?;
    let dashboard_item = MenuItemBuilder::with_id("dashboard", "Show dashboard")
        .accelerator("CmdOrCtrl+Shift+Return")
        .build(app)?;
    let settings_item = MenuItemBuilder::with_id("settings", "Settings")
        .accelerator("CmdOrCtrl+,")
        .build(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "Quit")
        .accelerator("CmdOrCtrl+Q")
        .build(app)?;

    let stats_item =
        MenuItemBuilder::with_id("stats", format!("clippy v{VERSION}"))
            .enabled(false)
            .build(app)?;

    let mut clip_items: Vec<tauri::menu::MenuItem<tauri::Wry>> = Vec::with_capacity(10);
    for i in 0..10 {
        let key = if i == 9 {
            "0".to_string()
        } else {
            (i + 1).to_string()
        };
        let item = MenuItemBuilder::with_id(format!("clip_{}", i), "")
            .accelerator(format!("CmdOrCtrl+{}", key))
            .enabled(false)
            .build(app)?;
        clip_items.push(item);
    }

    let mut menu_builder = MenuBuilder::new(app);

    for clip_item in &clip_items {
        menu_builder = menu_builder.item(clip_item);
    }

    let menu = menu_builder
        .item(&PredefinedMenuItem::separator(app)?)
        .item(&stats_item)
        .item(&show_item)
        .item(&dashboard_item)
        .item(&settings_item)
        .item(&PredefinedMenuItem::separator(app)?)
        .item(&quit_item)
        .build()?;

    start_tray_stats_updater(
        stats_item.clone(),
        clip_items.iter().cloned().collect(),
        tray_items.clone(),
    );

    app.manage(tray_items.clone());

    let default_icon = app
        .default_window_icon()
        .ok_or("No default window icon available")?
        .clone();

    TrayIconBuilder::new()
        .icon(default_icon)
        .menu(&menu)
        .on_menu_event(move |app, event| {
            handle_tray_menu_event(app, event, tray_items.clone(), open_settings_fn);
        })
        .on_tray_icon_event(|tray, event| {
            handle_tray_icon_event(tray, event);
        })
        .build(app)?;

    Ok(())
}

fn start_tray_stats_updater(
    stats_item: tauri::menu::MenuItem<tauri::Wry>,
    clip_items_handles: Vec<tauri::menu::MenuItem<tauri::Wry>>,
    tray_items: TrayClipboardItems,
) {
    tauri::async_runtime::spawn(async move {
        let client = reqwest::Client::new();
        let stats_url = api::stats_url();
        let items_url = api::items_url(10);

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            match client.get(&stats_url).send().await {
                Ok(response) => {
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

                        let text = format!("clippy v{VERSION} · {} items · {}", count, size_str);
                        let _ = stats_item.set_text(text);
                    }
                }
                Err(_) => {
                    let _ = stats_item.set_text(format!("clippy v{VERSION} · ⚠ API not connected"));
                }
            }

            match client.get(&items_url).send().await {
                Ok(response) => {
                    if let Ok(items) = response.json::<Vec<serde_json::Value>>().await {
                        let mut tray_items_lock = tray_items.lock().await;
                        tray_items_lock.clear();

                        let item_count = items.len().min(10);

                        for (i, item) in items.iter().take(10).enumerate() {
                            let id = item["hash"]
                                .as_str()
                                .or(item["id"].as_str())
                                .unwrap_or("")
                                .to_string();
                            let summary = item["summary"].as_str().unwrap_or("").to_string();

                            let display_summary = {
                                let char_count = summary.chars().count();
                                if char_count > 40 {
                                    let truncated: String = summary.chars().take(37).collect();
                                    format!("{}...", truncated)
                                } else {
                                    summary.clone()
                                }
                            };

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

                        for i in item_count..10 {
                            if let Some(menu_item) = clip_items_handles.get(i) {
                                let _ = menu_item.set_text("");
                                let _ = menu_item.set_enabled(false);
                            }
                        }
                    }
                }
                Err(_) => {
                    for menu_item in &clip_items_handles {
                        let _ = menu_item.set_text("");
                        let _ = menu_item.set_enabled(false);
                    }
                }
            }
        }
    });
}

fn handle_tray_menu_event(
    app: &tauri::AppHandle,
    event: tauri::menu::MenuEvent,
    tray_items: TrayClipboardItems,
    open_settings_fn: fn(tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>>,
) {
    let event_id = event.id().as_ref();
    match event_id {
        "show" => {
            if app.get_webview_window("main").is_some() {
                let _ = visibility::show(app.clone());
            }
        }
        "dashboard" => {
            let _ = app.opener().open_url(&api::dashboard_url(), None::<&str>);
        }
        "settings" => {
            if let Err(e) = open_settings_fn(app.clone()) {
                eprintln!("Failed to open settings: {}", e);
            }
        }
        "quit" => {
            app.exit(0);
        }
        id if id.starts_with("clip_") => {
            if let Ok(index) = id.strip_prefix("clip_").unwrap_or("").parse::<usize>() {
                let app_clone = app.clone();
                tauri::async_runtime::spawn(async move {
                    let tray_items: tauri::State<'_, TrayClipboardItems> = app_clone.state();
                    let items = tray_items.lock().await;
                    if let Some((id, _)) = items.get(index) {
                        if let Err(e) = sidecar::paste_item(app_clone.clone(), id.clone()).await {
                            eprintln!("Failed to paste item from tray: {}", e);
                        }
                    }
                });
            }
        }
        _ => (),
    }
}

fn handle_tray_icon_event(tray: &tauri::tray::TrayIcon, event: TrayIconEvent) {
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
}
