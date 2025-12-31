use tauri::Emitter;
use tauri::Manager;
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};

use crate::api;

#[tauri::command]
pub fn preview_item(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let preview_window = app
        .get_webview_window("preview")
        .ok_or("Failed to get preview window")?;

    let main_window = app
        .get_webview_window("main")
        .ok_or("Failed to get main window")?;

    let is_visible = main_window
        .is_visible()
        .map_err(|e| format!("Failed to check main window visibility: {}", e))?;
    println!("is_visible: {}", is_visible);
    if !is_visible {
        let _ = preview_window.hide();
        return Ok(());
    }

    let main_pos = main_window
        .outer_position()
        .map_err(|e| format!("Failed to get main window position: {}", e))?;
    let main_size = main_window
        .outer_size()
        .map_err(|e: tauri::Error| format!("Failed to get main window size: {}", e))?;

    println!("main_size.width: {}", main_size.width);
    println!("Previewing item: {}", id);

    let gap = 10;
    let preview_x = main_pos.x + main_size.width as i32 + gap;
    let preview_y = main_pos.y;

    preview_window
        .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: preview_x,
            y: preview_y,
        }))
        .map_err(|e| format!("Failed to set preview position: {}", e))?;

    preview_window
        .set_focusable(false)
        .map_err(|e| format!("Failed to set focusable: {}", e))?;

    preview_window
        .show()
        .map_err(|e| format!("Failed to show preview window: {}", e))?;

    #[cfg(target_os = "macos")]
    {
        let app_clone = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(preview_window) = app_clone.get_webview_window("preview") {
                let _ = apply_vibrancy(
                    &preview_window,
                    NSVisualEffectMaterial::HudWindow,
                    None,
                    None,
                );
            }
        });
    }

    println!("Showing preview for item: {}", id);

    preview_window
        .emit("preview-item", id)
        .map_err(|e| format!("Failed to emit preview event: {}", e))?;

    println!("Emitted preview-item event");

    Ok(())
}

#[tauri::command]
pub fn hide_preview(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(preview_window) = app.get_webview_window("preview") {
        preview_window
            .hide()
            .map_err(|e| format!("Failed to hide preview window: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn focus_preview(app: tauri::AppHandle) -> Result<(), String> {
    println!("[focus_preview] Called");
    if let Some(preview_window) = app.get_webview_window("preview") {
        println!("[focus_preview] Got preview window, setting focusable");
        preview_window
            .set_focusable(true)
            .map_err(|e| format!("Failed to set focusable: {}", e))?;
        println!("[focus_preview] Focusable set, now focusing");
        preview_window
            .set_focus()
            .map_err(|e| format!("Failed to focus preview window: {}", e))?;
        println!("[focus_preview] Focus set successfully");
    } else {
        println!("[focus_preview] No preview window found");
    }
    Ok(())
}

#[tauri::command]
pub fn is_preview_visible(app: tauri::AppHandle) -> Result<bool, String> {
    if let Some(preview_window) = app.get_webview_window("preview") {
        preview_window
            .is_visible()
            .map_err(|e| format!("Failed to check preview visibility: {}", e))
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub async fn get_preview_content(id: String) -> Result<serde_json::Value, String> {
    let url = api::item_preview_url(&id, false);
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

#[tauri::command]
pub async fn get_item_data(id: String) -> Result<serde_json::Value, String> {
    let url = api::item_data_url(&id);
    let client = reqwest::Client::new();

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch item data: {}", e))?;

    println!("get_item_data response status: {}", response.status());

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let json = response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    Ok(json)
}

#[tauri::command]
pub fn open_in_dashboard(app: tauri::AppHandle, id: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;

    let url = api::dashboard_item_url(&id);
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| format!("Failed to open URL: {}", e))?;

    if let Some(window) = app.get_webview_window("preview") {
        window
            .hide()
            .map_err(|e| format!("Failed to hide preview window: {}", e))?;
    }

    Ok(())
}
