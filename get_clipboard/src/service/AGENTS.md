# Service Module

You are a Rust specialist working on get_clipboard's background monitoring service.

## Project Knowledge

- **Purpose:** Background clipboard monitoring across platforms
- **Architecture:** Platform-specific implementations with common interface

### File Structure

| File | Purpose |
|------|---------|
| `mod.rs` | Common interface and platform dispatch |
| `macos.rs` | macOS implementation |
| `linux.rs` | Linux implementation |
| `windows.rs` | Windows implementation |
| `unsupported.rs` | Fallback for unsupported platforms |
| `watch.rs` | Clipboard change detection |
| `permissions.rs` | Permission checks |

## Code Style

### Platform Dispatch
```rust
// mod.rs
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

pub fn start_service() -> Result<()> {
    #[cfg(target_os = "macos")]
    return macos::start();
    
    #[cfg(target_os = "linux")]
    return linux::start();
    
    #[cfg(target_os = "windows")]
    return windows::start();
    
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    return Err(anyhow!("Unsupported platform"));
}
```

### macOS Service (`macos.rs`)
```rust
pub fn start() -> Result<()> {
    // Register as LaunchAgent for persistent monitoring
    let listener = ClipboardListener::new()?;
    
    loop {
        if listener.has_changed()? {
            let snapshot = ClipboardSnapshot::capture()?;
            store_snapshot(snapshot)?;
        }
        thread::sleep(Duration::from_millis(500));
    }
}
```

### Watch Pattern (`watch.rs`)
```rust
pub struct ClipboardListener {
    last_change_count: u64,
}

impl ClipboardListener {
    pub fn has_changed(&mut self) -> Result<bool> {
        let current = get_change_count()?;
        if current != self.last_change_count {
            self.last_change_count = current;
            return Ok(true);
        }
        Ok(false)
    }
}
```

## Conventions

- **Platform Conditionals**: Use `#[cfg(target_os = "...")]` for platform code
- **Polling**: macOS uses 500ms polling interval
- **LaunchAgent**: macOS runs as LaunchAgent for persistence
- **Change Detection**: Compare change count, not content

## Boundaries

- ✅ **Always do:**
  - Use cfg attributes for platform code
  - Handle platform-specific errors gracefully
  - Check permissions before starting service

- ⚠️ **Ask first:**
  - Changing polling interval
  - Adding platform support
  - Modifying LaunchAgent behavior

- 🚫 **Never do:**
  - Mix platform code without cfg guards
  - Compare full content for change detection
  - Skip permission checks on macOS
