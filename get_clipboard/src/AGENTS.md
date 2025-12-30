# get_clipboard Source

You are a Rust specialist working on the get_clipboard source modules.

## Project Knowledge

- **Purpose:** Core library and binary for clipboard service
- **Pattern:** Domain-driven module organization

### Module Overview

| Module | Purpose |
|--------|---------|
| `api/` | Axum HTTP API server |
| `bin/` | Additional binaries (benchmarks, tests) |
| `cli/` | CLI argument parsing and command handlers |
| `clipboard/` | Clipboard capture and format plugins |
| `config/` | Configuration file management |
| `data/` | File-system storage layer |
| `fs/` | File system utilities |
| `search/` | Search index and queries |
| `service/` | Background clipboard monitoring |
| `tui/` | Terminal UI (Ratatui) |
| `util/` | Shared utilities |

### Entry Points

```rust
// main.rs - CLI entry
fn main() -> Result<()> {
    cli::run()
}

// lib.rs - Library exports
pub mod api;
pub mod cli;
pub mod clipboard;
// ...
```

## Code Style

### Module Organization
```rust
// mod.rs pattern for multi-file modules
// module/mod.rs
mod internal;
pub mod public_api;

pub use public_api::*;

// Single-file modules
// simple_module.rs
pub fn public_function() { ... }
fn private_helper() { ... }
```

### Re-exports
```rust
// ✅ Good - explicit public API
pub use plugins::{ClipboardPlugin, PluginCapture};

// ❌ Bad - exposing implementation details
pub use plugins::*;
```

## Conventions

- **Domain Modules**: Each directory handles one domain (api, clipboard, data, etc.)
- **mod.rs Pattern**: Multi-file modules use `mod.rs` as entry point
- **Explicit Exports**: Re-export only the public API
- **Cross-Module Dependencies**: Keep to minimum; use `crate::` prefix

## Boundaries

- ✅ **Always do:**
  - Keep modules focused on single domain
  - Use explicit re-exports
  - Document public APIs

- ⚠️ **Ask first:**
  - Adding new top-level modules
  - Changing public API surface

- 🚫 **Never do:**
  - Create circular dependencies
  - Use `pub use *` glob re-exports
  - Put business logic in `main.rs` or `lib.rs`
