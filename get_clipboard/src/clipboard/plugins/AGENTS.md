# Clipboard Plugins

You are a Rust plugin specialist working on get_clipboard's format handlers.

## Project Knowledge

- **Tech Stack:** Handlebars (templates), serde_json
- **Purpose:** Format-specific capture, storage, preview, and editing
- **Architecture:** Trait-based plugin system

### File Structure

| File | Purpose |
|------|---------|
| `mod.rs` | Plugin trait, registry, shared types |
| `text.rs` | Plain text handling |
| `html.rs` | HTML content |
| `image.rs` | Images (PNG, JPEG, etc.) |
| `files.rs` | File references |
| `rtf.rs` | Rich Text Format |
| `README.md` | Plugin development guide |

### Plugin Registry
```rust
static REGISTRY: Lazy<Vec<&'static dyn ClipboardPlugin>> = Lazy::new(|| {
    vec![
        FILES_PLUGIN,
        IMAGE_PLUGIN,
        TEXT_PLUGIN,
        HTML_PLUGIN,
        RTF_PLUGIN,
    ]
});
```

## Code Style

### Plugin Trait
```rust
pub trait ClipboardPlugin: Send + Sync {
    fn name(&self) -> &'static str;
    
    fn can_handle(&self, snapshot: &ClipboardSnapshot) -> bool;
    
    fn capture(&self, snapshot: &ClipboardSnapshot) -> Result<PluginCapture>;
    
    fn searchable_text(&self, capture: &PluginCapture) -> Option<String>;
    
    fn preview_template_name(&self) -> String;
    
    fn get_preview_data(&self, ctx: &PluginContext<'_>) -> Result<Value>;
    
    fn get_summary(&self, is_tty: bool, ctx: &PluginContext<'_>) -> Option<String>;
    
    fn is_editable(&self) -> bool { false }
    
    fn get_editable_text(&self, ctx: &PluginContext<'_>) -> Result<String>;
    
    fn edit_item(&self, new_text: &str) -> Result<PluginImport>;
}
```

### Implementing a Plugin
```rust
pub static TEXT_PLUGIN: &TextPlugin = &TextPlugin;

struct TextPlugin;

impl ClipboardPlugin for TextPlugin {
    fn name(&self) -> &'static str { "text" }
    
    fn can_handle(&self, snapshot: &ClipboardSnapshot) -> bool {
        snapshot.types.iter().any(|t| t == "public.utf8-plain-text")
    }
    
    fn capture(&self, snapshot: &ClipboardSnapshot) -> Result<PluginCapture> {
        let text = snapshot.get_string("public.utf8-plain-text")?;
        Ok(PluginCapture {
            plugin_name: self.name().to_string(),
            files: vec![StoredFile::new("text.txt", text.as_bytes())],
            metadata: json!({}),
        })
    }
    
    fn get_summary(&self, is_tty: bool, ctx: &PluginContext<'_>) -> Option<String> {
        let text = ctx.files.get("text.txt")?.read_string().ok()?;
        let line = text.lines().next()?;
        Some(if is_tty {
            truncate(line, 80)
        } else {
            line.to_string()
        })
    }
}
```

### Preview Data Pattern
```rust
fn get_preview_data(&self, ctx: &PluginContext<'_>) -> Result<Value> {
    Ok(json!({
        "text": ctx.files.get("text.txt")?.read_string()?,
        "lineCount": ctx.metadata.line_count,
        "isEditable": self.is_editable(),
    }))
}
```

## Conventions

- **Plugin Priority**: Order in registry determines which plugin wins when multiple match
- **Files Over Memory**: Store content in files, not plugin metadata
- **Idempotent Capture**: Same clipboard content should produce same capture
- **Template Naming**: Use `{plugin_name}.hbs` in `templates/`
- **Regex Optimization**: Use `std::sync::OnceLock` for regex compilation to avoid overhead.

## Adding a New Plugin

1. Create `new_format.rs` in this directory
2. Implement `ClipboardPlugin` trait
3. Create `pub static NEW_FORMAT_PLUGIN: &NewFormatPlugin = &NewFormatPlugin;`
4. Add to registry in `mod.rs`
5. Create preview template in `templates/new_format.hbs`

## Boundaries

- ✅ **Always do:**
  - Implement all required trait methods
  - Store content as files, not in metadata
  - Provide searchable text when applicable
  - Create preview template

- ⚠️ **Ask first:**
  - Changing plugin priority/order
  - Modifying the trait interface
  - Adding dependencies for new formats

- 🚫 **Never do:**
  - Store large content in metadata JSON
  - Skip `is_editable()` for editable formats
  - Forget to register in REGISTRY
