use tauri::Manager;
use tauri::WebviewUrl;
use tauri::webview::WebviewWindowBuilder;
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};

use crate::visibility;

pub fn open_settings_window(
    app_handle: tauri::AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    {
        app_handle.set_activation_policy(tauri::ActivationPolicy::Regular)?;
    }

    if app_handle.get_webview_window("main").is_some() {
        visibility::hide(&app_handle).ok();
    }

    let settings_window = match app_handle.get_webview_window("settings") {
        Some(window) => window,
        None => {
            WebviewWindowBuilder::new(
                &app_handle,
                "settings",
                WebviewUrl::App("/settings".into()),
            )
            .title("Clippy Settings")
            .inner_size(400.0, 500.0)
            .transparent(true)
            .resizable(false)
            .minimizable(true)
            .maximizable(false)
            .visible(false)
            .build()?
        }
    };

    settings_window.show()?;
    settings_window.set_focus()?;

    #[cfg(target_os = "macos")]
    {
        let app_clone = app_handle.clone();
        let _ = app_handle.run_on_main_thread(move || {
            if let Some(settings_window) = app_clone.get_webview_window("settings") {
                let _ = apply_vibrancy(
                    &settings_window,
                    NSVisualEffectMaterial::HudWindow,
                    None,
                    None,
                );
            }
        });
    }

    Ok(())
}

pub fn open_welcome_window(
    app_handle: tauri::AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    {
        app_handle.set_activation_policy(tauri::ActivationPolicy::Regular)?;
    }

    if app_handle.get_webview_window("main").is_some() {
        visibility::hide(&app_handle).ok();
    }

    let welcome_window = match app_handle.get_webview_window("welcome") {
        Some(window) => window,
        None => {
            WebviewWindowBuilder::new(
                &app_handle,
                "welcome",
                WebviewUrl::App("/welcome".into()),
            )
            .title("Welcome to Clippy")
            .inner_size(450.0, 580.0)
            .transparent(true)
            .resizable(false)
            .minimizable(false)
            .maximizable(false)
            .hidden_title(true)
            .title_bar_style(tauri::TitleBarStyle::Overlay)
            .visible(false)
            .build()?
        }
    };

    welcome_window.show()?;
    welcome_window.set_focus()?;

    #[cfg(target_os = "macos")]
    {
        let app_clone = app_handle.clone();
        let _ = app_handle.run_on_main_thread(move || {
            if let Some(welcome_window) = app_clone.get_webview_window("welcome") {
                let _ = apply_vibrancy(
                    &welcome_window,
                    NSVisualEffectMaterial::HudWindow,
                    None,
                    None,
                );
            }
        });
    }

    Ok(())
}

pub fn handle_window_destroyed(window: &tauri::Window) {
    if window.label() == "settings" || window.label() == "welcome" {
        println!("{} window destroyed, restoring dock state", window.label());
        #[cfg(target_os = "macos")]
        {
            let _ = window
                .app_handle()
                .set_activation_policy(tauri::ActivationPolicy::Accessory);
        }
    }
}

pub fn handle_main_window_focus_lost(window: &tauri::Window) {
    if let Some(preview) = window.app_handle().get_webview_window("preview") {
        if !preview.is_focused().unwrap_or(false) {
            let _ = preview.hide();
        }
    }
}

pub fn handle_main_window_destroyed(window: &tauri::Window) {
    if let Some(preview) = window.app_handle().get_webview_window("preview") {
        let _ = preview.hide();
    }
}
