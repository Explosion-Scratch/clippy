mod files;
mod html;
mod image;
mod rtf;
mod text;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use clipboard_rs::common::ClipboardContent;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::clipboard::snapshot::{ClipboardSnapshot, FileOutput};
use crate::data::model::EntryMetadata;
use crate::util::hash::sha256_bytes;

pub use files::FILES_PLUGIN;
pub use html::HTML_PLUGIN;
pub use image::IMAGE_PLUGIN;
pub use rtf::RTF_PLUGIN;
pub use text::TEXT_PLUGIN;

pub trait ClipboardPlugin: Sync + Send {
    fn id(&self) -> &'static str;
    fn kind(&self) -> &'static str;
    fn priority(&self) -> u8;
    fn entry_kind(&self) -> crate::data::model::EntryKind;
    fn matches(&self, snapshot: &ClipboardSnapshot) -> bool;
    fn capture(&self, snapshot: &ClipboardSnapshot) -> Option<PluginCapture>;
    fn to_clipboard_items(&self, ctx: &PluginContext<'_>) -> Result<Vec<ClipboardContent>>;
    fn display_content(&self, ctx: &PluginContext<'_>) -> Result<DisplayContent>;
    fn export_json(&self, ctx: &PluginContext<'_>) -> Result<Value>;
    fn import_json(&self, format: &ClipboardJsonFormat) -> Result<PluginImport>;
    fn detail_log(&self, ctx: &PluginContext<'_>) -> Result<Vec<(String, String)>>;
    fn searchable_text(
        &self,
        _snapshot: &ClipboardSnapshot,
        capture: &PluginCapture,
    ) -> Option<String> {
        capture.summary.clone()
    }

    fn preview_template_name(&self) -> String {
        format!("{}.hbs", self.id())
    }

    fn get_preview_priority(&self) -> u8 {
        self.priority()
    }

    fn get_preview_data(&self, _ctx: &PluginContext<'_>) -> Result<Value> {
        Ok(Value::Object(serde_json::Map::new()))
    }

    fn get_summary(&self, _is_tty: bool, ctx: &PluginContext<'_>) -> Option<String> {
        ctx.metadata.summary.clone()
    }
}

#[derive(Debug, Clone)]
pub struct PluginCapture {
    pub plugin_id: &'static str,
    pub kind: &'static str,
    pub entry_kind: crate::data::model::EntryKind,
    pub priority: u8,
    pub summary: Option<String>,
    pub search_text: Option<String>,
    pub files: Vec<FileOutput>,
    pub metadata: Value,
    pub byte_size: u64,
    pub sources: Vec<String>,
}

impl PluginCapture {
    pub fn finalize_metadata(&mut self) {
        let mut meta = match &self.metadata {
            Value::Object(map) => map.clone(),
            _ => Map::new(),
        };

        let stored_files: Vec<Value> = self
            .files
            .iter()
            .map(|file| Value::String(file.filename.clone()))
            .collect();

        if !stored_files.is_empty() {
            meta.insert("storedFiles".into(), Value::Array(stored_files));
        }

        meta.insert("pluginId".into(), Value::String(self.plugin_id.into()));
        meta.insert("pluginKind".into(), Value::String(self.kind.into()));

        if let Some(summary) = &self.summary {
            meta.insert("summary".into(), Value::String(summary.clone()));
        }

        self.metadata = Value::Object(meta);
        if self.search_text.is_none() {
            self.search_text = self.summary.clone();
        }
    }
}

#[derive(Debug)]
pub struct StoredFile {
    pub filename: String,
    pub path: PathBuf,
}

impl StoredFile {
    pub fn read_string(&self) -> Result<String> {
        fs::read_to_string(&self.path)
            .with_context(|| format!("Failed to read {}", self.path.display()))
    }

    pub fn read_bytes(&self) -> Result<Vec<u8>> {
        fs::read(&self.path).with_context(|| format!("Failed to read {}", self.path.display()))
    }
}

pub struct PluginContext<'a> {
    pub metadata: &'a EntryMetadata,
    pub plugin_meta: &'a Value,
    pub item_dir: &'a Path,
    pub stored_files: &'a [StoredFile],
}

#[derive(Debug, Clone)]
pub struct ImageDisplay {
    pub path: PathBuf,
    pub fallback: Option<String>,
}

#[derive(Debug, Clone)]
pub enum DisplayContent {
    Text(String),
    Lines(Vec<String>),
    Image(ImageDisplay),
    Empty,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize)]
pub struct ClipboardJsonItem {
    pub index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _index: Option<usize>,
    pub id: String,
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firstDate: Option<String>,
    #[serde(rename = "type")]
    pub item_type: String,
    pub size: u64,
    pub dataPath: String,
    pub data: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copyCount: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detectedFormats: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardJsonFormat {
    pub plugin_id: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub priority: Option<u8>,
    #[serde(default)]
    pub entry_kind: Option<crate::data::model::EntryKind>,
    #[serde(default)]
    pub data: Value,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardJsonFullItem {
    #[serde(default)]
    pub index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub _index: Option<usize>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub first_date: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(rename = "type", default)]
    pub item_type: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub copy_count: Option<u64>,
    #[serde(default)]
    pub detected_formats: Vec<String>,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub search_text: Option<String>,
    #[serde(rename = "dataPath", default)]
    pub data_path: Option<String>,
    #[serde(default)]
    pub formats: Vec<ClipboardJsonFormat>,
}

pub struct ClipboardJsonImport {
    pub captures: Vec<PluginCapture>,
    pub clipboard_contents: Vec<ClipboardContent>,
}

pub struct PluginImport {
    pub capture: PluginCapture,
    pub clipboard_contents: Vec<ClipboardContent>,
}

#[derive(Debug, Clone)]
pub struct PreviewFormat {
    pub plugin_id: String,
    pub template_name: String,
    pub priority: u8,
    pub data: Value,
    pub text: Option<String>,
}

static REGISTRY: Lazy<Vec<&'static dyn ClipboardPlugin>> = Lazy::new(|| {
    vec![
        FILES_PLUGIN as &'static dyn ClipboardPlugin,
        IMAGE_PLUGIN as &'static dyn ClipboardPlugin,
        TEXT_PLUGIN as &'static dyn ClipboardPlugin,
        HTML_PLUGIN as &'static dyn ClipboardPlugin,
        RTF_PLUGIN as &'static dyn ClipboardPlugin,
    ]
});

pub fn plugin_registry() -> &'static [&'static dyn ClipboardPlugin] {
    REGISTRY.as_slice()
}

pub fn plugin_by_id(id: &str) -> Option<&'static dyn ClipboardPlugin> {
    plugin_registry()
        .iter()
        .copied()
        .find(|plugin| plugin.id() == id)
}

pub fn capture_plugins(snapshot: &ClipboardSnapshot) -> Vec<PluginCapture> {
    let mut captures = Vec::new();
    for plugin in plugin_registry() {
        if plugin.matches(snapshot) {
            if let Some(mut capture) = plugin.capture(snapshot) {
                capture.finalize_metadata();
                if capture.search_text.is_none() {
                    capture.search_text = plugin.searchable_text(snapshot, &capture);
                }
                captures.push(capture);
            }
        }
    }
    captures
}

pub fn prioritized_capture<'a>(captures: &'a [PluginCapture]) -> Option<&'a PluginCapture> {
    captures.iter().min_by_key(|capture| capture.priority)
}

pub fn plugin_order(captures: &[PluginCapture]) -> Vec<String> {
    captures
        .iter()
        .map(|capture| capture.plugin_id.to_string())
        .collect()
}

pub fn rebuild_clipboard_contents(
    metadata: &EntryMetadata,
    item_dir: &Path,
) -> Result<Vec<ClipboardContent>> {
    let (order, map) = extract_plugin_meta(metadata)?
        .ok_or_else(|| anyhow!("Missing plugin metadata for {}", metadata.hash))?;

    let mut results = Vec::new();
    for plugin_id in order {
        let plugin_meta = map
            .get(&plugin_id)
            .ok_or_else(|| anyhow!("Missing plugin metadata for plugin {plugin_id}"))?;
        let plugin = plugin_by_id(&plugin_id)
            .ok_or_else(|| anyhow!("Unknown clipboard plugin {plugin_id}"))?;
        let instance = PluginInstance::new(plugin, metadata, item_dir, plugin_meta)?;
        results.extend(plugin.to_clipboard_items(&instance.context())?);
    }
    Ok(results)
}

pub fn build_display_content(metadata: &EntryMetadata, item_dir: &Path) -> Result<DisplayContent> {
    build_display_content_with_preference(metadata, item_dir, None)
}

pub fn build_display_content_with_preference(
    metadata: &EntryMetadata,
    item_dir: &Path,
    preferred: Option<&str>,
) -> Result<DisplayContent> {
    let (order, map) = extract_plugin_meta(metadata)?
        .ok_or_else(|| anyhow!("Missing plugin metadata for {}", metadata.hash))?;

    if let Some(plugin_id) = preferred {
        if let Some(content) = display_with_plugin(metadata, item_dir, &map, plugin_id)? {
            return Ok(content);
        }
    }

    for plugin_id in order {
        if let Some(content) = display_with_plugin(metadata, item_dir, &map, &plugin_id)? {
            return Ok(content);
        }
    }

    Ok(DisplayContent::Empty)
}

pub fn build_summary(metadata: &EntryMetadata, item_dir: &Path, is_tty: bool) -> Option<String> {
    let (order, map) = match extract_plugin_meta(metadata) {
        Ok(Some(result)) => result,
        _ => return metadata.summary.clone(),
    };

    for plugin_id in order {
        let Some(plugin_meta) = map.get(&plugin_id) else {
            continue;
        };
        let Some(plugin) = plugin_by_id(&plugin_id) else {
            continue;
        };
        let Ok(instance) = PluginInstance::new(plugin, metadata, item_dir, plugin_meta) else {
            continue;
        };
        if let Some(summary) = plugin.get_summary(is_tty, &instance.context()) {
            return Some(summary);
        }
    }

    metadata.summary.clone()
}

fn display_with_plugin(
    metadata: &EntryMetadata,
    item_dir: &Path,
    map: &Map<String, Value>,
    plugin_id: &str,
) -> Result<Option<DisplayContent>> {
    let plugin_meta = match map.get(plugin_id) {
        Some(value) => value,
        None => return Ok(None),
    };
    let plugin = match plugin_by_id(plugin_id) {
        Some(plugin) => plugin,
        None => return Ok(None),
    };
    let instance = PluginInstance::new(plugin, metadata, item_dir, plugin_meta)?;
    Ok(Some(plugin.display_content(&instance.context())?))
}

fn json_with_plugin(
    metadata: &EntryMetadata,
    item_dir: &Path,
    item_path: &Path,
    map: &Map<String, Value>,
    plugin_id: &str,
    index: usize,
    real_index: Option<usize>,
) -> Result<Option<ClipboardJsonItem>> {
    let plugin_meta = match map.get(plugin_id) {
        Some(value) => value,
        None => return Ok(None),
    };
    let plugin = match plugin_by_id(plugin_id) {
        Some(plugin) => plugin,
        None => return Ok(None),
    };
    let instance = PluginInstance::new(plugin, metadata, item_dir, plugin_meta)?;
    let data = plugin.export_json(&instance.context())?;
    Ok(Some(ClipboardJsonItem {
        index,
        _index: real_index,
        id: metadata.hash.clone(),
        date: crate::util::time::format_iso(metadata.last_seen),
        firstDate: Some(crate::util::time::format_iso(metadata.first_seen)),
        item_type: plugin.kind().to_string(),
        size: metadata.byte_size,
        dataPath: item_path.to_string_lossy().to_string(),
        data,
        summary: metadata.summary.clone(),
        copyCount: Some(metadata.copy_count),
        detectedFormats: Some(metadata.detected_formats.clone()),
    }))
}

pub fn build_detail_log(
    metadata: &EntryMetadata,
    item_dir: &Path,
) -> Result<Vec<(String, String)>> {
    if let Some(order_map) = extract_plugin_meta(metadata)? {
        let (order, map) = order_map;
        for plugin_id in order {
            if let Some(plugin_meta) = map.get(&plugin_id) {
                if let Some(plugin) = plugin_by_id(&plugin_id) {
                    let instance = PluginInstance::new(plugin, metadata, item_dir, plugin_meta)?;
                    return plugin.detail_log(&instance.context());
                }
            }
        }
    }
    Ok(Vec::new())
}

pub fn build_json_item(
    metadata: &EntryMetadata,
    item_dir: &Path,
    index: usize,
) -> Result<ClipboardJsonItem> {
    build_json_item_with_preference(metadata, item_dir, index, None, None)
}

pub fn build_json_item_with_preference(
    metadata: &EntryMetadata,
    item_dir: &Path,
    index: usize,
    preferred: Option<&str>,
    real_index: Option<usize>,
) -> Result<ClipboardJsonItem> {
    let item_path = item_dir
        .canonicalize()
        .unwrap_or_else(|_| item_dir.to_path_buf());
    let (order, map) = extract_plugin_meta(metadata)?
        .ok_or_else(|| anyhow!("Missing plugin metadata for {}", metadata.hash))?;

    if let Some(plugin_id) = preferred {
        if let Some(item) = json_with_plugin(
            metadata, item_dir, &item_path, &map, plugin_id, index, real_index,
        )? {
            return Ok(item);
        }
    }
    for plugin_id in order {
        if let Some(item) = json_with_plugin(
            metadata, item_dir, &item_path, &map, &plugin_id, index, real_index,
        )? {
            return Ok(item);
        }
    }

    Err(anyhow!(
        "No plugin could build JSON item for {}",
        metadata.hash
    ))
}

pub fn build_full_json_item(
    metadata: &EntryMetadata,
    item_dir: &Path,
    index: Option<usize>,
    real_index: Option<usize>,
) -> Result<ClipboardJsonFullItem> {
    let item_path = item_dir
        .canonicalize()
        .unwrap_or_else(|_| item_dir.to_path_buf());
    let mut formats = Vec::new();

    let (order, map) = extract_plugin_meta(metadata)?
        .ok_or_else(|| anyhow!("Missing plugin metadata for {}", metadata.hash))?;

    for plugin_id in order {
        let Some(plugin_meta) = map.get(&plugin_id) else {
            continue;
        };
        let Some(plugin) = plugin_by_id(&plugin_id) else {
            continue;
        };
        let instance = PluginInstance::new(plugin, metadata, item_dir, plugin_meta)?;
        let data = plugin.export_json(&instance.context())?;
        formats.push(ClipboardJsonFormat {
            plugin_id: plugin.id().to_string(),
            kind: Some(plugin.kind().to_string()),
            priority: Some(plugin.priority()),
            entry_kind: Some(plugin.entry_kind()),
            data,
            metadata: plugin_meta.clone(),
        });
    }

    Ok(ClipboardJsonFullItem {
        index,
        _index: real_index,
        id: Some(metadata.hash.clone()),
        date: Some(crate::util::time::format_iso(metadata.last_seen)),
        first_date: Some(crate::util::time::format_iso(metadata.first_seen)),
        summary: metadata.summary.clone(),
        item_type: Some(format!("{:?}", metadata.kind)),
        size: Some(metadata.byte_size),
        copy_count: Some(metadata.copy_count),
        detected_formats: metadata.detected_formats.clone(),
        sources: metadata.sources.clone(),
        search_text: metadata.search_text.clone(),
        data_path: Some(item_path.to_string_lossy().to_string()),
        formats,
    })
}

pub fn build_preview_formats(
    metadata: &EntryMetadata,
    item_dir: &Path,
) -> Result<Vec<PreviewFormat>> {
    let mut previews = Vec::new();

    let (order, map) = extract_plugin_meta(metadata)?
        .ok_or_else(|| anyhow!("Missing plugin metadata for {}", metadata.hash))?;

    for plugin_id in order {
        let Some(plugin_meta) = map.get(&plugin_id) else {
            continue;
        };
        let Some(plugin) = plugin_by_id(&plugin_id) else {
            continue;
        };
        let instance = PluginInstance::new(plugin, metadata, item_dir, plugin_meta)?;
        let data = plugin.get_preview_data(&instance.context())?;

        let text = if plugin.id() == "text" {
            data.get("raw_text").and_then(|v| v.as_str()).map(String::from)
        } else if plugin.id() == "files" {
            data.get("files")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|f| f.get("path").and_then(|p| p.as_str()))
                        .collect::<Vec<_>>()
                        .join("\n")
                })
        } else {
            None
        };

        previews.push(PreviewFormat {
            plugin_id: plugin.id().to_string(),
            template_name: plugin.preview_template_name(),
            priority: plugin.get_preview_priority(),
            data,
            text,
        });
    }

    previews.sort_by_key(|p| p.priority);
    Ok(previews)
}

pub fn prepare_import(item: &ClipboardJsonFullItem) -> Result<ClipboardJsonImport> {
    if item.formats.is_empty() {
        bail!("clipboard item includes no formats");
    }

    let mut captures = Vec::new();
    let mut clipboard_contents = Vec::new();

    for format in &item.formats {
        let plugin = plugin_by_id(&format.plugin_id)
            .ok_or_else(|| anyhow!("Unknown clipboard plugin {}", format.plugin_id))?;
        let import = plugin.import_json(format)?;
        captures.push(import.capture);
        clipboard_contents.extend(import.clipboard_contents);
    }

    Ok(ClipboardJsonImport {
        captures,
        clipboard_contents,
    })
}

pub fn compute_json_item_hash(item: &ClipboardJsonFullItem) -> Result<String> {
    let mut normalized = Vec::new();
    for format in &item.formats {
        let mut entry = Map::new();
        entry.insert("pluginId".into(), Value::String(format.plugin_id.clone()));
        entry.insert("data".into(), format.data.clone());
        if !format.metadata.is_null() {
            entry.insert("metadata".into(), format.metadata.clone());
        }
        normalized.push(Value::Object(entry));
    }
    let bytes = serde_json::to_vec(&normalized)?;
    Ok(sha256_bytes(&bytes))
}

pub fn extract_plugin_meta(
    metadata: &EntryMetadata,
) -> Result<Option<(Vec<String>, Map<String, Value>)>> {
    let root = match metadata.extra.as_object() {
        Some(root) => root,
        None => return Ok(None),
    };
    let plugins = match root.get("plugins").and_then(Value::as_object) {
        Some(map) => map.clone(),
        None => return Ok(None),
    };
    let order = root
        .get("pluginOrder")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        })
        .unwrap_or_else(|| plugins.keys().cloned().collect());
    Ok(Some((order, plugins)))
}

struct PluginInstance<'a> {
    plugin: &'static dyn ClipboardPlugin,
    metadata: &'a EntryMetadata,
    plugin_meta: &'a Value,
    item_dir: &'a Path,
    stored_files: Vec<StoredFile>,
}

impl<'a> PluginInstance<'a> {
    fn new(
        plugin: &'static dyn ClipboardPlugin,
        metadata: &'a EntryMetadata,
        item_dir: &'a Path,
        plugin_meta: &'a Value,
    ) -> Result<Self> {
        let stored_files = load_plugin_files(item_dir, plugin_meta)?;
        Ok(Self {
            plugin,
            metadata,
            plugin_meta,
            item_dir,
            stored_files,
        })
    }

    fn context(&'a self) -> PluginContext<'a> {
        PluginContext {
            metadata: self.metadata,
            plugin_meta: self.plugin_meta,
            item_dir: self.item_dir,
            stored_files: &self.stored_files,
        }
    }
}

fn load_plugin_files(item_dir: &Path, plugin_meta: &Value) -> Result<Vec<StoredFile>> {
    let stored_files = match plugin_meta.get("storedFiles") {
        Some(Value::Array(array)) => array
            .iter()
            .filter_map(Value::as_str)
            .map(|name| {
                let path = item_dir.join(name);
                Ok(StoredFile {
                    filename: name.to_string(),
                    path,
                })
            })
            .collect::<Result<Vec<_>>>()?,
        _ => Vec::new(),
    };
    Ok(stored_files)
}
