# get_clipboard - Core Clipboard Service

You are a Rust specialist working on `get_clipboard`, the core clipboard monitoring and storage service.

## Project Knowledge

- **Tech Stack:** Rust, Axum (web), Clap (CLI), Ratatui (TUI), clipboard-rs
- **Purpose:** Background clipboard monitoring, storage, CLI interface, and HTTP API
- **Binaries:** `get_clipboard` (main CLI/service), benchmarks, test utilities

### Architecture

```
get_clipboard/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library exports
│   ├── api/             # HTTP API server (Axum)
│   ├── bin/             # Additional binaries
│   ├── cli/             # CLI argument parsing and handlers
│   ├── clipboard/       # Clipboard capture and plugins
│   ├── config/          # Configuration management
│   ├── data/            # Storage layer (file-system)
│   ├── fs/              # File system utilities
│   ├── search/          # Index and search
│   ├── service/         # Background monitoring service
│   ├── tui/             # Terminal UI
│   └── util/            # Shared utilities
├── templates/           # Handlebars preview templates
└── frontend-app/        # Dashboard web UI
```

## Commands

```bash
# Build
cargo build -p get_clipboard              # Debug build
cargo build -p get_clipboard --release    # Release build

# Run
cargo run -p get_clipboard -- list        # List items
cargo run -p get_clipboard -- serve       # Start API server (port 3016)
cargo run -p get_clipboard -- watch       # Monitor clipboard
cargo run -p get_clipboard -- --help      # All commands

# Test
cargo test -p get_clipboard               # Run tests
cargo fmt                                 # Format
cargo clippy                              # Lint
```

## Code Style

### Result Pattern with anyhow
```rust
use anyhow::{Context, Result, bail};

// ✅ Good - context-rich errors
fn load_item(hash: &str) -> Result<Item> {
    let path = item_path(hash)?;
    let content = fs::read_to_string(&path)
        .context(format!("Failed to read item {hash}"))?;
    serde_json::from_str(&content)
        .context("Invalid item JSON")
}

// ❌ Bad - no context
fn load_item(hash: &str) -> Result<Item> {
    let content = fs::read_to_string(item_path(hash)?)?;
    Ok(serde_json::from_str(&content)?)
}
```

### CLI Output
```rust
// TTY-aware formatting
fn print_item(item: &Item, is_tty: bool) {
    if is_tty {
        println!("{:>3} │ {}", item.index, item.summary.cyan());
    } else {
        println!("{}\t{}", item.index, item.summary);
    }
}
```

## Conventions

- **File-System Storage**: Items stored in `~/Library/Application Support/com.clipboard/data/`
- **Hash-Based Paths**: SHA-256 hash determines storage path: `data/ab/cd/abcd1234.../`
- **Plugin Architecture**: Format-specific handlers (text, html, image, files, rtf)
- **API Port**: 3016 (hardcoded)
- **TTY Detection**: Adjust output formatting based on `stdout.is_terminal()`

## Boundaries

- ✅ **Always do:**
  - Use `anyhow::Result` for library code
  - Add `.context()` to errors
  - Run `cargo fmt` and `cargo clippy`
  - Support both TTY and pipe output

- ⚠️ **Ask first:**
  - Changing storage format or paths
  - Adding new clipboard plugins
  - Modifying API endpoints

- 🚫 **Never do:**
  - Use SQLite or any database
  - Change the API port without updating Tauri
  - Store sensitive data unencrypted
  - Use `.unwrap()` outside tests
