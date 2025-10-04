use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn simulate_system_paste(app: AppHandle) -> Result<(), String> {
    println!("Pasting...");

    let window = app.get_webview_window("main").unwrap();

    #[cfg(target_os = "macos")]
    {
        // Use native macOS APIs for much faster keyboard simulation
        // This method is significantly faster than spawning osascript processes
        use objc::runtime::Object;
        use objc::{class, msg_send, sel, sel_impl};

        unsafe {
            // Get the shared application instance
            let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];

            // Post the key events directly to the application event queue
            // This is faster and more reliable than targeting specific windows
            let cmd_modifier: u64 = 0x100000; // NSEventModifierFlagCommand
            let key_v: u16 = 9; // Key code for 'V'

            // Create key down event (Command + V)
            let key_down: *mut Object = msg_send![
                class!(NSEvent),
                keyEventWithType: 0x0a  // NSEventTypeKeyDown
                location: (0.0, 0.0)
                modifierFlags: cmd_modifier
                timestamp: 0.0
                windowNumber: 0
                context: std::ptr::null::<Object>()
                characters: std::ptr::null::<Object>()
                charactersIgnoringModifiers: std::ptr::null::<Object>()
                isARepeat: false
                keyCode: key_v
            ];

            // Create key up event
            let key_up: *mut Object = msg_send![
                class!(NSEvent),
                keyEventWithType: 0x0b  // NSEventTypeKeyUp
                location: (0.0, 0.0)
                modifierFlags: cmd_modifier
                timestamp: 0.0
                windowNumber: 0
                context: std::ptr::null::<Object>()
                characters: std::ptr::null::<Object>()
                charactersIgnoringModifiers: std::ptr::null::<Object>()
                isARepeat: false
                keyCode: key_v
            ];

            // Post events to the global event queue for immediate processing
            let _: () = msg_send![app, postEvent: key_down atStart: false];
            let _: () = msg_send![app, postEvent: key_up atStart: false];
        }
    }

    let _ = window.hide();

    Ok(())
}

#[tauri::command]
pub fn simulate_system_paste_fallback(app: AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").unwrap();

    // Simple fallback that uses the original osascript approach
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Events\" to keystroke \"v\" using command down")
            .output()
            .map_err(|e| format!("Failed to simulate paste: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new("powershell")
            .arg("-Command")
            .arg("Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.SendKeys]::SendWait('^{V}')")
            .output()
            .map_err(|e| format!("Failed to simulate paste: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        Command::new("xdotool")
            .args(&["key", "ctrl+v"])
            .output()
            .map_err(|e| format!("Failed to simulate paste: {}", e))?;
    }

    // Hide window after paste
    let _ = window.hide();

    Ok(())
}
