# Tauri Backend

You are a Rust/Tauri specialist working on Clippy's native macOS backend.

## Project Knowledge

- **Tech Stack:** Tauri 2.x, Rust, tokio, reqwest
- **Purpose:** Native window management, system tray, global shortcuts, sidecar API bridge
- **Entry:** `main.rs` → `lib.rs::run()`

### Module Overview

| Module | Purpose |
|--------|---------|
| `lib.rs` | Main entry, Tauri commands, window/tray setup |
| `sidecar.rs` | Spawns and manages `get_clipboard` process |
| `visibility.rs` | macOS-specific window show/hide logic |
| `paste.rs` | Simulates Cmd+V via CoreGraphics |
| `settings.rs` | Settings persistence and management |
| `shortcut.rs` | Global hotkey registration |
| `accessibility.rs` | macOS accessibility permission checks |
| `api.rs` | Sidecar API endpoint helpers |
| `clipboard.rs` | Clipboard utilities |

## Commands

```bash
cd src-tauri
cargo build                    # Debug build
cargo build --release          # Release build
cargo fmt                      # Format code
cargo clippy                   # Lint
cargo test                     # Run tests
```

## Code Style

### Tauri Commands
```rust
// ✅ Good - Result return, proper error conversion
#[tauri::command]
async fn get_items(app: tauri::AppHandle) -> Result<Vec<Item>, String> {
    let response = reqwest::get(format!("{}/items", api::base_url()))
        .await
        .map_err(|e| format!("API request failed: {e}"))?;

    response.json().await.map_err(|e| format!("Parse failed: {e}"))
}

// ❌ Bad - unwrap, blocking
#[tauri::command]
fn get_items() -> Vec<Item> {
    reqwest::blocking::get("http://localhost:3016/items")
        .unwrap()
        .json()
        .unwrap()
}
```

### Window Management
```rust
// Pattern for creating windows
fn open_settings_window(app: &tauri::AppHandle) -> Result<(), Box<dyn Error>> {
    if let Some(window) = app.get_webview_window("settings") {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }

    let window = tauri::WebviewWindowBuilder::new(
        app,
        "settings",
        tauri::WebviewUrl::App("/#/settings".into()),
    )
    .title("Settings")
    .inner_size(600.0, 500.0)
    .build()?;

    Ok(())
}
```

### State Pattern
```rust
// Shared state with Arc<Mutex<T>>
type TrayClipboardItems = Arc<Mutex<Vec<(String, String)>>>;

#[tauri::command]
async fn update_tray(
    items: tauri::State<'_, TrayClipboardItems>,
) -> Result<(), String> {
    let mut guard = items.lock().map_err(|e| e.to_string())?;
    // Update items...
    Ok(())
}
```

## Conventions

- **Async Commands**: Use `async` with `tokio` for I/O operations
- **Window Types**:
  - `main`: Search UI, Accessory activation policy
  - `preview`: Non-focusable hover window with HudWindow vibrancy
  - `settings`/`welcome`: Standard windows with Regular policy
- **Sidecar API**: Always use HTTP API (port 3016), never access files directly
- **Vibrancy**: Apply `NSVisualEffectMaterial::HudWindow` on main thread after window visible

## Boundaries

- ✅ **Always do:**
  - Return `Result<T, String>` from commands
  - Use `cargo fmt` after changes
  - Prefer async over blocking operations
  - Use `tauri::State` for shared data

- ⚠️ **Ask first:**
  - Adding new Tauri plugins
  - Changing window activation policies
  - Modifying sidecar spawn behavior

- 🚫 **Never do:**
  - Use `.unwrap()` or `.expect()` in production code
  - Block the main thread
  - Access `~/Library/.../data/` directly (use sidecar API)
  - Make preview window focusable during normal browsing
