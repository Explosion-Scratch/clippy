use std::fs;

use anyhow::{Context, Result, anyhow};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::json;

use crate::clipboard::snapshot::{ClipboardSnapshot, FileOutput, human_kb, mime_for_extension};
use crate::data::model::EntryKind;

use super::{
    ClipboardPlugin, DisplayContent, ImageDisplay, PluginCapture, PluginContext, StoredFile,
};
use clipboard_rs::common::{ClipboardContent, RustImage, RustImageData};

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
