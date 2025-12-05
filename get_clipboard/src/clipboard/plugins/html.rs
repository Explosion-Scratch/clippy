use std::fs;

use anyhow::{Result, anyhow};
use serde_json::json;

use crate::clipboard::snapshot::{ClipboardSnapshot, FileOutput, truncate_summary};
use crate::data::model::EntryKind;

use super::{
    ClipboardJsonFormat, ClipboardPlugin, DisplayContent, PluginCapture, PluginContext,
    PluginImport,
};

pub static HTML_PLUGIN: &HtmlPlugin = &HtmlPlugin;

pub struct HtmlPlugin;

impl ClipboardPlugin for HtmlPlugin {
    fn id(&self) -> &'static str {
        "html"
    }

    fn kind(&self) -> &'static str {
        "html"
    }

    fn priority(&self) -> u8 {
        3
    }

    fn get_preview_format_order(&self) -> u8 {
        2
    }

    fn entry_kind(&self) -> EntryKind {
        EntryKind::Text
    }

    fn matches(&self, snapshot: &ClipboardSnapshot) -> bool {
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

        let files = vec![FileOutput {
            filename: "html__content.html".to_string(),
            bytes: html.clone().into_bytes(),
        }];

        let summary = truncate_summary(html);

        Some(PluginCapture {
            plugin_id: self.id(),
            kind: self.kind(),
            entry_kind: self.entry_kind(),
            priority: self.priority(),
            summary: Some(summary.clone()),
            search_text: Some(html.clone()),
            files,
            metadata: json!({
                "length": html.chars().count(),
            }),
            byte_size: html.len() as u64,
            sources: Vec::new(),
        })
    }

    fn to_clipboard_items(
        &self,
        ctx: &PluginContext<'_>,
    ) -> Result<Vec<clipboard_rs::common::ClipboardContent>> {
        let html = read_html(ctx)?;
        Ok(vec![clipboard_rs::common::ClipboardContent::Html(html)])
    }

    fn display_content(&self, ctx: &PluginContext<'_>) -> Result<DisplayContent> {
        read_html(ctx).map(DisplayContent::Text)
    }

    fn export_json(&self, ctx: &PluginContext<'_>) -> Result<serde_json::Value> {
        read_html(ctx).map(serde_json::Value::String)
    }

    fn import_json(&self, format: &ClipboardJsonFormat) -> Result<PluginImport> {
        let html = format
            .data
            .as_str()
            .map(|value| value.to_string())
            .ok_or_else(|| anyhow!("html plugin expects string data"))?;

        let files = vec![FileOutput {
            filename: "html__content.html".to_string(),
            bytes: html.clone().into_bytes(),
        }];

        let summary = truncate_summary(&html);

        let mut capture = PluginCapture {
            plugin_id: self.id(),
            kind: self.kind(),
            entry_kind: self.entry_kind(),
            priority: self.priority(),
            summary: Some(summary.clone()),
            search_text: Some(html.clone()),
            files,
            metadata: json!({
                "length": html.chars().count(),
            }),
            byte_size: html.len() as u64,
            sources: Vec::new(),
        };
        capture.finalize_metadata();

        Ok(PluginImport {
            capture,
            clipboard_contents: vec![clipboard_rs::common::ClipboardContent::Html(html)],
        })
    }

    fn detail_log(&self, ctx: &PluginContext<'_>) -> Result<Vec<(String, String)>> {
        let length = read_html(ctx)?.chars().count();
        Ok(vec![
            ("kind".into(), self.kind().into()),
            ("length".into(), length.to_string()),
        ])
    }

    fn get_preview_data(&self, ctx: &PluginContext<'_>) -> Result<serde_json::Value> {
        let html_content = read_html(ctx)?;
        let escaped = html_escape::encode_double_quoted_attribute(&html_content).to_string();
        Ok(json!({
            "content": escaped
        }))
    }
}

fn read_html(ctx: &PluginContext<'_>) -> Result<String> {
    if let Some(file) = ctx.stored_files.first() {
        return file.read_string();
    }
    let fallback = ctx.item_dir.join(&ctx.metadata.content_filename);
    if fallback.exists() {
        return fs::read_to_string(&fallback)
            .map_err(|err| anyhow!("Failed to read {}: {err}", fallback.display()));
    }
    Err(anyhow!("html content not available"))
}
