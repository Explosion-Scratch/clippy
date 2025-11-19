use std::process::Command;

#[cfg(target_os = "macos")]
pub fn check_accessibility() -> bool {
    // Use AppleScript to check if the process is trusted
    // Note: This check often requires the app to actually attempt something,
    // but checking via AppleScript is a common workaround to avoid crashing.
    // However, the most reliable way is usually AXIsProcessTrusted from ApplicationServices,
    // which requires linking against the framework.
    // Since we are in a pure Rust environment without complex bindings in this file,
    // we'll use a simpler heuristic: try to create a CGEventSource.

    // Actually, the most robust way without C bindings is to use the system "tccutil"
    // or just attempt an operation.

    // Let's try using `test_accessibility` which attempts to create an event source.
    // If it fails, we likely lack permissions.
    test_accessibility_event_source()
}

#[cfg(target_os = "macos")]
fn test_accessibility_event_source() -> bool {
    use objc2_core_graphics::CGEventSource;
    use objc2_core_graphics::CGEventSourceStateID;

    unsafe {
        // Trying to create a HIDSystemState event source often fails if not trusted
        // when trying to post events, but creation might succeed.
        // Let's link to the C function if possible, or assume false if we can't verify.

        #[link(name = "ApplicationServices", kind = "framework")]
        unsafe extern "C" {
            fn AXIsProcessTrusted() -> bool;
        }

        AXIsProcessTrusted()
    }
}

#[cfg(not(target_os = "macos"))]
pub fn check_accessibility() -> bool {
    true // Not applicable on Linux/Windows usually
}

pub fn request_accessibility() {
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open")
            .args(["x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"])
            .spawn();
    }
}
