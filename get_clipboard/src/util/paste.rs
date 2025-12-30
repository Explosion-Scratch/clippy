use anyhow::{anyhow, Result};
use std::thread;
use std::time::Duration;

#[cfg(target_os = "macos")]
pub fn simulate_paste() -> Result<()> {
    use objc2_core_graphics::{
        CGEvent, CGEventFlags, CGEventSource, CGEventSourceStateID, CGEventTapLocation,
    };

    // Create an event source for keyboard events using HID system state
    let event_source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .ok_or_else(|| anyhow!("Failed to create CGEventSource"))?;

    // Create Command key down event (0x37 is left Command key)
    let cmd_down_event = CGEvent::new_keyboard_event(Some(&event_source), 0x37, true)
        .ok_or_else(|| anyhow!("Failed to create Command down event"))?;

    // Create V key down event (0x09 is V key)
    let v_down_event = CGEvent::new_keyboard_event(Some(&event_source), 0x09, true)
        .ok_or_else(|| anyhow!("Failed to create V down event"))?;

    // Create V key up event
    let v_up_event = CGEvent::new_keyboard_event(Some(&event_source), 0x09, false)
        .ok_or_else(|| anyhow!("Failed to create V up event"))?;

    // Create Command key up event
    let cmd_up_event = CGEvent::new_keyboard_event(Some(&event_source), 0x37, false)
        .ok_or_else(|| anyhow!("Failed to create Command up event"))?;

    // Set Command modifier flag on all events
    let cmd_flags = CGEventFlags::MaskCommand;
    CGEvent::set_flags(Some(&cmd_down_event), cmd_flags);
    CGEvent::set_flags(Some(&v_down_event), cmd_flags);
    CGEvent::set_flags(Some(&v_up_event), cmd_flags);
    CGEvent::set_flags(Some(&cmd_up_event), cmd_flags);

    // Post events to HID event tap with proper timing
    // Note: We assume the calling application has yielded focus or hidden its window
    // before calling this, otherwise we might paste into ourselves.
    
    CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&cmd_down_event));
    thread::sleep(Duration::from_micros(15000));
    CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&v_down_event));
    thread::sleep(Duration::from_micros(15000));
    CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&v_up_event));
    thread::sleep(Duration::from_micros(15000));
    CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&cmd_up_event));

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn simulate_paste() -> Result<()> {
    // Fallback or no-op for other platforms for now
    println!("Paste simulation not implemented for this platform");
    Ok(())
}
