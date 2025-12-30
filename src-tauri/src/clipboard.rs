use clipboard_rs::{Clipboard, ClipboardContext};
use tauri::AppHandle;

#[tauri::command]
pub fn write_to_clipboard(_app_handle: AppHandle, text: String) -> Result<(), String> {
    println!("=== WRITING PLAIN TEXT TO CLIPBOARD ===");

    let ctx = ClipboardContext::new()
        .map_err(|e| format!("Failed to create clipboard context: {}", e))?;

    if let Err(e) = ctx.set_text(text) {
        eprintln!("Failed to set text: {}", e);
        return Err(format!("Failed to write to clipboard: {}", e));
    }

    Ok(())
}
