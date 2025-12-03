use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_macos_permissions;
/// Check if the app has accessibility permissions on macOS
#[cfg(target_os = "macos")]
pub async fn check_accessibility_permissions() -> bool {
    tauri_plugin_macos_permissions::check_accessibility_permission().await
}

/// Request accessibility permissions on macOS
#[cfg(target_os = "macos")]
pub async fn request_accessibility_permissions() -> Result<(), String> {
    tauri_plugin_macos_permissions::request_accessibility_permission().await;
    Ok(())
}

/// Ensure accessibility permissions are available, requesting if needed
pub async fn ensure_accessibility_permissions() -> Result<bool, String> {
    if check_accessibility_permissions().await {
        println!("App already has accessibility permissions");
        Ok(true)
    } else {
        println!("Requesting accessibility permissions...");
        match request_accessibility_permissions().await {
            Ok(()) => {
                println!("Accessibility permissions request sent successfully");
                Ok(false) // User still needs to grant permissions
            }
            Err(e) => {
                eprintln!("Failed to request accessibility permissions: {}", e);
                Err(e)
            }
        }
    }
}

/// Tauri command to check accessibility permissions
#[tauri::command]
pub async fn check_permissions() -> bool {
    check_accessibility_permissions().await
}

/// Show permissions alert to the user using native dialog
#[cfg(target_os = "macos")]
pub async fn show_permissions_alert(app_handle: AppHandle) -> Result<(), String> {
    app_handle
        .dialog()
        .message("Accessibility permissions are required for this app to function properly.\n\nPlease grant accessibility permissions in System Settings > Privacy & Security > Accessibility.")
        .kind(MessageDialogKind::Info)
        .title("Accessibility Permissions Required")
        .blocking_show();
    Ok(())
}

/// Tauri command to request accessibility permissions
#[tauri::command]
pub async fn request_permissions() -> Result<String, String> {
    match request_accessibility_permissions().await {
        Ok(()) => Ok("System Settings opened to Accessibility pane".to_string()),
        Err(e) => Err(format!("Failed to open System Settings: {}", e)),
    }
}
