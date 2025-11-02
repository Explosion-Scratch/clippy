use anyhow::{Result, anyhow};
use serde_json::json;

use crate::clipboard::snapshot::{ClipboardSnapshot, FileOutput, format_file_summary, human_kb};
use crate::data::model::EntryKind;

use super::{ClipboardPlugin, DisplayContent, PluginCapture, PluginContext};

pub static FILES_PLUGIN: &FilesPlugin = &FilesPlugin;

pub struct FilesPlugin;

impl ClipboardPlugin for FilesPlugin {
    fn id(&self) -> &'static str {
        "files"
    }

    fn kind(&self) -> &'static str {
        "file"
    }

    fn priority(&self) -> u8 {
        0
    }

    fn entry_kind(&self) -> EntryKind {
        EntryKind::File
    }

    fn matches(&self, snapshot: &ClipboardSnapshot) -> bool {
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
        let joined = lines.join("\n");

        let files = vec![FileOutput {
            filename: "files__paths.txt".to_string(),
            bytes: joined.as_bytes().to_vec(),
        }];

        let byte_size = snapshot.files.iter().map(|record| record.size).sum();
        let sources = snapshot.sources();

        Some(PluginCapture {
            plugin_id: self.id(),
            kind: self.kind(),
            entry_kind: self.entry_kind(),
            priority: self.priority(),
            summary,
            search_text: Some(joined),
            files,
            metadata: json!({
                "entries": snapshot.files.clone(),
            }),
            byte_size,
            sources,
        })
    }

    fn to_clipboard_items(
        &self,
        ctx: &PluginContext<'_>,
    ) -> Result<Vec<clipboard_rs::common::ClipboardContent>> {
        let urls = collect_urls(ctx)?;
        Ok(vec![clipboard_rs::common::ClipboardContent::Files(urls)])
    }

    fn display_content(&self, ctx: &PluginContext<'_>) -> Result<DisplayContent> {
        let paths = collect_paths(ctx)?;
        Ok(DisplayContent::Lines(paths))
    }

    fn export_json(&self, ctx: &PluginContext<'_>) -> Result<serde_json::Value> {
        let paths = collect_paths(ctx)?;
        Ok(serde_json::Value::Array(
            paths.into_iter().map(serde_json::Value::String).collect(),
        ))
    }

    fn detail_log(&self, ctx: &PluginContext<'_>) -> Result<Vec<(String, String)>> {
        let entries = entry_count(ctx);
        Ok(vec![
            ("kind".into(), self.kind().into()),
            ("entries".into(), entries.to_string()),
        ])
    }
}

fn collect_entries(ctx: &PluginContext<'_>) -> Result<Vec<serde_json::Value>> {
    ctx.plugin_meta
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .map(|arr| arr.clone())
        .ok_or_else(|| anyhow!("file plugin metadata missing entries"))
}

fn collect_paths(ctx: &PluginContext<'_>) -> Result<Vec<String>> {
    let entries = collect_entries(ctx)?;
    let mut paths = Vec::new();
    for entry in entries {
        if let Some(path) = entry
            .get("source_path")
            .or_else(|| entry.get("path"))
            .and_then(serde_json::Value::as_str)
        {
            paths.push(path.to_string());
        }
    }
    Ok(paths)
}

fn collect_urls(ctx: &PluginContext<'_>) -> Result<Vec<String>> {
    collect_paths(ctx).map(|paths| {
        paths
            .into_iter()
            .map(|path| format!("file://{}", path))
            .collect()
    })
}

fn entry_count(ctx: &PluginContext<'_>) -> usize {
    ctx.plugin_meta
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .map(|arr| arr.len())
        .unwrap_or(0)
}
