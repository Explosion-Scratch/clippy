use crate::data::model::EntryKind;
use crate::util::hash::sha256_bytes;
use anyhow::{Result, anyhow};
use clipboard_rs::{Clipboard, ClipboardContext, ContentFormat};
use clipboard_rs::common::RustImage;
use objc2_app_kit::NSPasteboard;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;

#[derive(Debug, Clone)]
enum FormatPreview {
    Text(String),
    Binary(Vec<u8>),
    Empty,
}

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
    #[serde(skip)]
    format_previews: Vec<(String, FormatPreview)>,
}

impl ClipboardSnapshot {
    pub fn from_pasteboard(_pasteboard: &NSPasteboard) -> Result<Option<Self>> {
        let ctx =
            ClipboardContext::new().map_err(|e| anyhow!("Failed to access clipboard: {e}"))?;
        let available_formats = ctx
            .available_formats()
            .map_err(|e| anyhow!("Failed to list clipboard formats: {e}"))?;
        if available_formats.is_empty() {
            return Ok(None);
        }

        let mut detected = Vec::new();
        let mut format_previews: Vec<(String, FormatPreview)> = Vec::new();
        let mut text = None;
        let mut html = None;
        let mut rtf = None;
        let mut image_bytes = None;
        let mut image_mime = None;
        let mut files: Vec<FileRecord> = Vec::new();
        let mut seen_paths: HashSet<PathBuf> = HashSet::new();

        for format in &available_formats {
            if !detected.contains(format) {
                detected.push(format.clone());
            }

            let preview = match ctx.get_buffer(format) {
                Ok(buffer) if buffer.is_empty() => FormatPreview::Empty,
                Ok(buffer) => {
                    if let Ok(string_data) = String::from_utf8(buffer.clone()) {
                        FormatPreview::Text(string_data)
                    } else {
                        FormatPreview::Binary(buffer)
                    }
                }
                Err(_) => FormatPreview::Empty,
            };

            format_previews.push((format.clone(), preview));
        }

        if ctx.has(ContentFormat::Text) {
            if let Ok(value) = ctx.get_text() {
                if !value.is_empty() {
                    text = Some(value);
                }
            }
        }

        if ctx.has(ContentFormat::Html) {
            if let Ok(value) = ctx.get_html() {
                if !value.is_empty() {
                    html = Some(value);
                }
            }
        }

        if ctx.has(ContentFormat::Rtf) {
            if let Ok(value) = ctx.get_rich_text() {
                if !value.is_empty() {
                    rtf = Some(value.into_bytes());
                }
            }
        }

        if ctx.has(ContentFormat::Image) {
            if let Ok(image_data) = ctx.get_image() {
                match image_data.to_png() {
                    Ok(png) => {
                        let bytes = png.get_bytes().to_vec();
                        if !bytes.is_empty() {
                            image_bytes = Some(bytes);
                            image_mime = Some("image/png".to_string());
                        }
                    }
                    Err(err) => {
                        eprintln!("Failed to convert clipboard image to PNG: {err}");
                    }
                }
            }
        }

        if ctx.has(ContentFormat::Files) {
            if let Ok(raw_files) = ctx.get_files() {
                for reference in raw_files {
                    for entry in reference
                        .split('\n')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                    {
                        if let Some(path) = parse_clipboard_file_reference(entry) {
                            if seen_paths.insert(path.clone()) {
                                process_file_path(&path, &mut files);
                            }
                        }
                    }
                }
            }
        }

        if files.is_empty()
            && text.as_ref().map_or(true, |s| s.is_empty())
            && html.as_ref().map_or(true, |s| s.is_empty())
            && rtf.as_ref().map_or(true, |s| s.is_empty())
            && image_bytes.as_ref().map_or(true, |s| s.is_empty())
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

        let summary = None;

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
            format_previews,
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

    pub fn log_format_details(&self) {
        eprintln!("\n=== Clipboard Change Detected ===");
        eprintln!("All formats ({} total):", self.format_previews.len());

        for (format_name, preview) in &self.format_previews {
            match preview {
                FormatPreview::Text(text) => {
                    let preview = self.truncate_preview(text, 120);
                    eprintln!("  • {}: \"{}\"", format_name, preview);
                }
                FormatPreview::Binary(bytes) => {
                    if is_likely_text_binary(bytes) {
                        let text = String::from_utf8_lossy(bytes);
                        let preview = self.truncate_preview(&text, 120);
                        eprintln!(
                            "  • {}: \"{}\" ({} bytes)",
                            format_name,
                            preview,
                            bytes.len()
                        );
                    } else {
                        eprintln!("  • {}: <binary data, {} bytes>", format_name, bytes.len());
                    }
                }
                FormatPreview::Empty => {
                    eprintln!("  • {}: <empty>", format_name);
                }
            }
        }

        eprintln!("================================\n");
    }

    fn truncate_preview(&self, content: &str, max_len: usize) -> String {
        let normalized = content.trim().replace('\n', " ").replace('\r', " ");
        let mut chars = normalized.chars();
        let char_count = normalized.chars().count();

        if char_count > max_len {
            let take_len = max_len.saturating_sub(3);
            let mut truncated = String::new();
            for ch in chars.by_ref().take(take_len) {
                truncated.push(ch);
            }
            truncated.push_str("...");
            truncated
        } else {
            normalized
        }
    }
}

fn parse_clipboard_file_reference(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(url) = Url::parse(trimmed) {
        if url.scheme() == "file" {
            return url.to_file_path().ok();
        }
    }

    Some(PathBuf::from(trimmed))
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

pub(crate) fn human_kb(size: u64) -> String {
    format!("{:.1} KB", size as f64 / 1024.0)
}

pub(crate) fn truncate_summary(input: &str) -> String {
    let snippet = input.trim().replace('\n', " ").replace('\r', " ");
    let char_count = snippet.chars().count();
    if char_count > 120 {
        let truncated: String = snippet.chars().take(117).collect();
        format!("{}...", truncated)
    } else {
        snippet
    }
}

pub(crate) fn format_file_summary(files: &[FileRecord]) -> String {
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

pub(crate) fn mime_for_extension(ext: &str) -> Option<String> {
    mime_guess::from_ext(ext).first_raw().map(String::from)
}

fn is_likely_text_binary(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let sample_size = bytes.len().min(512);
    let sample = &bytes[..sample_size];
    let non_printable = sample
        .iter()
        .filter(|&&b| b < 32 && b != 9 && b != 10 && b != 13)
        .count();
    non_printable < sample_size / 10
}
