# Clippy Clipboard Manager

You are an expert developer working on Clippy, a macOS-native clipboard manager built with Tauri, Vue 3, and Rust.

## Project Knowledge

- **Tech Stack:** Tauri 2.x, Vue 3 (Composition API), Rust, LESS
- **Architecture:** Three-tier desktop app with sidecar CLI service
- **Storage:** File-system based with SHA-256 deduplication (no database)

### Directory Structure

| Directory | Purpose |
|-----------|---------|
| `src/` | Main Clippy app UI (Vue 3) |
| `src-tauri/` | Tauri backend (Rust) |
| `get_clipboard/` | Core clipboard service, API, and CLI (Rust) |
| `frontend/` | Marketing landing page (Vue 3) |

## Commands

```bash
# Primary build & dev (use justfile)
just build              # Build sidecar + main app
just dev                # Run in development mode
just increment-version  # Bump version across all configs

# Individual builds
./build-sidecar.sh      # Build get_clipboard for bundling
bun run tauri build     # Production Tauri build
bun run tauri dev       # Development with hot reload

# get_clipboard CLI (after build)
./target/release/get_clipboard list    # List clipboard items
./target/release/get_clipboard serve   # Start API server (port 3016)
```

## Code Style

### Rust (src-tauri/, get_clipboard/)
```rust
// ✅ Good - inline format args, proper error handling
fn get_item(id: &str) -> Result<Item, String> {
    let path = format!("{}/items/{}", data_dir, id);
    fs::read_to_string(&path).map_err(|e| format!("Failed to read {id}: {e}"))
}

// ❌ Bad - separate format args, unwrap
fn get_item(id: &str) -> Item {
    let path = format!("{}/items/{}", data_dir, id);
    fs::read_to_string(&path).unwrap()
}
```

### Vue (src/, frontend/)
```vue
<!-- ✅ Good - script setup, CSS variables -->
<script setup>
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const items = ref([])
await invoke('get_items').then(data => items.value = data)
</script>

<style scoped lang="less">
.item { color: var(--text-primary); }
</style>
```

## Conventions

- **Sidecar Pattern**: `get_clipboard` runs as a separate process; Tauri communicates via local HTTP API on port 3016
- **FS-Based Storage**: Items stored in `~/Library/Application Support/com.clipboard/data/` with hash-based directory nesting
- **Version Sync**: Use `just increment-version` to update versions across `tauri.conf.json`, `Cargo.toml`, and `package.json`
- **macOS Native**: Use vibrancy, transparency, and hidden title bars for native feel

## Boundaries

- ✅ **Always do:**
  - Use `Result<T, String>` for Tauri commands
  - Run `cargo fmt` after Rust changes
  - Use `<script setup>` for Vue components
  - Use CSS variables for theming

- ⚠️ **Ask first:**
  - Adding new Tauri plugins
  - Changing window behavior or policies
  - Modifying sidecar communication protocol

- 🚫 **Never do:**
  - Add SQLite or any database (use file-system storage)
  - Edit version fields manually (use `just increment-version`)
  - Access filesystem directly from Vue (use Tauri commands)
  - Use `.unwrap()` in production Rust code
