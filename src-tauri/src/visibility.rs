use tauri::{AppHandle, Manager};

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

/// Hide the app and main window
pub fn hide(app: &AppHandle) -> Result<(), String> {
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
    Ok(())
}

/// Show the app and main window
pub fn show(app: AppHandle) -> Result<(), String> {
    println!("Showing window and app");
    
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
    Ok(())
}

/// Tauri command to hide the app and window
#[tauri::command]
pub fn hide_app(app: AppHandle) -> Result<(), String> {
    hide(&app)
}

/// Tauri command to show the app and window
#[tauri::command]
pub fn show_app(app: AppHandle) -> Result<(), String> {
    show(app)
}