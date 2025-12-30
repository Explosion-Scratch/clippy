# Utility Modules

You are a Rust specialist working on get_clipboard's shared utilities.

## Project Knowledge

- **Purpose:** Common utilities used across the codebase
- **Pattern:** Pure functions with no external dependencies

### File Structure

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports |
| `hash.rs` | SHA-256 hashing utilities |
| `time.rs` | Time formatting helpers |
| `paste.rs` | Clipboard paste simulation |

## Code Style

### Hash Utilities (`hash.rs`)
```rust
use sha2::{Digest, Sha256};

pub fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

pub fn sha256_string(s: &str) -> String {
    sha256_bytes(s.as_bytes())
}
```

### Time Formatting (`time.rs`)
```rust
use time::{OffsetDateTime, format_description};

pub fn format_relative(dt: OffsetDateTime) -> String {
    let now = OffsetDateTime::now_utc();
    let duration = now - dt;
    
    if duration.whole_minutes() < 1 {
        "just now".to_string()
    } else if duration.whole_hours() < 1 {
        format!("{}m ago", duration.whole_minutes())
    } else if duration.whole_days() < 1 {
        format!("{}h ago", duration.whole_hours())
    } else {
        format!("{}d ago", duration.whole_days())
    }
}

pub fn format_timestamp(dt: OffsetDateTime) -> String {
    // ISO-8601 format
    dt.format(&format_description::well_known::Rfc3339).unwrap_or_default()
}
```

### Paste Simulation (`paste.rs`)
```rust
// Platform-specific paste simulation
#[cfg(target_os = "macos")]
pub fn simulate_paste() -> Result<()> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGKeyCode};
    
    // Create Cmd+V key event
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)?;
    let event = CGEvent::new_keyboard_event(source, CGKeyCode::V, true)?;
    event.set_flags(CGEventFlags::CGEventFlagCommand);
    event.post(CGEventTapLocation::HID);
    
    Ok(())
}
```

## Conventions

- **Pure Functions**: No side effects except `paste.rs`
- **No Dependencies**: Utilities should be self-contained
- **Platform Guards**: Use `#[cfg]` for platform-specific code
- **String Returns**: Hash functions return hex strings

## Boundaries

- ✅ **Always do:**
  - Keep functions pure when possible
  - Use `#[cfg]` for platform code
  - Document function behavior

- ⚠️ **Ask first:**
  - Adding new utility modules
  - Adding external dependencies

- 🚫 **Never do:**
  - Add business logic to utilities
  - Create stateful utilities
  - Skip cfg guards for platform code
