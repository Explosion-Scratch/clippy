use std::fs;

use anyhow::{Result, anyhow};
use serde_json::json;

use crate::clipboard::snapshot::ClipboardSnapshot;
use crate::clipboard::snapshot::{FileOutput, truncate_summary};
use crate::data::model::EntryKind;

use super::{ClipboardPlugin, DisplayContent, PluginCapture, PluginContext};

pub static TEXT_PLUGIN: &TextPlugin = &TextPlugin;

pub struct TextPlugin;

impl ClipboardPlugin for TextPlugin {
    fn id(&self) -> &'static str {
        "text"
    }

    fn kind(&self) -> &'static str {
        "text"
    }

    fn priority(&self) -> u8 {
        2
    }

    fn entry_kind(&self) -> EntryKind {
        EntryKind::Text
    }

    fn matches(&self, snapshot: &ClipboardSnapshot) -> bool {
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

        let files = vec![FileOutput {
            filename: "text__content.txt".to_string(),
            bytes: text.clone().into_bytes(),
        }];

        Some(PluginCapture {
            plugin_id: self.id(),
            kind: self.kind(),
            entry_kind: self.entry_kind(),
            priority: self.priority(),
            summary: Some(truncate_summary(text)),
            search_text: Some(text.clone()),
            files,
            metadata: json!({
                "length": text.chars().count(),
            }),
            byte_size: text.len() as u64,
            sources: Vec::new(),
        })
    }

    fn to_clipboard_items(
        &self,
        ctx: &PluginContext<'_>,
    ) -> Result<Vec<clipboard_rs::common::ClipboardContent>> {
        let text = read_text(ctx)?;
        Ok(vec![clipboard_rs::common::ClipboardContent::Text(text)])
    }

    fn display_content(&self, ctx: &PluginContext<'_>) -> Result<DisplayContent> {
        read_text(ctx).map(DisplayContent::Text)
    }

    fn export_json(&self, ctx: &PluginContext<'_>) -> Result<serde_json::Value> {
        read_text(ctx).map(serde_json::Value::String)
    }

    fn detail_log(&self, ctx: &PluginContext<'_>) -> Result<Vec<(String, String)>> {
        let length = read_text(ctx)?.chars().count();
        Ok(vec![
            ("kind".into(), self.kind().into()),
            ("length".into(), length.to_string()),
        ])
    }
}

fn read_text(ctx: &PluginContext<'_>) -> Result<String> {
    if let Some(file) = ctx.stored_files.first() {
        return file.read_string();
    }
    let fallback = ctx.item_dir.join(&ctx.metadata.content_filename);
    if fallback.exists() {
        return fs::read_to_string(&fallback)
            .map_err(|err| anyhow!("Failed to read {}: {err}", fallback.display()));
    }
    Err(anyhow!("text content not available"))
}
