use std::process::Command;

#[cfg(target_os = "macos")]
pub fn check_accessibility() -> bool {
    unsafe { is_process_trusted() }
}

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

#[cfg(target_os = "macos")]
unsafe fn is_process_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

#[cfg(not(target_os = "macos"))]
pub fn check_accessibility() -> bool {
    true // Not applicable on Linux/Windows usually
}

pub fn request_accessibility() {
    #[cfg(target_os = "macos")]
    {
        // Opening the preference pane is the standard way to request
        // We can also trigger the system prompt by attempting an action,
        // calling AXIsProcessTrustedWithOptions can trigger it.
        let _ = Command::new("open")
            .args(["x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"])
            .spawn();
    }
}
