# Clipboard Plugins

This module implements a plugin architecture for handling different clipboard data formats. Each plugin is responsible for capturing, storing, displaying, and exporting a specific type of clipboard content.

## Overview

Plugins extend the `ClipboardPlugin` trait and are registered in the `REGISTRY` static. The system automatically selects plugins based on clipboard content.

## Plugin Trait

```rust
pub trait ClipboardPlugin: Sync + Send {
    // Required methods
    fn id(&self) -> &'static str;
    fn kind(&self) -> &'static str;
    fn priority(&self) -> u8;
    fn entry_kind(&self) -> EntryKind;
    fn matches(&self, snapshot: &ClipboardSnapshot) -> bool;
    fn capture(&self, snapshot: &ClipboardSnapshot) -> Option<PluginCapture>;
    fn to_clipboard_items(&self, ctx: &PluginContext<'_>) -> Result<Vec<ClipboardContent>>;
    fn display_content(&self, ctx: &PluginContext<'_>) -> Result<DisplayContent>;
    fn export_json(&self, ctx: &PluginContext<'_>) -> Result<Value>;
    fn import_json(&self, format: &ClipboardJsonFormat) -> Result<PluginImport>;
    fn detail_log(&self, ctx: &PluginContext<'_>) -> Result<Vec<(String, String)>>;

    // Optional methods with defaults
    fn searchable_text(&self, snapshot: &ClipboardSnapshot, capture: &PluginCapture) -> Option<String>;
    fn preview_template_name(&self) -> String;
    fn get_preview_priority(&self) -> u8;
    fn get_preview_data(&self, ctx: &PluginContext<'_>) -> Result<Value>;
}
```

## Method Reference

### Required Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `id()` | `&'static str` | Unique plugin identifier (e.g., "text", "image") |
| `kind()` | `&'static str` | Content category (e.g., "text", "image", "file") |
| `priority()` | `u8` | Lower = higher priority when multiple plugins match |
| `entry_kind()` | `EntryKind` | Classification for the entry (Text, Image, File, Other) |
| `matches()` | `bool` | Returns true if plugin can handle this clipboard snapshot |
| `capture()` | `Option<PluginCapture>` | Captures clipboard data into storable format |
| `to_clipboard_items()` | `Result<Vec<ClipboardContent>>` | Converts stored data back to clipboard format |
| `display_content()` | `Result<DisplayContent>` | Returns content for display (text, HTML, or path) |
| `export_json()` | `Result<Value>` | Serializes content for JSON export |
| `import_json()` | `Result<PluginImport>` | Reconstructs plugin data from JSON import |
| `detail_log()` | `Result<Vec<(String, String)>>` | Returns key-value pairs for logging/debugging |

### Optional Methods (with defaults)

| Method | Default | Description |
|--------|---------|-------------|
| `searchable_text()` | Returns `capture.summary` | Text used for full-text search indexing |
| `preview_template_name()` | `"{plugin_id}.hbs"` | Handlebars template file for preview rendering |
| `get_preview_priority()` | `self.priority()` | Order in format tabs (lower = first) |
| `get_preview_data()` | Empty object `{}` | JSON data passed to the preview template |

## Built-in Plugins

### Text Plugin (`text`)
- **Priority:** 2
- **Kind:** text
- **Handles:** Plain text from clipboard
- **Preview:** Detects SVGs, color values, and URLs with link previews

### Image Plugin (`image`)
- **Priority:** 1
- **Kind:** image
- **Handles:** PNG, JPEG, TIFF, BMP image data
- **Preview:** Zoomable/pannable image with dimensions

### Files Plugin (`files`)
- **Priority:** 0
- **Kind:** file
- **Handles:** File URLs / file references
- **Preview:** Clickable file list with sizes

### HTML Plugin (`html`)
- **Priority:** 3
- **Kind:** text
- **Handles:** HTML formatted content
- **Preview:** Rendered HTML in iframe

### RTF Plugin (`rtf`)
- **Priority:** 4
- **Kind:** text
- **Handles:** Rich Text Format content
- **Preview:** Raw RTF code display

## Creating a New Plugin

### 1. Create the Plugin Module

```rust
// src/clipboard/plugins/my_plugin.rs
use anyhow::Result;
use serde_json::json;

use super::{
    ClipboardPlugin, ClipboardJsonFormat, PluginCapture, 
    PluginContext, PluginImport, DisplayContent
};

pub static MY_PLUGIN: &MyPlugin = &MyPlugin;

pub struct MyPlugin;

impl ClipboardPlugin for MyPlugin {
    fn id(&self) -> &'static str { "my_format" }
    fn kind(&self) -> &'static str { "text" }
    fn priority(&self) -> u8 { 10 }
    fn entry_kind(&self) -> EntryKind { EntryKind::Other }
    
    fn matches(&self, snapshot: &ClipboardSnapshot) -> bool {
        // Check if clipboard has your format
        false
    }
    
    fn capture(&self, snapshot: &ClipboardSnapshot) -> Option<PluginCapture> {
        // Extract and store data
        None
    }
    
    // ... implement remaining required methods
}
```

### 2. Register the Plugin

In `src/clipboard/plugins/mod.rs`:

```rust
mod my_plugin;
pub use my_plugin::MY_PLUGIN;

static REGISTRY: Lazy<Vec<&'static dyn ClipboardPlugin>> = Lazy::new(|| {
    vec![
        FILES_PLUGIN,
        IMAGE_PLUGIN,
        TEXT_PLUGIN,
        HTML_PLUGIN,
        RTF_PLUGIN,
        MY_PLUGIN,  // Add your plugin
    ]
});
```

### 3. Create Preview Template

Create `templates/my_format.hbs`:

```handlebars
<!DOCTYPE html>
<html>
<head>
    <style>
        {{>style.css}}
        /* Add template-specific styles here */
    </style>
</head>
<body>
    <div class="preview-container">
        <!-- Your preview content -->
        {{{content}}}
    </div>
    <script>
        {{> base_iframe.js }}
    </script>
</body>
</html>
```

## PluginContext

The `PluginContext` provides access to stored data:

```rust
pub struct PluginContext<'a> {
    pub metadata: &'a EntryMetadata,
    pub plugin_meta: &'a Value,
    pub item_dir: &'a Path,
    pub stored_files: Vec<StoredFile>,
}
```

- `metadata`: Entry metadata (hash, timestamps, size, etc.)
- `plugin_meta`: Plugin-specific metadata from capture
- `item_dir`: Directory containing stored files
- `stored_files`: Files stored during capture

## PluginCapture

Returned from `capture()` to store clipboard data:

```rust
pub struct PluginCapture {
    pub plugin_id: &'static str,
    pub kind: &'static str,
    pub entry_kind: EntryKind,
    pub priority: u8,
    pub summary: Option<String>,
    pub search_text: Option<String>,
    pub files: Vec<FileOutput>,
    pub metadata: Value,
    pub byte_size: u64,
    pub sources: Vec<String>,
}
```

## Preview System

Plugins provide preview data via `get_preview_data()`. The returned JSON is merged with:
- `interactive`: Boolean indicating full vs compact preview mode

Templates should handle both modes and use CSS variables from `style.css`.
