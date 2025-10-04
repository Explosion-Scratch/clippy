use tauri::{AppHandle, Runtime, Window};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_global_shortcut::Shortcut;
use tauri::Manager;
use tauri_plugin_global_shortcut::ShortcutState;

pub fn set_window_shortcut<R: Runtime>(app_handle: &AppHandle<R>, shortcut_str: String) -> Result<(), String> {
    // Parse shortcut string to Shortcut object
    let shortcut = shortcut_str
        .parse::<Shortcut>()
        .map_err(|_| format!("Invalid shortcut format: {}", shortcut_str))?;

    // Unregister any existing shortcut if needed (optional, not handled here)

    // Get main window handle
    let window = app_handle
        .get_webview_window("main")
        .ok_or("Main window not found".to_string())?;

    // Register the new shortcut with a handler to toggle window visibility
    app_handle
        .global_shortcut()
        .register(shortcut)
        .map_err(|e| format!("Failed to register shortcut: {}", e))?;

    let _ = app_handle.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, event| {
        if let ShortcutState::Pressed = event.state() {
            if window.is_visible().unwrap_or(false) {
                let _ = window.hide();
            } else {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    });

    Ok(())
}
