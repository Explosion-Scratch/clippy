use crate::clipboard::mac;
use crate::data::model::{EntryKind, EntryMetadata};
use crate::util::hash::sha256_bytes;
use anyhow::{Context, Result, anyhow};
use clipboard_rs::common::{ClipboardContent, RustImage, RustImageData};
use image::{GenericImageView, ImageFormat, ImageReader};
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardType};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileOutput {
    pub filename: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub name: String,
    pub extension: Option<String>,
    pub size: u64,
    pub source_path: PathBuf,
    pub mime: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardSnapshot {
    pub kind: EntryKind,
    pub text: Option<String>,
    pub html: Option<String>,
    pub rtf: Option<Vec<u8>>,
    pub image_bytes: Option<Vec<u8>>,
    pub image_mime: Option<String>,
    pub files: Vec<FileRecord>,
    pub summary: Option<String>,
    pub detected_formats: Vec<String>,
    pub extra: Value,
}

impl ClipboardSnapshot {
    pub fn from_pasteboard(pasteboard: &NSPasteboard) -> Result<Option<Self>> {
        let items = pasteboard
            .pasteboardItems()
            .map(|arr| arr.to_vec())
            .unwrap_or_default();
        let item = match items.into_iter().next() {
            Some(value) => value,
            None => return Ok(None),
        };

        let mut detected = Vec::new();
        let mut text = None;
        let mut html = None;
        let mut rtf = None;
        let mut image_bytes = None;
        let mut image_mime = None;
        let mut files: Vec<FileRecord> = Vec::new();

        for ty in item.types().iter() {
            let ty_string = ty.to_string();
            if !detected.contains(&ty_string) {
                detected.push(ty_string.clone());
            }
            match ty_string.as_str() {
                s if s.contains("public.utf8-plain-text") || s.contains("public.text") => {
                    if let Some(value) = read_string(&item, ty.as_ref()) {
                        text = Some(value);
                    }
                }
                s if s.contains("public.file-url") => {
                    if let Some(raw_urls) = read_string(&item, ty.as_ref()) {
                        for path in mac::parse_file_urls(&raw_urls) {
                            process_file_path(&path, &mut files);
                        }
                    }
                }
                s if s.contains("public.png") => {
                    if let Some(bytes) = read_data(&item, ty.as_ref()) {
                        image_bytes = Some(bytes);
                        image_mime = Some("image/png".to_string());
                    }
                }
                s if s.contains("public.tiff") => {
                    if let Some(bytes) = read_data(&item, ty.as_ref()) {
                        image_bytes = Some(bytes);
                        image_mime = Some("image/tiff".to_string());
                    }
                }
                s if s.contains("text/html") => {
                    if let Some(content) = read_string(&item, ty.as_ref()) {
                        html = Some(content);
                    }
                }
                s if s.contains("public.rtf") => {
                    if let Some(bytes) = read_data(&item, ty.as_ref()) {
                        rtf = Some(bytes);
                    }
                }
                _ => {}
            }
        }

        if files.is_empty()
            && text.is_none()
            && html.is_none()
            && image_bytes.is_none()
            && rtf.is_none()
        {
            return Ok(None);
        }

        files.sort_by(|a, b| a.source_path.cmp(&b.source_path));
        let kind = if !files.is_empty() {
            EntryKind::File
        } else if image_bytes.is_some() {
            EntryKind::Image
        } else if text.is_some() || html.is_some() {
            EntryKind::Text
        } else {
            EntryKind::Other
        };

        let summary = match kind {
            EntryKind::File => Some(format_file_summary(&files)),
            EntryKind::Image => {
                let mime = image_mime.clone().unwrap_or_else(|| "image".into());
                let size = image_bytes
                    .as_ref()
                    .map(|b| human_kb(b.len() as u64))
                    .unwrap_or_else(|| "? KB".into());
                Some(format!("Image [{} - {}]", size, mime))
            }
            EntryKind::Text => {
                let content = text.clone().or(html.clone()).unwrap_or_default();
                Some(truncate_summary(&content))
            }
            EntryKind::Other => Some("(binary item)".into()),
        };

        Ok(Some(Self {
            kind,
            text,
            html,
            rtf,
            image_bytes,
            image_mime,
            files,
            summary,
            detected_formats: detected,
            extra: Value::Null,
        }))
    }

    pub fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        if let Some(text) = &self.text {
            hasher.update(text.as_bytes());
        }
        if let Some(html) = &self.html {
            hasher.update(html.as_bytes());
        }
        if let Some(rtf) = &self.rtf {
            hasher.update(rtf);
        }
        if let Some(bytes) = &self.image_bytes {
            hasher.update(bytes);
        }
        for record in &self.files {
            hasher.update(record.source_path.to_string_lossy().as_bytes());
            hasher.update(record.size.to_le_bytes());
            if let Some(mime) = &record.mime {
                hasher.update(mime.as_bytes());
            }
        }
        sha256_bytes(&hasher.finalize())
    }

    pub fn classify(&self) -> Result<SnapshotHandler> {
        if !self.files.is_empty() {
            return FileHandler::from_snapshot(self).map(SnapshotHandler::File);
        }
        if let Some(bytes) = &self.image_bytes {
            return ImageHandler::from_snapshot(self, bytes.clone()).map(SnapshotHandler::Image);
        }
        if self.text.is_some() || self.html.is_some() {
            return TextHandler::from_snapshot(self).map(SnapshotHandler::Text);
        }
        OtherHandler::from_snapshot(self).map(SnapshotHandler::Other)
    }

    pub fn sources(&self) -> Vec<String> {
        self.files
            .iter()
            .map(|f| f.source_path.display().to_string())
            .collect()
    }

    pub fn total_size(&self) -> u64 {
        let mut total = 0u64;
        if let Some(text) = &self.text {
            total += text.len() as u64;
        }
        if let Some(html) = &self.html {
            total += html.len() as u64;
        }
        if let Some(rtf) = &self.rtf {
            total += rtf.len() as u64;
        }
        if let Some(bytes) = &self.image_bytes {
            total += bytes.len() as u64;
        }
        total += self.files.iter().map(|f| f.size).sum::<u64>();
        total
    }
}

pub enum SnapshotHandler {
    Text(TextHandler),
    Image(ImageHandler),
    File(FileHandler),
    Other(OtherHandler),
}

pub trait ClipboardClass {
    fn to_string(&self) -> String;
    fn to_files(&self) -> Result<Vec<FileOutput>>;
    fn to_metadata(&self) -> Value;
    fn to_clipboard_item(
        &self,
        metadata: &EntryMetadata,
        base_dir: &Path,
    ) -> Result<Vec<ClipboardContent>>;
    fn detected_formats(&self) -> &[String];
    fn sources(&self) -> Vec<String>;
    fn extra_files(&self) -> Vec<FileOutput> {
        Vec::new()
    }
}

impl ClipboardClass for SnapshotHandler {
    fn to_string(&self) -> String {
        match self {
            SnapshotHandler::Text(handler) => handler.to_string(),
            SnapshotHandler::Image(handler) => handler.to_string(),
            SnapshotHandler::File(handler) => handler.to_string(),
            SnapshotHandler::Other(handler) => handler.to_string(),
        }
    }

    fn to_files(&self) -> Result<Vec<FileOutput>> {
        match self {
            SnapshotHandler::Text(handler) => handler.to_files(),
            SnapshotHandler::Image(handler) => handler.to_files(),
            SnapshotHandler::File(handler) => handler.to_files(),
            SnapshotHandler::Other(handler) => handler.to_files(),
        }
    }

    fn to_metadata(&self) -> Value {
        match self {
            SnapshotHandler::Text(handler) => handler.to_metadata(),
            SnapshotHandler::Image(handler) => handler.to_metadata(),
            SnapshotHandler::File(handler) => handler.to_metadata(),
            SnapshotHandler::Other(handler) => handler.to_metadata(),
        }
    }

    fn to_clipboard_item(
        &self,
        metadata: &EntryMetadata,
        base_dir: &Path,
    ) -> Result<Vec<ClipboardContent>> {
        match self {
            SnapshotHandler::Text(handler) => handler.to_clipboard_item(metadata, base_dir),
            SnapshotHandler::Image(handler) => handler.to_clipboard_item(metadata, base_dir),
            SnapshotHandler::File(handler) => handler.to_clipboard_item(metadata, base_dir),
            SnapshotHandler::Other(handler) => handler.to_clipboard_item(metadata, base_dir),
        }
    }

    fn detected_formats(&self) -> &[String] {
        match self {
            SnapshotHandler::Text(handler) => handler.detected_formats(),
            SnapshotHandler::Image(handler) => handler.detected_formats(),
            SnapshotHandler::File(handler) => handler.detected_formats(),
            SnapshotHandler::Other(handler) => handler.detected_formats(),
        }
    }

    fn sources(&self) -> Vec<String> {
        match self {
            SnapshotHandler::Text(handler) => handler.sources(),
            SnapshotHandler::Image(handler) => handler.sources(),
            SnapshotHandler::File(handler) => handler.sources(),
            SnapshotHandler::Other(handler) => handler.sources(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextHandler {
    content: String,
    is_html: bool,
    detected_formats: Vec<String>,
}

impl TextHandler {
    fn from_snapshot(snapshot: &ClipboardSnapshot) -> Result<Self> {
        if let Some(text) = &snapshot.text {
            Ok(Self {
                content: text.clone(),
                is_html: false,
                detected_formats: snapshot.detected_formats.clone(),
            })
        } else if let Some(html) = &snapshot.html {
            Ok(Self {
                content: html.clone(),
                is_html: true,
                detected_formats: snapshot.detected_formats.clone(),
            })
        } else {
            Err(anyhow!("No textual content available"))
        }
    }

    fn extension(&self) -> &str {
        if self.is_html { "html" } else { "txt" }
    }

    fn detected_formats(&self) -> &[String] {
        &self.detected_formats
    }

    fn sources(&self) -> Vec<String> {
        Vec::new()
    }
}

impl ClipboardClass for TextHandler {
    fn to_string(&self) -> String {
        truncate_summary(&self.content)
    }

    fn to_files(&self) -> Result<Vec<FileOutput>> {
        Ok(vec![FileOutput {
            filename: format!("item.{}", self.extension()),
            bytes: self.content.clone().into_bytes(),
        }])
    }

    fn to_metadata(&self) -> Value {
        json!({
            "type": "text",
            "extension": self.extension(),
            "length": self.content.chars().count(),
        })
    }

    fn to_clipboard_item(
        &self,
        _metadata: &EntryMetadata,
        _base_dir: &Path,
    ) -> Result<Vec<ClipboardContent>> {
        let mut contents = Vec::new();
        contents.push(ClipboardContent::Text(self.content.clone()));
        if self.is_html {
            contents.push(ClipboardContent::Html(self.content.clone()));
        }
        Ok(contents)
    }

    fn detected_formats(&self) -> &[String] {
        TextHandler::detected_formats(self)
    }

    fn sources(&self) -> Vec<String> {
        TextHandler::sources(self)
    }
}

#[derive(Debug, Clone)]
pub struct ImageHandler {
    bytes: Vec<u8>,
    mime: String,
    extension: String,
    width: u32,
    height: u32,
    detected_formats: Vec<String>,
}

impl ImageHandler {
    fn from_snapshot(snapshot: &ClipboardSnapshot, bytes: Vec<u8>) -> Result<Self> {
        let format = image::guess_format(&bytes).unwrap_or(ImageFormat::Png);
        let extension = match format {
            ImageFormat::Png => "png".to_string(),
            ImageFormat::Jpeg => "jpg".to_string(),
            ImageFormat::Gif => "gif".to_string(),
            ImageFormat::Tiff => "tiff".to_string(),
            _ => "bin".to_string(),
        };
        let mime = snapshot
            .image_mime
            .clone()
            .or_else(|| mime_for_extension(&extension))
            .unwrap_or_else(|| "image/png".into());
        let img = ImageReader::new(std::io::Cursor::new(&bytes))
            .with_guessed_format()
            .context("Unable to read image data")?
            .decode()
            .context("Unable to decode image")?;
        let (width, height) = img.dimensions();
        Ok(Self {
            bytes,
            mime,
            extension,
            width,
            height,
            detected_formats: snapshot.detected_formats.clone(),
        })
    }

    fn detected_formats(&self) -> &[String] {
        &self.detected_formats
    }

    fn sources(&self) -> Vec<String> {
        Vec::new()
    }
}

impl ClipboardClass for ImageHandler {
    fn to_string(&self) -> String {
        format!(
            "Image {}x{} [{} - {}]",
            self.width,
            self.height,
            human_kb(self.bytes.len() as u64),
            self.mime
        )
    }

    fn to_files(&self) -> Result<Vec<FileOutput>> {
        Ok(vec![FileOutput {
            filename: format!("item.{}", self.extension),
            bytes: self.bytes.clone(),
        }])
    }

    fn to_metadata(&self) -> Value {
        json!({
            "type": "image",
            "mime": self.mime,
            "extension": self.extension,
            "width": self.width,
            "height": self.height,
            "byte_size": self.bytes.len(),
        })
    }

    fn to_clipboard_item(
        &self,
        metadata: &EntryMetadata,
        base_dir: &Path,
    ) -> Result<Vec<ClipboardContent>> {
        let item_path = base_dir.join(&metadata.content_filename);
        let temp_path =
            std::env::temp_dir().join(format!("clipboard-{}.{}", metadata.hash, self.extension));
        fs::copy(&item_path, &temp_path).with_context(|| {
            format!(
                "Failed to copy image data to temp file {}",
                temp_path.display()
            )
        })?;
        let image_data = RustImageData::from_path(temp_path.to_string_lossy().as_ref())
            .map_err(|e| anyhow!("Failed to load image for clipboard: {e}"))?;
        fs::remove_file(&temp_path).ok();
        Ok(vec![ClipboardContent::Image(image_data)])
    }

    fn detected_formats(&self) -> &[String] {
        ImageHandler::detected_formats(self)
    }

    fn sources(&self) -> Vec<String> {
        ImageHandler::sources(self)
    }
}

#[derive(Debug, Clone)]
pub struct FileHandler {
    files: Vec<FileRecord>,
    detected_formats: Vec<String>,
}

impl FileHandler {
    fn from_snapshot(snapshot: &ClipboardSnapshot) -> Result<Self> {
        if snapshot.files.is_empty() {
            return Err(anyhow!("No files available"));
        }
        Ok(Self {
            files: snapshot.files.clone(),
            detected_formats: snapshot.detected_formats.clone(),
        })
    }

    fn detected_formats(&self) -> &[String] {
        &self.detected_formats
    }

    fn sources(&self) -> Vec<String> {
        self.files
            .iter()
            .map(|f| f.source_path.display().to_string())
            .collect()
    }
}

impl ClipboardClass for FileHandler {
    fn to_string(&self) -> String {
        format_file_summary(&self.files)
    }

    fn to_files(&self) -> Result<Vec<FileOutput>> {
        let mut lines = Vec::new();
        for record in &self.files {
            let mime = record.mime.clone().unwrap_or_else(|| "file".into());
            lines.push(format!(
                "{} ({} - {})",
                record.source_path.display(),
                human_kb(record.size),
                mime
            ));
        }
        Ok(vec![FileOutput {
            filename: "filepath.txt".into(),
            bytes: lines.join("\n").into_bytes(),
        }])
    }

    fn to_metadata(&self) -> Value {
        json!({
            "type": "file",
            "entries": self.files.iter().map(|f| json!({
                "path": f.source_path,
                "size": f.size,
                "mime": f.mime,
                "name": f.name,
                "extension": f.extension,
            })).collect::<Vec<_>>()
        })
    }

    fn to_clipboard_item(
        &self,
        metadata: &EntryMetadata,
        _base_dir: &Path,
    ) -> Result<Vec<ClipboardContent>> {
        let urls: Vec<String> = metadata
            .files
            .iter()
            .map(|path| format!("file://{}", path))
            .collect();
        if urls.is_empty() {
            return Err(anyhow!("No file paths recorded"));
        }
        Ok(vec![ClipboardContent::Files(urls)])
    }

    fn detected_formats(&self) -> &[String] {
        FileHandler::detected_formats(self)
    }

    fn sources(&self) -> Vec<String> {
        FileHandler::sources(self)
    }
}

#[derive(Debug, Clone)]
pub struct OtherHandler {
    bytes: Vec<u8>,
    mime: String,
    detected_formats: Vec<String>,
}

impl OtherHandler {
    fn from_snapshot(snapshot: &ClipboardSnapshot) -> Result<Self> {
        if let Some(rtf) = &snapshot.rtf {
            return Ok(Self {
                bytes: rtf.clone(),
                mime: "text/rtf".into(),
                detected_formats: snapshot.detected_formats.clone(),
            });
        }
        Err(anyhow!("Unsupported clipboard content"))
    }

    fn detected_formats(&self) -> &[String] {
        &self.detected_formats
    }

    fn sources(&self) -> Vec<String> {
        Vec::new()
    }
}

impl ClipboardClass for OtherHandler {
    fn to_string(&self) -> String {
        format!("Binary item [{}]", human_kb(self.bytes.len() as u64))
    }

    fn to_files(&self) -> Result<Vec<FileOutput>> {
        Ok(vec![FileOutput {
            filename: "item.bin".into(),
            bytes: self.bytes.clone(),
        }])
    }

    fn to_metadata(&self) -> Value {
        json!({
            "type": "other",
            "mime": self.mime,
            "byte_size": self.bytes.len(),
        })
    }

    fn to_clipboard_item(
        &self,
        _metadata: &EntryMetadata,
        _base_dir: &Path,
    ) -> Result<Vec<ClipboardContent>> {
        Ok(vec![ClipboardContent::Other(
            self.mime.clone(),
            self.bytes.clone(),
        )])
    }

    fn detected_formats(&self) -> &[String] {
        OtherHandler::detected_formats(self)
    }

    fn sources(&self) -> Vec<String> {
        OtherHandler::sources(self)
    }
}

pub fn handler_from_metadata(metadata: &EntryMetadata, item_dir: &Path) -> Result<SnapshotHandler> {
    let typ = metadata
        .extra
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("other");
    match typ {
        "text" => {
            let content = fs::read_to_string(item_dir.join(&metadata.content_filename))
                .with_context(|| format!("Failed to read {}", metadata.content_filename))?;
            let is_html = metadata
                .extra
                .get("extension")
                .and_then(Value::as_str)
                .unwrap_or("txt")
                == "html";
            Ok(SnapshotHandler::Text(TextHandler {
                content,
                is_html,
                detected_formats: metadata.detected_formats.clone(),
            }))
        }
        "image" => {
            let bytes = fs::read(item_dir.join(&metadata.content_filename))
                .with_context(|| format!("Failed to read {}", metadata.content_filename))?;
            let handler = ImageHandler {
                bytes,
                mime: metadata
                    .extra
                    .get("mime")
                    .and_then(Value::as_str)
                    .unwrap_or("image/png")
                    .to_string(),
                extension: metadata
                    .extra
                    .get("extension")
                    .and_then(Value::as_str)
                    .unwrap_or("png")
                    .to_string(),
                width: metadata
                    .extra
                    .get("width")
                    .and_then(Value::as_u64)
                    .unwrap_or_default() as u32,
                height: metadata
                    .extra
                    .get("height")
                    .and_then(Value::as_u64)
                    .unwrap_or_default() as u32,
                detected_formats: metadata.detected_formats.clone(),
            };
            Ok(SnapshotHandler::Image(handler))
        }
        "file" => {
            let mut records = Vec::new();
            if let Some(entries) = metadata.extra.get("entries").and_then(Value::as_array) {
                for entry in entries {
                    let path = entry
                        .get("path")
                        .and_then(Value::as_str)
                        .map(PathBuf::from)
                        .unwrap_or_default();
                    let name = entry
                        .get("name")
                        .and_then(Value::as_str)
                        .map(String::from)
                        .unwrap_or_else(|| {
                            path.file_name()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_default()
                        });
                    records.push(FileRecord {
                        name,
                        extension: entry
                            .get("extension")
                            .and_then(Value::as_str)
                            .map(String::from),
                        size: entry
                            .get("size")
                            .and_then(Value::as_u64)
                            .unwrap_or_default(),
                        source_path: path,
                        mime: entry.get("mime").and_then(Value::as_str).map(String::from),
                    });
                }
            }
            Ok(SnapshotHandler::File(FileHandler {
                files: records,
                detected_formats: metadata.detected_formats.clone(),
            }))
        }
        _ => {
            let bytes = fs::read(item_dir.join(&metadata.content_filename))
                .with_context(|| format!("Failed to read {}", metadata.content_filename))?;
            let mime = metadata
                .extra
                .get("mime")
                .and_then(Value::as_str)
                .unwrap_or("application/octet-stream")
                .to_string();
            Ok(SnapshotHandler::Other(OtherHandler {
                bytes,
                mime,
                detected_formats: metadata.detected_formats.clone(),
            }))
        }
    }
}

fn read_string(item: &NSPasteboardItem, ty: &NSPasteboardType) -> Option<String> {
    unsafe { item.stringForType(ty).map(|s| s.to_string()) }
}

fn read_data(item: &NSPasteboardItem, ty: &NSPasteboardType) -> Option<Vec<u8>> {
    unsafe { item.dataForType(ty).map(|d| d.to_vec()) }
}

fn process_file_path(path: &Path, files: &mut Vec<FileRecord>) {
    if is_temporary_file(path) {
        return;
    }
    if let Ok(metadata) = fs::metadata(path) {
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        let extension = path.extension().map(|s| s.to_string_lossy().to_string());
        let mime = mime_guess::from_path(path).first_raw().map(String::from);
        files.push(FileRecord {
            name,
            extension,
            size: metadata.len(),
            source_path: path.to_path_buf(),
            mime,
        });
    }
}

fn is_temporary_file(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    path_str.contains("/.file/id=") || path_str.starts_with("/.file/") || path_str.contains("/tmp/")
}

fn human_kb(size: u64) -> String {
    format!("{:.1} KB", size as f64 / 1024.0)
}

fn truncate_summary(input: &str) -> String {
    let mut snippet = input.trim().replace('\n', " ").replace('\r', " ");
    if snippet.len() > 120 {
        snippet.truncate(117);
        snippet.push_str("...");
    }
    snippet
}

fn format_file_summary(files: &[FileRecord]) -> String {
    files
        .iter()
        .map(|f| {
            let mime = f.mime.clone().unwrap_or_else(|| "file".into());
            format!(
                "{} ({} - {})",
                f.source_path.display(),
                human_kb(f.size),
                mime
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn mime_for_extension(ext: &str) -> Option<String> {
    mime_guess::from_ext(ext).first_raw().map(String::from)
}
