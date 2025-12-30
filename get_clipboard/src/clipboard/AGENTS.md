# Clipboard Module

You are a Rust specialist working on get_clipboard's clipboard capture system.

## Project Knowledge

- **Tech Stack:** clipboard-rs (cross-platform clipboard access)
- **Purpose:** Capture clipboard contents and delegate to format plugins
- **Architecture:** Plugin-based format handling

### File Structure

| File | Purpose |
|------|---------|
| `mod.rs` | Exports, ClipboardSnapshot type |
| `mac.rs` | macOS-specific clipboard access |
| `snapshot.rs` | Snapshot creation and management |
| `plugins/` | Format-specific handlers |

## Code Style

### Snapshot Pattern
```rust
#[derive(Debug, Clone)]
pub struct ClipboardSnapshot {
    pub timestamp: OffsetDateTime,
    pub types: Vec<String>,           // Available pasteboard types
    pub contents: Vec<ClipboardContent>, // Raw content by type
}

impl ClipboardSnapshot {
    pub fn capture() -> Result<Self> {
        let clipboard = ClipboardContext::new()?;
        // Capture all available formats...
    }
}
```

### Plugin Integration
```rust
// Capture delegates to plugins for format-specific handling
pub fn capture_and_store() -> Result<EntryMetadata> {
    let snapshot = ClipboardSnapshot::capture()?;
    let captures = plugins::process(&snapshot)?;
    store::persist_entry(captures)
}
```

## Conventions

- **Snapshot First**: Always create a full snapshot before processing
- **Plugin Delegation**: Don't add format-specific logic here; add plugins instead
- **Type Preservation**: Store all available clipboard types, not just primary

## Boundaries

- ✅ **Always do:**
  - Capture all available clipboard formats
  - Create snapshot before delegating to plugins
  - Handle clipboard access errors gracefully

- ⚠️ **Ask first:**
  - Adding new platform-specific code
  - Changing snapshot structure

- 🚫 **Never do:**
  - Add format-specific parsing here (use plugins)
  - Ignore clipboard types that might be useful
  - Skip error handling for empty clipboard
