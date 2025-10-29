use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use clipboard_rs::common::{ClipboardContent, RustImage, RustImageData};
use image::GenericImageView;
use serde_json::{Map, Value, json};

use crate::clipboard::snapshot::{
    ClipboardSnapshot, FileOutput, format_file_summary, human_kb, mime_for_extension,
    truncate_summary,
};
use crate::data::model::EntryMetadata;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginType {
    File,
    Image,
    Text,
    Html,
    Rtf,
}

impl PluginType {
    fn priority_value(&self) -> u8 {
        match self {
            PluginType::File => 0,
            PluginType::Image => 1,
            PluginType::Text => 2,
            PluginType::Html => 3,
            PluginType::Rtf => 4,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            PluginType::File => "file",
            PluginType::Image => "image",
            PluginType::Text => "text",
            PluginType::Html => "html",
            PluginType::Rtf => "rtf",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PluginCapture {
    pub plugin_id: &'static str,
    pub plugin_type: PluginType,
    pub summary: Option<String>,
    pub files: Vec<FileOutput>,
    pub metadata: Value,
    pub byte_size: u64,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone)]
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

pub trait ClipboardPlugin: Sync + Send {
    fn id(&self) -> &'static str;
    fn plugin_type(&self) -> PluginType;
    fn does_match(&self, snapshot: &ClipboardSnapshot) -> bool;
    fn capture(&self, snapshot: &ClipboardSnapshot) -> Option<PluginCapture>;
    fn to_clipboard_items(
        &self,
        plugin_meta: &Value,
        files: &[StoredFile],
    ) -> Result<Vec<ClipboardContent>>;
}

pub fn capture_plugins(snapshot: &ClipboardSnapshot) -> Vec<PluginCapture> {
    let mut captures = Vec::new();
    for plugin in plugin_registry() {
        if plugin.does_match(snapshot) {
            if let Some(mut capture) = plugin.capture(snapshot) {
                finalize_capture_metadata(&mut capture);
                captures.push(capture);
            }
        }
    }
    captures
}

pub fn rebuild_clipboard_contents(
    metadata: &EntryMetadata,
    item_dir: &Path,
) -> Result<Vec<ClipboardContent>> {
    let extra = &metadata.extra;
    let root = match extra.as_object() {
        Some(root) => root,
        None => return rebuild_legacy_clipboard_contents(metadata, item_dir),
    };

    let plugins_value = match root.get("plugins").and_then(Value::as_object) {
        Some(plugins) => plugins,
        None => return rebuild_legacy_clipboard_contents(metadata, item_dir),
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
        .unwrap_or_else(|| plugins_value.keys().cloned().collect::<Vec<String>>());

    let mut contents = Vec::new();
    for plugin_id in order {
        let plugin_meta = match plugins_value.get(&plugin_id) {
            Some(value) => value,
            None => continue,
        };
        let plugin = match plugin_by_id(&plugin_id) {
            Some(plugin) => plugin,
            None => continue,
        };
        let files = load_plugin_files(item_dir, plugin_meta)?;
        contents.extend(plugin.to_clipboard_items(plugin_meta, &files)?);
    }

    if contents.is_empty() {
        rebuild_legacy_clipboard_contents(metadata, item_dir)
    } else {
        Ok(contents)
    }
}

pub fn prioritized_capture<'a>(captures: &'a [PluginCapture]) -> Option<&'a PluginCapture> {
    captures
        .iter()
        .min_by_key(|capture| capture.plugin_type.priority_value())
}

pub fn plugin_order(captures: &[PluginCapture]) -> Vec<String> {
    captures
        .iter()
        .map(|capture| capture.plugin_id.to_string())
        .collect()
}

fn finalize_capture_metadata(capture: &mut PluginCapture) {
    let mut meta = match &capture.metadata {
        Value::Object(map) => map.clone(),
        _ => Map::new(),
    };

    let stored_files: Vec<Value> = capture
        .files
        .iter()
        .map(|file| Value::String(file.filename.clone()))
        .collect();

    if !stored_files.is_empty() {
        meta.insert("storedFiles".into(), Value::Array(stored_files));
    }

    meta.insert(
        "pluginId".into(),
        Value::String(capture.plugin_id.to_string()),
    );
    meta.insert(
        "pluginType".into(),
        Value::String(capture.plugin_type.as_str().to_string()),
    );

    if let Some(summary) = &capture.summary {
        meta.insert("summary".into(), Value::String(summary.clone()));
    }

    capture.metadata = Value::Object(meta);
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

fn plugin_registry() -> Vec<Box<dyn ClipboardPlugin>> {
    vec![
        Box::new(FilesPlugin),
        Box::new(ImagePlugin),
        Box::new(TextPlugin),
        Box::new(HtmlPlugin),
        Box::new(RtfPlugin),
    ]
}

fn plugin_by_id(id: &str) -> Option<Box<dyn ClipboardPlugin>> {
    match id {
        "files" => Some(Box::new(FilesPlugin)),
        "image" => Some(Box::new(ImagePlugin)),
        "text" => Some(Box::new(TextPlugin)),
        "html" => Some(Box::new(HtmlPlugin)),
        "rtf" => Some(Box::new(RtfPlugin)),
        _ => None,
    }
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

struct TextPlugin;

impl ClipboardPlugin for TextPlugin {
    fn id(&self) -> &'static str {
        "text"
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Text
    }

    fn does_match(&self, snapshot: &ClipboardSnapshot) -> bool {
        snapshot
            .text
            .as_ref()
            .map(|text| !text.is_empty())
            .unwrap_or(false)
    }

    fn capture(&self, snapshot: &ClipboardSnapshot) -> Option<PluginCapture> {
        let text = snapshot.text.as_ref()?;
        if text.is_empty() {
            return None;
        }

        let file_name = "text__content.txt";
        let summary = Some(truncate_summary(text));
        let files = vec![FileOutput {
            filename: file_name.to_string(),
            bytes: text.clone().into_bytes(),
        }];

        let metadata = json!({
            "length": text.chars().count(),
        });

        Some(PluginCapture {
            plugin_id: self.id(),
            plugin_type: self.plugin_type(),
            summary,
            files,
            metadata,
            byte_size: text.len() as u64,
            sources: Vec::new(),
        })
    }

    fn to_clipboard_items(
        &self,
        plugin_meta: &Value,
        files: &[StoredFile],
    ) -> Result<Vec<ClipboardContent>> {
        let file_name = plugin_meta
            .get("storedFiles")
            .and_then(Value::as_array)
            .and_then(|arr| arr.iter().filter_map(Value::as_str).next())
            .or_else(|| files.first().map(|file| file.filename.as_str()))
            .ok_or_else(|| anyhow!("text plugin missing stored file information"))?;

        let file = files
            .iter()
            .find(|stored| stored.filename == file_name)
            .ok_or_else(|| anyhow!("text plugin stored file not found"))?;

        let text = file.read_string()?;
        Ok(vec![ClipboardContent::Text(text)])
    }
}

struct HtmlPlugin;

impl ClipboardPlugin for HtmlPlugin {
    fn id(&self) -> &'static str {
        "html"
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Html
    }

    fn does_match(&self, snapshot: &ClipboardSnapshot) -> bool {
        snapshot
            .html
            .as_ref()
            .map(|html| !html.is_empty())
            .unwrap_or(false)
    }

    fn capture(&self, snapshot: &ClipboardSnapshot) -> Option<PluginCapture> {
        let html = snapshot.html.as_ref()?;
        if html.is_empty() {
            return None;
        }

        let file_name = "html__content.html";
        let files = vec![FileOutput {
            filename: file_name.to_string(),
            bytes: html.clone().into_bytes(),
        }];

        let metadata = json!({
            "length": html.chars().count(),
        });

        Some(PluginCapture {
            plugin_id: self.id(),
            plugin_type: self.plugin_type(),
            summary: None,
            files,
            metadata,
            byte_size: html.len() as u64,
            sources: Vec::new(),
        })
    }

    fn to_clipboard_items(
        &self,
        plugin_meta: &Value,
        files: &[StoredFile],
    ) -> Result<Vec<ClipboardContent>> {
        let file_name = plugin_meta
            .get("storedFiles")
            .and_then(Value::as_array)
            .and_then(|arr| arr.iter().filter_map(Value::as_str).next())
            .or_else(|| files.first().map(|file| file.filename.as_str()))
            .ok_or_else(|| anyhow!("html plugin missing stored file information"))?;

        let file = files
            .iter()
            .find(|stored| stored.filename == file_name)
            .ok_or_else(|| anyhow!("html plugin stored file not found"))?;

        let html = file.read_string()?;
        Ok(vec![ClipboardContent::Html(html)])
    }
}

struct RtfPlugin;

impl ClipboardPlugin for RtfPlugin {
    fn id(&self) -> &'static str {
        "rtf"
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Rtf
    }

    fn does_match(&self, snapshot: &ClipboardSnapshot) -> bool {
        snapshot
            .rtf
            .as_ref()
            .map(|rtf| !rtf.is_empty())
            .unwrap_or(false)
    }

    fn capture(&self, snapshot: &ClipboardSnapshot) -> Option<PluginCapture> {
        let rtf = snapshot.rtf.as_ref()?;
        if rtf.is_empty() {
            return None;
        }

        let file_name = "rtf__content.rtf";
        let files = vec![FileOutput {
            filename: file_name.to_string(),
            bytes: rtf.clone(),
        }];

        let metadata = json!({
            "byteSize": rtf.len(),
        });

        Some(PluginCapture {
            plugin_id: self.id(),
            plugin_type: self.plugin_type(),
            summary: None,
            files,
            metadata,
            byte_size: rtf.len() as u64,
            sources: Vec::new(),
        })
    }

    fn to_clipboard_items(
        &self,
        plugin_meta: &Value,
        files: &[StoredFile],
    ) -> Result<Vec<ClipboardContent>> {
        let file_name = plugin_meta
            .get("storedFiles")
            .and_then(Value::as_array)
            .and_then(|arr| arr.iter().filter_map(Value::as_str).next())
            .or_else(|| files.first().map(|file| file.filename.as_str()))
            .ok_or_else(|| anyhow!("rtf plugin missing stored file information"))?;

        let file = files
            .iter()
            .find(|stored| stored.filename == file_name)
            .ok_or_else(|| anyhow!("rtf plugin stored file not found"))?;

        let bytes = file.read_bytes()?;
        let rtf = String::from_utf8_lossy(&bytes).into_owned();
        Ok(vec![ClipboardContent::Rtf(rtf)])
    }
}

struct ImagePlugin;

impl ClipboardPlugin for ImagePlugin {
    fn id(&self) -> &'static str {
        "image"
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Image
    }

    fn does_match(&self, snapshot: &ClipboardSnapshot) -> bool {
        snapshot
            .image_bytes
            .as_ref()
            .map(|bytes| !bytes.is_empty())
            .unwrap_or(false)
    }

    fn capture(&self, snapshot: &ClipboardSnapshot) -> Option<PluginCapture> {
        let bytes = snapshot.image_bytes.as_ref()?.clone();
        if bytes.is_empty() {
            return None;
        }

        let reader = ImagePlugin::decode_image(&bytes).ok()?;
        let mime = snapshot
            .image_mime
            .clone()
            .or_else(|| mime_for_extension("png"))
            .unwrap_or_else(|| "image/png".into());

        let (width, height) = reader.dimensions();
        let summary = Some(format!(
            "Image {}x{} [{} - {}]",
            width,
            height,
            human_kb(bytes.len() as u64),
            mime
        ));

        let files = vec![FileOutput {
            filename: "image__full.png".to_string(),
            bytes: bytes.clone(),
        }];

        let metadata = json!({
            "width": width,
            "height": height,
            "mime": mime,
            "byteSize": bytes.len(),
        });

        Some(PluginCapture {
            plugin_id: self.id(),
            plugin_type: self.plugin_type(),
            summary,
            files,
            metadata,
            byte_size: bytes.len() as u64,
            sources: Vec::new(),
        })
    }

    fn to_clipboard_items(
        &self,
        plugin_meta: &Value,
        files: &[StoredFile],
    ) -> Result<Vec<ClipboardContent>> {
        let file_name = plugin_meta
            .get("storedFiles")
            .and_then(Value::as_array)
            .and_then(|arr| arr.iter().filter_map(Value::as_str).next())
            .or_else(|| files.first().map(|file| file.filename.as_str()))
            .ok_or_else(|| anyhow!("image plugin missing stored file information"))?;

        let file = files
            .iter()
            .find(|stored| stored.filename == file_name)
            .ok_or_else(|| anyhow!("image plugin stored file not found"))?;

        let image_data = RustImageData::from_path(file.path.to_string_lossy().as_ref())
            .map_err(|e| anyhow!("Failed to load stored image: {e}"))?;

        Ok(vec![ClipboardContent::Image(image_data)])
    }
}

impl ImagePlugin {
    fn decode_image(bytes: &[u8]) -> Result<image::DynamicImage> {
        image::ImageReader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()
            .context("Unable to read image data")?
            .decode()
            .context("Unable to decode image")
    }
}

struct FilesPlugin;

impl ClipboardPlugin for FilesPlugin {
    fn id(&self) -> &'static str {
        "files"
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::File
    }

    fn does_match(&self, snapshot: &ClipboardSnapshot) -> bool {
        !snapshot.files.is_empty()
    }

    fn capture(&self, snapshot: &ClipboardSnapshot) -> Option<PluginCapture> {
        if snapshot.files.is_empty() {
            return None;
        }

        let summary = Some(format_file_summary(&snapshot.files));
        let lines: Vec<String> = snapshot
            .files
            .iter()
            .map(|record| {
                let mime = record.mime.clone().unwrap_or_else(|| "file".into());
                format!(
                    "{} ({} - {})",
                    record.source_path.display(),
                    human_kb(record.size),
                    mime
                )
            })
            .collect();

        let files = vec![FileOutput {
            filename: "files__paths.txt".to_string(),
            bytes: lines.join("\n").into_bytes(),
        }];

        let metadata = json!({
            "entries": snapshot.files.clone(),
        });

        let byte_size = snapshot.files.iter().map(|record| record.size).sum();
        let sources = snapshot.sources();

        Some(PluginCapture {
            plugin_id: self.id(),
            plugin_type: self.plugin_type(),
            summary,
            files,
            metadata,
            byte_size,
            sources,
        })
    }

    fn to_clipboard_items(
        &self,
        plugin_meta: &Value,
        _files: &[StoredFile],
    ) -> Result<Vec<ClipboardContent>> {
        let entries = plugin_meta
            .get("entries")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("file plugin metadata missing entries"))?;

        let mut urls = Vec::new();
        for entry in entries {
            let path = entry
                .get("source_path")
                .or_else(|| entry.get("path"))
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("file entry missing path"))?;
            urls.push(format!("file://{}", path));
        }

        if urls.is_empty() {
            return Err(anyhow!("file plugin requires at least one path"));
        }

        Ok(vec![ClipboardContent::Files(urls)])
    }
}
