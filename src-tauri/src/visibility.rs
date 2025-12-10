use tauri::{AppHandle, Emitter, Manager};

/// Check if the app or window is visible
#[tauri::command]
pub fn is_visible(app: AppHandle) -> Result<bool, String> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(false);
    };

    let is_visible = window
        .is_visible()
        .map_err(|e| format!("Failed to check window visibility: {}", e))?;

    println!("Window visibility: {}", is_visible);
    Ok(is_visible)
}

/// Hide the main window and preview window
pub fn hide(app: &AppHandle) -> Result<(), String> {
    println!("Hiding main window");

    if let Some(window) = app.get_webview_window("main") {
        window
            .hide()
            .map_err(|e| format!("Failed to hide window: {}", e))?;
    }

    if let Some(preview) = app.get_webview_window("preview") {
        let _ = preview.hide();
    }

    println!("Main window hidden successfully");

    app.emit("window-visibility-changed", ())
        .map_err(|e| format!("Failed to emit visibility event: {}", e))?;

    Ok(())
}

/// Hide the entire app (all windows) - used when dismissing the clipboard manager
pub fn hide_all(app: &AppHandle) -> Result<(), String> {
    println!("Hiding window and app");

    if let Some(window) = app.get_webview_window("main") {
        window
            .hide()
            .map_err(|e| format!("Failed to hide window: {}", e))?;
    }

    if let Some(preview) = app.get_webview_window("preview") {
        let _ = preview.hide();
    }

    #[cfg(target_os = "macos")]
    {
        app.hide()
            .map_err(|e| format!("Failed to hide app: {}", e))?;
    }

    println!("Window and app hidden successfully");

    app.emit("window-visibility-changed", ())
        .map_err(|e| format!("Failed to emit visibility event: {}", e))?;

    Ok(())
}

/// Show the app and main window
pub fn show(app: AppHandle) -> Result<(), String> {
    use tauri::WebviewUrl;
    use tauri::webview::WebviewWindowBuilder;

    println!("Showing window and app");

    if let Some(settings_window) = app.get_webview_window("settings") {
        println!("Closing settings window before showing main window");
        let _ = settings_window.close();

        #[cfg(target_os = "macos")]
        {
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        }
    }

    let window = match app.get_webview_window("main") {
        Some(w) => w,
        None => {
            println!("Main window not found, recreating...");
            WebviewWindowBuilder::new(
                &app,
                "main",
                WebviewUrl::App("/".into()),
            )
            .title("clippy")
            .inner_size(400.0, 600.0)
            .min_inner_size(300.0, 200.0)
            .transparent(true)
            .resizable(true)
            .minimizable(false)
            .maximizable(false)
            .always_on_top(true)
            .visible_on_all_workspaces(true)
            .skip_taskbar(true)
            .hidden_title(true)
            .title_bar_style(tauri::TitleBarStyle::Overlay)
            .visible(false)
            .build()
            .map_err(|e| format!("Failed to recreate main window: {}", e))?
        }
    };

    window
        .show()
        .map_err(|e| format!("Failed to show window: {}", e))?;

    window
        .set_focus()
        .map_err(|e| format!("Failed to set window focus: {}", e))?;

    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
        let _ = apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, None);
        
        app.show()
            .map_err(|e| format!("Failed to show app: {}", e))?;
    }

    println!("Window and app shown successfully");

    app.emit("window-visibility-changed", ())
        .map_err(|e| format!("Failed to emit visibility event: {}", e))?;

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
