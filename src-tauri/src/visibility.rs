use tauri::{AppHandle, Emitter, Manager};

/// Check if the app or window is visible
#[tauri::command]
pub fn is_visible(app: AppHandle) -> Result<bool, String> {
    let window = app.get_webview_window("main")
        .ok_or("Failed to get main window")?;
    
    let is_visible = window.is_visible()
        .map_err(|e| format!("Failed to check window visibility: {}", e))?;
    
    println!("Window visibility: {}", is_visible);
    Ok(is_visible)
}

/// Hide the main window only (not the entire app)
pub fn hide(app: &AppHandle) -> Result<(), String> {
    println!("Hiding main window");
    
    let window = app.get_webview_window("main")
        .ok_or("Failed to get main window")?;
    
    // Hide the main window only
    window.hide()
        .map_err(|e| format!("Failed to hide window: {}", e))?;
    
    println!("Main window hidden successfully");
    
    // Emit event to update tray menu
    app.emit("window-visibility-changed", ()).map_err(|e| format!("Failed to emit visibility event: {}", e))?;
    
    Ok(())
}

/// Hide the entire app (all windows) - used when dismissing the clipboard manager
pub fn hide_all(app: &AppHandle) -> Result<(), String> {
    println!("Hiding window and app");
    
    let window = app.get_webview_window("main")
        .ok_or("Failed to get main window")?;
    
    // Hide the window first
    window.hide()
        .map_err(|e| format!("Failed to hide window: {}", e))?;
    
    // On macOS, also hide the app to properly return focus to the previous application
    #[cfg(target_os = "macos")]
    {
        app.hide()
            .map_err(|e| format!("Failed to hide app: {}", e))?;
    }
    
    println!("Window and app hidden successfully");
    
    // Emit event to update tray menu
    app.emit("window-visibility-changed", ()).map_err(|e| format!("Failed to emit visibility event: {}", e))?;
    
    Ok(())
}

/// Show the app and main window
pub fn show(app: AppHandle) -> Result<(), String> {
    println!("Showing window and app");
    
    // First, close any settings window that might be open
    // Only one of clipboard manager or settings can be open at a time
    if let Some(settings_window) = app.get_webview_window("settings") {
        println!("Closing settings window before showing main window");
        let _ = settings_window.close();
        
        // Restore accessory mode since settings is closing
        #[cfg(target_os = "macos")]
        {
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        }
    }
    
    let window = app.get_webview_window("main")
        .ok_or("Failed to get main window")?;
    
    // Show the window
    window.show()
        .map_err(|e| format!("Failed to show window: {}", e))?;
    
    // Set focus to the window
    window.set_focus()
        .map_err(|e| format!("Failed to set window focus: {}", e))?;
    
    // On macOS, also show the app
    #[cfg(target_os = "macos")]
    {
        app.show()
            .map_err(|e| format!("Failed to show app: {}", e))?;
    }
    
    println!("Window and app shown successfully");
    
    // Emit event to update tray menu
    app.emit("window-visibility-changed", ()).map_err(|e| format!("Failed to emit visibility event: {}", e))?;
    
    Ok(())
}

/// Tauri command to hide the app and window (used by ESC key in clipboard manager)
#[tauri::command]
pub fn hide_app(app: AppHandle) -> Result<(), String> {
    hide_all(&app)
}

/// Tauri command to show the app and window
#[tauri::command]
pub fn show_app(app: AppHandle) -> Result<(), String> {
    show(app)
}