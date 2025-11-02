use std::fs;

use anyhow::{Result, anyhow};
use serde_json::json;

use crate::clipboard::snapshot::{ClipboardSnapshot, FileOutput};
use crate::data::model::EntryKind;

use super::{ClipboardPlugin, DisplayContent, PluginCapture, PluginContext};

pub static RTF_PLUGIN: &RtfPlugin = &RtfPlugin;

pub struct RtfPlugin;

impl ClipboardPlugin for RtfPlugin {
    fn id(&self) -> &'static str {
        "rtf"
    }

    fn kind(&self) -> &'static str {
        "rtf"
    }

    fn priority(&self) -> u8 {
        4
    }

    fn entry_kind(&self) -> EntryKind {
        EntryKind::Text
    }

    fn matches(&self, snapshot: &ClipboardSnapshot) -> bool {
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

        let files = vec![FileOutput {
            filename: "rtf__content.rtf".to_string(),
            bytes: rtf.clone(),
        }];

        Some(PluginCapture {
            plugin_id: self.id(),
            kind: self.kind(),
            entry_kind: self.entry_kind(),
            priority: self.priority(),
            summary: None,
            search_text: Some(String::from_utf8_lossy(rtf).into_owned()),
            files,
            metadata: json!({
                "byteSize": rtf.len(),
            }),
            byte_size: rtf.len() as u64,
            sources: Vec::new(),
        })
    }

    fn to_clipboard_items(
        &self,
        ctx: &PluginContext<'_>,
    ) -> Result<Vec<clipboard_rs::common::ClipboardContent>> {
        let rtf = read_rtf(ctx)?;
        Ok(vec![clipboard_rs::common::ClipboardContent::Rtf(rtf)])
    }

    fn display_content(&self, ctx: &PluginContext<'_>) -> Result<DisplayContent> {
        read_rtf(ctx).map(DisplayContent::Text)
    }

    fn export_json(&self, ctx: &PluginContext<'_>) -> Result<serde_json::Value> {
        read_rtf(ctx).map(serde_json::Value::String)
    }

    fn detail_log(&self, ctx: &PluginContext<'_>) -> Result<Vec<(String, String)>> {
        let bytes = read_rtf(ctx)?.into_bytes().len();
        Ok(vec![
            ("kind".into(), self.kind().into()),
            ("bytes".into(), bytes.to_string()),
        ])
    }
}

fn read_rtf(ctx: &PluginContext<'_>) -> Result<String> {
    if let Some(file) = ctx.stored_files.first() {
        let bytes = file.read_bytes()?;
        return Ok(String::from_utf8_lossy(&bytes).into_owned());
    }
    let fallback = ctx.item_dir.join(&ctx.metadata.content_filename);
    if fallback.exists() {
        let bytes = fs::read(&fallback)
            .map_err(|err| anyhow!("Failed to read {}: {err}", fallback.display()))?;
        return Ok(String::from_utf8_lossy(&bytes).into_owned());
    }
    Err(anyhow!("rtf content not available"))
}
