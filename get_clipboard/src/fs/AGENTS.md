# File System Utilities

You are a Rust specialist working on get_clipboard's file system helpers.

## Project Knowledge

- **Purpose:** File system utilities and path helpers
- **Consumers:** Data storage, configuration, templates

### File Structure

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports |
| `layout.rs` | Directory layout and path helpers |

## Code Style

### Path Helpers
```rust
pub fn data_dir_path() -> Result<PathBuf> {
    dirs::data_dir()
        .context("Could not determine data directory")?
        .join("com.clipboard")
        .join("data")
        .pipe(Ok)
}

pub fn config_path() -> Result<PathBuf> {
    dirs::config_dir()
        .context("Could not determine config directory")?
        .join("com.clipboard")
        .join("config.json")
        .pipe(Ok)
}
```

### Directory Operations
```rust
pub fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)
            .context(format!("Failed to create directory: {}", path.display()))?;
    }
    Ok(())
}

pub fn list_entries(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = vec![];
    for entry in fs::read_dir(dir)? {
        entries.push(entry?.path());
    }
    entries.sort();
    Ok(entries)
}
```

## Conventions

- **Platform Paths**: Use `dirs` crate for platform-appropriate directories
- **Error Context**: Always add context to path operations
- **Sorted Listings**: Return directory listings in sorted order
- **Path Display**: Use `.display()` in error messages

## Boundaries

- ✅ **Always do:**
  - Use `dirs` crate for platform paths
  - Add context to errors with path info
  - Return sorted directory listings

- ⚠️ **Ask first:**
  - Changing base directory locations
  - Adding new path helpers

- 🚫 **Never do:**
  - Hardcode absolute paths
  - Use `unwrap()` on path operations
  - Assume directories exist
