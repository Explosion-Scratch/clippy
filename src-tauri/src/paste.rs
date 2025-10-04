use tauri::AppHandle;

#[tauri::command]
pub fn simulate_system_paste(app: AppHandle) -> Result<(), String> {
    simulate_system_paste_internal(&app)
}

/// Internal function to simulate system paste without Tauri command wrapper
pub fn simulate_system_paste_internal(_app: &AppHandle) -> Result<(), String> {
    println!("Simulating system paste...");
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
            // Create an event source for keyboard events using HID system state
            let event_source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
                .ok_or("Failed to create CGEventSource")?;

            // Create Command key down event (0x37 is left Command key)
            let cmd_down_event = CGEvent::new_keyboard_event(
                Some(&event_source),
                0x37, // kVK_Command
                true, // key down
            ).ok_or("Failed to create Command down event")?;

            // Create V key down event (0x09 is V key)
            let v_down_event = CGEvent::new_keyboard_event(
                Some(&event_source),
                0x09, // kVK_ANSI_V
                true, // key down
            ).ok_or("Failed to create V down event")?;

            // Create V key up event
            let v_up_event = CGEvent::new_keyboard_event(
                Some(&event_source),
                0x09, // kVK_ANSI_V
                false, // key up
            ).ok_or("Failed to create V up event")?;

            // Create Command key up event
            let cmd_up_event = CGEvent::new_keyboard_event(
                Some(&event_source),
                0x37, // kVK_Command
                false, // key up
            ).ok_or("Failed to create Command up event")?;

            // Set Command modifier flag on all events
            let cmd_flags = CGEventFlags::MaskCommand;
            CGEvent::set_flags(Some(&cmd_down_event), cmd_flags);
            CGEvent::set_flags(Some(&v_down_event), cmd_flags);
            CGEvent::set_flags(Some(&v_up_event), cmd_flags);
            CGEvent::set_flags(Some(&cmd_up_event), cmd_flags);

            println!("Sending paste key events with proper sequence");

            // Post events to HID event tap with proper timing
            CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&cmd_down_event));
            std::thread::sleep(std::time::Duration::from_micros(15000));
            CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&v_down_event));
            std::thread::sleep(std::time::Duration::from_micros(15000));
            CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&v_up_event));
            std::thread::sleep(std::time::Duration::from_micros(15000));
            CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&cmd_up_event));
        }
    }
    Ok(())
}
