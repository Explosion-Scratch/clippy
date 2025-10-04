use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn simulate_system_paste(app: AppHandle) -> Result<(), String> {
    println!("Pasting...");

    let window = app.get_webview_window("main").unwrap();
    let _ = window.hide();

    // On macOS, also hide the app to properly return focus to the previous application
    #[cfg(target_os = "macos")]
    {
        let _ = app.hide();
    }

    #[cfg(target_os = "macos")]
    {
        // Use CGEvent for the most reliable and fastest keyboard simulation
        // CGEvent provides direct access to Core Graphics event system
        use objc2_core_graphics::CGEvent;
        use objc2_core_graphics::CGEventSource;
        use objc2_core_graphics::CGEventSourceStateID;
        use objc2_core_graphics::CGEventFlags;
        use objc2_core_graphics::CGEventTapLocation;

        unsafe {
            // Create an event source for keyboard events
            let event_source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
                .ok_or("Failed to create CGEventSource")?;

            // Create Command+V key down event
            let key_down_event = CGEvent::new_keyboard_event(
                Some(&event_source),
                9, // kVK_ANSI_V (key code for 'V')
                true, // key down
            ).ok_or("Failed to create key down event")?;

            // Set Command modifier flag
            let cmd_flags = CGEventFlags::MaskCommand;
            CGEvent::set_flags(Some(&key_down_event), cmd_flags);

            // Create Command+V key up event
            let key_up_event = CGEvent::new_keyboard_event(
                Some(&event_source),
                9, // kVK_ANSI_V (key code for 'V')
                false, // key up
            ).ok_or("Failed to create key up event")?;

            // Set Command modifier flag for key up as well
            CGEvent::set_flags(Some(&key_up_event), cmd_flags);
            println!("Sending key down");
            // Post the events to the system event queue
            // This ensures immediate processing by the system
            CGEvent::post(CGEventTapLocation::SessionEventTap, Some(&key_down_event));
            CGEvent::post(CGEventTapLocation::SessionEventTap, Some(&key_up_event));
        }
    }

    Ok(())
}


