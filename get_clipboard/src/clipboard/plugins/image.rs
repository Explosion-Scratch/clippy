use std::fs;
use std::io::{Cursor, Write};

use anyhow::{Context, Result, anyhow, bail};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use image::ImageFormat;
use serde_json::json;

use crate::clipboard::snapshot::{ClipboardSnapshot, FileOutput, human_kb, mime_for_extension};
use crate::data::model::EntryKind;

use super::{
    ClipboardJsonFormat, ClipboardPlugin, DisplayContent, ImageDisplay, PluginCapture,
    PluginContext, PluginImport, StoredFile,
};
use clipboard_rs::common::{ClipboardContent, RustImage, RustImageData};
use tempfile::NamedTempFile;

pub static IMAGE_PLUGIN: &ImagePlugin = &ImagePlugin;

pub struct ImagePlugin;

impl ClipboardPlugin for ImagePlugin {
    fn id(&self) -> &'static str {
        "image"
    }

    fn kind(&self) -> &'static str {
        "image"
    }

    fn priority(&self) -> u8 {
        1
    }

    fn entry_kind(&self) -> EntryKind {
        EntryKind::Image
    }

    fn matches(&self, snapshot: &ClipboardSnapshot) -> bool {
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

        let reader = image::ImageReader::new(std::io::Cursor::new(&bytes))
            .with_guessed_format()
            .ok()?;
        let decoded = reader.decode().ok()?;
        let width = decoded.width();
        let height = decoded.height();

        let mime = snapshot
            .image_mime
            .clone()
            .or_else(|| mime_for_extension("png"))
            .unwrap_or_else(|| "image/png".into());

        let files = vec![FileOutput {
            filename: "image__full.png".to_string(),
            bytes: bytes.clone(),
        }];

        Some(PluginCapture {
            plugin_id: self.id(),
            kind: self.kind(),
            entry_kind: self.entry_kind(),
            priority: self.priority(),
            summary: Some(format!(
                "Image {}x{} [{} - {}]",
                width,
                height,
                human_kb(bytes.len() as u64),
                mime
            )),
            search_text: None,
            files,
            metadata: json!({
                "width": width,
                "height": height,
                "mime": mime,
                "byteSize": bytes.len(),
            }),
            byte_size: bytes.len() as u64,
            sources: Vec::new(),
        })
    }

    fn to_clipboard_items(&self, ctx: &PluginContext<'_>) -> Result<Vec<ClipboardContent>> {
        let file = primary_file(ctx)?;
        let image_data = RustImageData::from_path(file.path.to_string_lossy().as_ref())
            .map_err(|e| anyhow!("Failed to load stored image: {e}"))?;
        Ok(vec![ClipboardContent::Image(image_data)])
    }

    fn display_content(&self, ctx: &PluginContext<'_>) -> Result<DisplayContent> {
        let file = primary_file(ctx)?;
        let fallback = ctx
            .metadata
            .summary
            .clone()
            .or_else(|| Some("(image item)".into()));
        Ok(DisplayContent::Image(ImageDisplay {
            path: file.path.clone(),
            fallback,
        }))
    }

    fn export_json(&self, ctx: &PluginContext<'_>) -> Result<serde_json::Value> {
        let file = primary_file(ctx)?;
        let bytes = fs::read(&file.path)
            .with_context(|| format!("Failed to read {}", file.path.display()))?;
        let mime = ctx
            .plugin_meta
            .get("mime")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("image/png");
        let data_url = format!("data:{};base64,{}", mime, BASE64.encode(bytes));
        Ok(serde_json::Value::String(data_url))
    }

    fn import_json(&self, format: &ClipboardJsonFormat) -> Result<PluginImport> {
        let data_url = format
            .data
            .as_str()
            .map(|value| value.to_string())
            .ok_or_else(|| anyhow!("image plugin expects data URL string"))?;

        let (source_mime, raw_bytes) = decode_data_url(&data_url)?;
        let reader = image::ImageReader::new(Cursor::new(&raw_bytes))
            .with_guessed_format()
            .map_err(|err| anyhow!("Failed to read image data: {err}"))?;
        let decoded = reader
            .decode()
            .map_err(|err| anyhow!("Failed to decode image data: {err}"))?;
        let width = decoded.width();
        let height = decoded.height();

        let png_bytes = if source_mime == "image/png" {
            raw_bytes.clone()
        } else {
            let mut cursor = Cursor::new(Vec::new());
            decoded
                .write_to(&mut cursor, ImageFormat::Png)
                .map_err(|err| anyhow!("Failed to convert image to PNG: {err}"))?;
            cursor.into_inner()
        };
        let png_size = png_bytes.len();

        let mut temp_file = NamedTempFile::new()
            .map_err(|err| anyhow!("Failed to create temporary file: {err}"))?;
        temp_file
            .write_all(&png_bytes)
            .map_err(|err| anyhow!("Failed to write temporary image file: {err}"))?;
        temp_file
            .flush()
            .map_err(|err| anyhow!("Failed to flush temporary image file: {err}"))?;
        let path_string = temp_file.path().to_string_lossy().to_string();
        let image_data = RustImageData::from_path(path_string.as_ref())
            .map_err(|err| anyhow!("Failed to prepare clipboard image: {err}"))?;
        temp_file
            .close()
            .map_err(|err| anyhow!("Failed to remove temporary image file: {err}"))?;

        let stored_mime = "image/png";
        let summary = Some(format!(
            "Image {}x{} [{} - {}]",
            width,
            height,
            human_kb(png_size as u64),
            source_mime
        ));

        let files = vec![FileOutput {
            filename: "image__full.png".to_string(),
            bytes: png_bytes.clone(),
        }];

        let mut capture = PluginCapture {
            plugin_id: self.id(),
            kind: self.kind(),
            entry_kind: self.entry_kind(),
            priority: self.priority(),
            summary,
            search_text: None,
            files,
            metadata: json!({
                "width": width,
                "height": height,
                "mime": stored_mime,
                "byteSize": png_size,
            }),
            byte_size: png_size as u64,
            sources: Vec::new(),
        };
        capture.finalize_metadata();

        let mut clipboard_contents = Vec::new();
        clipboard_contents.push(ClipboardContent::Image(image_data));
        clipboard_contents.push(ClipboardContent::Other("public.png".into(), png_bytes));

        Ok(PluginImport {
            capture,
            clipboard_contents,
        })
    }

    fn detail_log(&self, ctx: &PluginContext<'_>) -> Result<Vec<(String, String)>> {
        let width = ctx
            .plugin_meta
            .get("width")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let height = ctx
            .plugin_meta
            .get("height")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let mime = ctx
            .plugin_meta
            .get("mime")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("image/png");
        Ok(vec![
            ("kind".into(), self.kind().into()),
            ("dimensions".into(), format!("{}x{}", width, height)),
            ("mime".into(), mime.into()),
        ])
    }
}

fn primary_file<'a>(ctx: &'a PluginContext<'a>) -> Result<&'a StoredFile> {
    ctx.stored_files
        .first()
        .ok_or_else(|| anyhow!("image plugin missing stored file"))
}

fn decode_data_url(input: &str) -> Result<(String, Vec<u8>)> {
    let trimmed = input.trim();
    let Some(rest) = trimmed.strip_prefix("data:") else {
        bail!("image data must be a data URI");
    };
    let mut parts = rest.splitn(2, ',');
    let header = parts.next().unwrap_or("");
    let payload = parts
        .next()
        .ok_or_else(|| anyhow!("image data URI missing payload"))?;

    let mut mime = "application/octet-stream";
    let mut base64 = false;
    for segment in header.split(';') {
        let token = segment.trim();
        if token.is_empty() {
            continue;
        }
        if token.eq_ignore_ascii_case("base64") {
            base64 = true;
        } else {
            mime = token;
        }
    }

    if !base64 {
        bail!("image data URI must be base64 encoded");
    }

    let bytes = BASE64
        .decode(payload.as_bytes())
        .map_err(|err| anyhow!("Failed to decode image payload: {err}"))?;
    Ok((mime.to_string(), bytes))
}
