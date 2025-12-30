use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};
use tauri_plugin_store::StoreExt;

const STORE_PATH: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSettings {
    pub shortcut: String,
    pub first_run_complete: bool,
    pub welcome_shown: bool,
    pub cli_in_path: bool,
    pub accent_color: String,
}

impl AppSettings {
    pub fn default_shortcut() -> String {
        "Control+P".to_string()
    }

    pub fn default_accent_color() -> String {
        "#20b2aa".to_string()
    }
}

pub fn parse_shortcut(shortcut_str: &str) -> Result<Shortcut, String> {
    let parts: Vec<&str> = shortcut_str.split('+').collect();
    if parts.is_empty() {
        return Err("Empty shortcut string".to_string());
    }

    let mut modifiers = Modifiers::empty();
    let mut key_code: Option<Code> = None;

    for part in parts {
        match part {
            "Control" | "Ctrl" => modifiers |= Modifiers::CONTROL,
            "Alt" | "Option" => modifiers |= Modifiers::ALT,
            "Shift" => modifiers |= Modifiers::SHIFT,
            "Meta" | "Cmd" | "Command" | "Super" => modifiers |= Modifiers::META,
            key => {
                key_code = Some(string_to_code(key)?);
            }
        }
    }

    let code = key_code.ok_or("No key specified in shortcut")?;
    let mods = if modifiers.is_empty() {
        None
    } else {
        Some(modifiers)
    };

    Ok(Shortcut::new(mods, code))
}

fn string_to_code(key: &str) -> Result<Code, String> {
    let code = match key.to_uppercase().as_str() {
        "A" => Code::KeyA,
        "B" => Code::KeyB,
        "C" => Code::KeyC,
        "D" => Code::KeyD,
        "E" => Code::KeyE,
        "F" => Code::KeyF,
        "G" => Code::KeyG,
        "H" => Code::KeyH,
        "I" => Code::KeyI,
        "J" => Code::KeyJ,
        "K" => Code::KeyK,
        "L" => Code::KeyL,
        "M" => Code::KeyM,
        "N" => Code::KeyN,
        "O" => Code::KeyO,
        "P" => Code::KeyP,
        "Q" => Code::KeyQ,
        "R" => Code::KeyR,
        "S" => Code::KeyS,
        "T" => Code::KeyT,
        "U" => Code::KeyU,
        "V" => Code::KeyV,
        "W" => Code::KeyW,
        "X" => Code::KeyX,
        "Y" => Code::KeyY,
        "Z" => Code::KeyZ,
        "0" | "DIGIT0" => Code::Digit0,
        "1" | "DIGIT1" => Code::Digit1,
        "2" | "DIGIT2" => Code::Digit2,
        "3" | "DIGIT3" => Code::Digit3,
        "4" | "DIGIT4" => Code::Digit4,
        "5" | "DIGIT5" => Code::Digit5,
        "6" | "DIGIT6" => Code::Digit6,
        "7" | "DIGIT7" => Code::Digit7,
        "8" | "DIGIT8" => Code::Digit8,
        "9" | "DIGIT9" => Code::Digit9,
        "F1" => Code::F1,
        "F2" => Code::F2,
        "F3" => Code::F3,
        "F4" => Code::F4,
        "F5" => Code::F5,
        "F6" => Code::F6,
        "F7" => Code::F7,
        "F8" => Code::F8,
        "F9" => Code::F9,
        "F10" => Code::F10,
        "F11" => Code::F11,
        "F12" => Code::F12,
        "ESCAPE" | "ESC" => Code::Escape,
        "BACKSPACE" => Code::Backspace,
        "TAB" => Code::Tab,
        "ENTER" | "RETURN" => Code::Enter,
        "SPACE" => Code::Space,
        "ARROWUP" => Code::ArrowUp,
        "ARROWDOWN" => Code::ArrowDown,
        "ARROWLEFT" => Code::ArrowLeft,
        "ARROWRIGHT" => Code::ArrowRight,
        "DELETE" => Code::Delete,
        "HOME" => Code::Home,
        "END" => Code::End,
        "PAGEUP" => Code::PageUp,
        "PAGEDOWN" => Code::PageDown,
        _ => return Err(format!("Unknown key: {}", key)),
    };
    Ok(code)
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<AppSettings, String> {
    let store = app.store(STORE_PATH).map_err(|e| e.to_string())?;

    let settings = AppSettings {
        shortcut: store
            .get("shortcut")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(AppSettings::default_shortcut),
        first_run_complete: store
            .get("first_run_complete")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        welcome_shown: store
            .get("welcome_shown")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        cli_in_path: store
            .get("cli_in_path")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        accent_color: store
            .get("accent_color")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(AppSettings::default_accent_color),
    };

    Ok(settings)
}

#[tauri::command]
pub fn set_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let store = app.store(STORE_PATH).map_err(|e| e.to_string())?;

    store.set("shortcut", serde_json::json!(settings.shortcut));
    store.set(
        "first_run_complete",
        serde_json::json!(settings.first_run_complete),
    );
    store.set(
        "welcome_shown",
        serde_json::json!(settings.welcome_shown),
    );
    store.set("cli_in_path", serde_json::json!(settings.cli_in_path));
    store.set("accent_color", serde_json::json!(settings.accent_color));
    store.save().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn check_first_run(app: AppHandle) -> Result<bool, String> {
    let store = app.store(STORE_PATH).map_err(|e| e.to_string())?;

    let is_first_run = !store
        .get("first_run_complete")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok(is_first_run)
}

#[tauri::command]
pub fn check_welcome_shown(app: AppHandle) -> Result<bool, String> {
    let store = app.store(STORE_PATH).map_err(|e| e.to_string())?;

    let welcome_shown = store
        .get("welcome_shown")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok(welcome_shown)
}

#[tauri::command]
pub fn get_configured_shortcut(app: AppHandle) -> Result<String, String> {
    let store = app.store(STORE_PATH).map_err(|e| e.to_string())?;

    let shortcut = store
        .get("shortcut")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(AppSettings::default_shortcut);

    Ok(shortcut)
}

#[tauri::command]
pub async fn add_cli_to_path(app: AppHandle) -> Result<String, String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to get current executable path: {}", e))?;
    
    let macos_dir = current_exe
        .parent()
        .ok_or("Failed to get MacOS directory")?;
    
    let sidecar_path = macos_dir.join("get_clipboard");

    if !sidecar_path.exists() {
        let arch = if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else {
            "x86_64"
        };
        let sidecar_name = format!("get_clipboard-{}-apple-darwin", arch);
        let alt_path = macos_dir.join(&sidecar_name);
        
        if alt_path.exists() {
            return create_symlink(&app, &alt_path);
        }
        
        return Err(format!(
            "Sidecar binary not found at: {} or {}",
            sidecar_path.display(),
            alt_path.display()
        ));
    }
    
    create_symlink(&app, &sidecar_path)
}

fn create_symlink(app: &AppHandle, sidecar_path: &std::path::Path) -> Result<String, String> {

    let home = std::env::var("HOME").map_err(|_| "HOME not set")?;
    let bin_dir = PathBuf::from(&home).join(".local").join("bin");
    let target_path = bin_dir.join("get_clipboard");

    std::fs::create_dir_all(&bin_dir)
        .map_err(|e| format!("Failed to create ~/.local/bin: {}", e))?;

    if target_path.exists() || target_path.is_symlink() {
        std::fs::remove_file(&target_path).ok();
    }

    #[cfg(unix)]
    std::os::unix::fs::symlink(&sidecar_path, &target_path)
        .map_err(|e| format!("Failed to create symlink: {}", e))?;

    let store = app.store(STORE_PATH).map_err(|e| e.to_string())?;
    store.set("cli_in_path", serde_json::json!(true));
    store.save().map_err(|e| e.to_string())?;

    let shell_hint = format!(
        "Created symlink at {}\n\nAdd this to your shell config (~/.zshrc):\nexport PATH=\"$HOME/.local/bin:$PATH\"",
        target_path.display()
    );

    Ok(shell_hint)
}
