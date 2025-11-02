mod files;
mod html;
mod image;
mod rtf;
mod text;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use clipboard_rs::common::{ClipboardContent, RustImage, RustImageData};
use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::clipboard::snapshot::{ClipboardSnapshot, FileOutput};
use crate::data::model::EntryMetadata;

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
    fn detail_log(&self, ctx: &PluginContext<'_>) -> Result<Vec<(String, String)>>;
    fn searchable_text(
        &self,
        _snapshot: &ClipboardSnapshot,
        capture: &PluginCapture,
    ) -> Option<String> {
        capture.summary.clone()
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
    pub id: String,
    pub date: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub size: u64,
    pub dataPath: String,
    pub data: Value,
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
    if let Some(contents) = rebuild_with_plugins(metadata, item_dir)? {
        if contents.is_empty() {
            return rebuild_legacy_clipboard_contents(metadata, item_dir);
        }
        return Ok(contents);
    }
    rebuild_legacy_clipboard_contents(metadata, item_dir)
}

fn rebuild_with_plugins(
    metadata: &EntryMetadata,
    item_dir: &Path,
) -> Result<Option<Vec<ClipboardContent>>> {
    if let Some((order, map)) = extract_plugin_meta(metadata)? {
        return apply_plugins(order, map, metadata, item_dir, |instance| {
            instance.plugin.to_clipboard_items(&instance.context())
        });
    }
    Ok(None)
}

fn apply_plugins<F, T>(
    order: Vec<String>,
    plugin_map: Map<String, Value>,
    metadata: &EntryMetadata,
    item_dir: &Path,
    mut f: F,
) -> Result<Option<Vec<T>>>
where
    F: FnMut(&PluginInstance<'_>) -> Result<Vec<T>>,
{
    let mut results = Vec::new();
    for plugin_id in order {
        let plugin_meta = match plugin_map.get(&plugin_id) {
            Some(value) => value,
            None => continue,
        };
        let plugin = match plugin_by_id(&plugin_id) {
            Some(plugin) => plugin,
            None => continue,
        };
        let instance = PluginInstance::new(plugin, metadata, item_dir, plugin_meta)?;
        results.extend(f(&instance)?);
    }
    Ok(Some(results))
}

pub fn build_display_content(metadata: &EntryMetadata, item_dir: &Path) -> Result<DisplayContent> {
    build_display_content_with_preference(metadata, item_dir, None)
}

pub fn build_display_content_with_preference(
    metadata: &EntryMetadata,
    item_dir: &Path,
    preferred: Option<&str>,
) -> Result<DisplayContent> {
    if let Some(order_map) = extract_plugin_meta(metadata)? {
        let (order, map) = order_map;
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
    }
    legacy_display(metadata, item_dir)
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
        id: metadata.hash.clone(),
        date: crate::util::time::format_iso(metadata.last_seen),
        item_type: plugin.kind().to_string(),
        size: bytes_to_kb(metadata.byte_size),
        dataPath: item_path.to_string_lossy().to_string(),
        data,
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
    build_json_item_with_preference(metadata, item_dir, index, None)
}

pub fn build_json_item_with_preference(
    metadata: &EntryMetadata,
    item_dir: &Path,
    index: usize,
    preferred: Option<&str>,
) -> Result<ClipboardJsonItem> {
    let item_path = item_dir
        .canonicalize()
        .unwrap_or_else(|_| item_dir.to_path_buf());
    if let Some(order_map) = extract_plugin_meta(metadata)? {
        let (order, map) = order_map;
        if let Some(plugin_id) = preferred {
            if let Some(item) =
                json_with_plugin(metadata, item_dir, &item_path, &map, plugin_id, index)?
            {
                return Ok(item);
            }
        }
        for plugin_id in order {
            if let Some(item) =
                json_with_plugin(metadata, item_dir, &item_path, &map, &plugin_id, index)?
            {
                return Ok(item);
            }
        }
    }
    build_legacy_json_item(metadata, &item_path, index)
}

fn extract_plugin_meta(
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

fn bytes_to_kb(bytes: u64) -> u64 {
    if bytes == 0 {
        0
    } else {
        ((bytes as f64) / 1024.0).ceil() as u64
    }
}

fn legacy_display(metadata: &EntryMetadata, item_dir: &Path) -> Result<DisplayContent> {
    let contents = rebuild_legacy_clipboard_contents(metadata, item_dir)?;
    if contents.is_empty() {
        return Ok(DisplayContent::Empty);
    }
    match &contents[0] {
        ClipboardContent::Text(text) => Ok(DisplayContent::Text(text.clone())),
        ClipboardContent::Html(html) => Ok(DisplayContent::Text(html.clone())),
        ClipboardContent::Rtf(rtf) => Ok(DisplayContent::Text(rtf.clone())),
        ClipboardContent::Files(files) => Ok(DisplayContent::Lines(files.clone())),
        ClipboardContent::Image(_) => Ok(DisplayContent::Empty),
        ClipboardContent::Other(_, _) => Ok(DisplayContent::Empty),
    }
}

fn build_legacy_json_item(
    metadata: &EntryMetadata,
    item_path: &Path,
    index: usize,
) -> Result<ClipboardJsonItem> {
    Ok(ClipboardJsonItem {
        index,
        id: metadata.hash.clone(),
        date: crate::util::time::format_iso(metadata.last_seen),
        item_type: format!("{:?}", metadata.kind),
        size: bytes_to_kb(metadata.byte_size),
        dataPath: item_path.to_string_lossy().to_string(),
        data: Value::Null,
    })
}

fn rebuild_legacy_clipboard_contents(
    metadata: &EntryMetadata,
    item_dir: &Path,
) -> Result<Vec<ClipboardContent>> {
    if let Some(legacy_type) = metadata.extra.get("type").and_then(Value::as_str) {
        match legacy_type {
            "text" => {
                let text_path = item_dir.join(&metadata.content_filename);
                let content = fs::read_to_string(&text_path)
                    .with_context(|| format!("Failed to read {}", text_path.display()))?;
                let mut contents = vec![ClipboardContent::Text(content.clone())];
                let is_html = metadata
                    .extra
                    .get("extension")
                    .and_then(Value::as_str)
                    .unwrap_or("txt")
                    == "html";
                if is_html {
                    contents.push(ClipboardContent::Html(content));
                }
                return Ok(contents);
            }
            "image" => {
                let image_path = item_dir.join(&metadata.content_filename);
                let image_data = RustImageData::from_path(image_path.to_string_lossy().as_ref())
                    .map_err(|e| anyhow!("Failed to load image: {e}"))?;
                return Ok(vec![ClipboardContent::Image(image_data)]);
            }
            "file" => {
                let urls: Vec<String> = if !metadata.files.is_empty() {
                    metadata
                        .files
                        .iter()
                        .map(|path| format!("file://{}", path))
                        .collect()
                } else if let Some(entries) =
                    metadata.extra.get("entries").and_then(Value::as_array)
                {
                    entries
                        .iter()
                        .filter_map(|entry| {
                            entry
                                .get("path")
                                .or_else(|| entry.get("source_path"))
                                .and_then(Value::as_str)
                                .map(|path| format!("file://{}", path))
                        })
                        .collect()
                } else {
                    Vec::new()
                };

                if urls.is_empty() {
                    return Err(anyhow!("Legacy file item missing recorded paths"));
                }

                return Ok(vec![ClipboardContent::Files(urls)]);
            }
            _ => {}
        }
    }

    let bytes = fs::read(item_dir.join(&metadata.content_filename))
        .with_context(|| format!("Failed to read {}", metadata.content_filename))?;
    let mime = metadata
        .extra
        .get("mime")
        .and_then(Value::as_str)
        .unwrap_or("application/octet-stream");

    if mime == "text/rtf" || mime == "application/rtf" {
        let text = String::from_utf8_lossy(&bytes).into_owned();
        return Ok(vec![ClipboardContent::Rtf(text)]);
    }

    Ok(vec![ClipboardContent::Other(
        coerce_mime_to_uti(mime),
        bytes,
    )])
}

fn coerce_mime_to_uti(mime: &str) -> String {
    if mime.is_empty() {
        return "public.data".into();
    }
    if !mime.contains('/') {
        return mime.to_string();
    }
    match mime {
        "text/plain" | "text/utf-8" | "text/x-diff" => "public.utf8-plain-text".into(),
        "text/html" => "public.html".into(),
        "text/rtf" | "application/rtf" => "public.rtf".into(),
        "application/json" => "public.json".into(),
        "application/pdf" => "com.adobe.pdf".into(),
        "application/octet-stream" => "public.data".into(),
        "application/zip" => "com.pkware.zip-archive".into(),
        "image/png" => "public.png".into(),
        "image/jpeg" | "image/jpg" => "public.jpeg".into(),
        "image/gif" => "com.compuserve.gif".into(),
        "image/tiff" => "public.tiff".into(),
        _ => "public.data".into(),
    }
}
