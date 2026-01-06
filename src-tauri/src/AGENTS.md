# Tauri Source Modules

You are a Rust specialist working on Clippy's Tauri backend modules.

## Project Knowledge

- **Purpose:** Native macOS functionality for the Clippy app
- **Pattern:** Each file is a focused module with related functionality

### Module Responsibilities

| File | Responsibility |
|------|----------------|
| `lib.rs` | Main entry, plugin setup, Tauri commands |
| `sidecar.rs` | Sidecar process management and API proxy |
| `settings.rs` | User settings persistence |
| `tray.rs` | System tray menu and icon management |
| `preview.rs` | Preview window content and rendering |
| `visibility.rs` | macOS window visibility helpers |
| `windows.rs` | Window creation and management |
| `paste.rs` | CoreGraphics keyboard simulation |
| `accessibility.rs` | macOS accessibility checks |
| `api.rs` | API URL helpers |
| `polling.rs` | Clipboard change polling |
| `shortcut.rs` | Global hotkey utilities |
| `clipboard.rs` | Clipboard utilities |
| `main.rs` | Binary entry point |

## Code Style

### Module Structure
```rust
// Each module should have clear public API
pub fn do_something() -> Result<(), String> { ... }

// Keep implementation details private
fn internal_helper() { ... }
```

### Error Handling
```rust
// ✅ Good - map_err for context
fn load_settings() -> Result<Settings, String> {
    let path = settings_path()?;
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read settings at {}: {e}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Invalid settings JSON: {e}"))
}

// ❌ Bad - generic error, no context
fn load_settings() -> Result<Settings, String> {
    let content = fs::read_to_string(settings_path()?).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}
```

### Async Patterns
```rust
// Use spawn_blocking for CPU-intensive or blocking operations
tauri::async_runtime::spawn_blocking(|| {
    // Simulate keypress (blocking CGEvent operations)
    simulate_paste()
}).await.map_err(|e| e.to_string())?
```

## Key Patterns

### Sidecar Communication (`sidecar.rs`)
```rust
// All sidecar calls go through HTTP API
pub async fn get_items() -> Result<Vec<Item>, String> {
    let url = format!("{}/items", api::base_url());
    reqwest::get(&url).await?.json().await.map_err(|e| e.to_string())
}
```

### Window Visibility (`visibility.rs`)
```rust
// macOS-specific show/hide with activation policy changes
pub fn show_main_window(window: &WebviewWindow) -> Result<(), String> {
    window.show()?;
    // Set activation policy to Accessory for popup behavior
    Ok(())
}
```

### Paste Simulation (`paste.rs`)
```rust
// CoreGraphics event simulation
pub fn simulate_paste() -> Result<(), String> {
    // Create and post CGEvent for Cmd+V
}
```

### Static Initialization
```rust
// Use OnceLock for efficient static resource initialization
static REGEX: OnceLock<Regex> = OnceLock::new();
fn get_regex() -> &'static Regex {
    REGEX.get_or_init(|| Regex::new("pattern").unwrap())
}
```

## Boundaries

- ✅ **Always do:**
  - Add context to errors with `map_err`
  - Use `spawn_blocking` for blocking operations
  - Keep modules focused on single responsibility

- ⚠️ **Ask first:**
  - Adding new modules
  - Changing `lib.rs` structure significantly
  - Adding new dependencies

- 🚫 **Never do:**
  - Use `.unwrap()` or `.expect()`
  - Call sidecar API from multiple modules (centralize in `sidecar.rs`)
  - Add platform-specific code without `#[cfg(target_os = "macos")]`
