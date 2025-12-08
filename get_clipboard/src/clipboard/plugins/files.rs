use std::fs;
use std::path::PathBuf;

use anyhow::{Result, anyhow};
use serde_json::json;

use crate::clipboard::snapshot::{
    ClipboardSnapshot, FileOutput, FileRecord, format_file_summary, human_kb,
};
use crate::data::model::EntryKind;

use super::{
    ClipboardJsonFormat, ClipboardPlugin, DisplayContent, PluginCapture, PluginContext,
    PluginImport,
};

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

        let byte_size = files.iter().map(|f| f.bytes.len() as u64).sum();
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
        let entries = collect_entries(ctx)?;
        Ok(serde_json::Value::Array(entries))
    }

    fn import_json(&self, format: &ClipboardJsonFormat) -> Result<PluginImport> {
        let array = format
            .data
            .as_array()
            .ok_or_else(|| anyhow!("files plugin expects an array"))?;

        let mut paths = Vec::new();
        for value in array {
            if let Some(path) = value.as_str() {
                paths.push(path.to_string());
                continue;
            }
            if let Some(object) = value.as_object() {
                if let Some(path) = object
                    .get("source_path")
                    .or_else(|| object.get("sourcePath"))
                    .or_else(|| object.get("path"))
                    .and_then(serde_json::Value::as_str)
                {
                    paths.push(path.to_string());
                }
            }
        }

        if paths.is_empty() {
            anyhow::bail!("files plugin requires at least one path entry");
        }

        let mut records = Vec::new();
        let mut lines = Vec::new();
        let mut total_size = 0u64;

        for raw_path in &paths {
            let path_buf = PathBuf::from(raw_path);
            let metadata = fs::metadata(&path_buf).ok();
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
            total_size += size;
            let name = path_buf
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| raw_path.clone());
            let extension = path_buf
                .extension()
                .map(|ext| ext.to_string_lossy().to_string());
            let mime = mime_guess::from_path(&path_buf)
                .first_raw()
                .map(String::from);

            records.push(FileRecord {
                name,
                extension,
                size,
                source_path: path_buf.clone(),
                mime: mime.clone(),
            });

            let mime_label = mime.unwrap_or_else(|| "file".into());
            lines.push(format!(
                "{} ({} - {})",
                path_buf.display(),
                human_kb(size),
                mime_label
            ));
        }

        let summary = Some(format_file_summary(&records));
        let joined = lines.join("\n");
        let files = vec![FileOutput {
            filename: "files__paths.txt".to_string(),
            bytes: joined.clone().into_bytes(),
        }];

        let byte_size: u64 = files.iter().map(|f| f.bytes.len() as u64).sum();

        let mut capture = PluginCapture {
            plugin_id: self.id(),
            kind: self.kind(),
            entry_kind: self.entry_kind(),
            priority: self.priority(),
            summary,
            search_text: Some(joined),
            files,
            metadata: json!({
                "entries": records,
            }),
            byte_size,
            sources: paths.clone(),
        };
        capture.finalize_metadata();

        let urls = paths
            .iter()
            .map(|path| format!("file://{}", path))
            .collect::<Vec<_>>();

        Ok(PluginImport {
            capture,
            clipboard_contents: vec![clipboard_rs::common::ClipboardContent::Files(urls)],
        })
    }

    fn detail_log(&self, ctx: &PluginContext<'_>) -> Result<Vec<(String, String)>> {
        let entries = entry_count(ctx);
        Ok(vec![
            ("kind".into(), self.kind().into()),
            ("entries".into(), entries.to_string()),
        ])
    }

    fn get_preview_data(&self, ctx: &PluginContext<'_>) -> Result<serde_json::Value> {
        let entries = collect_entries(ctx)?;
        let mut file_items = Vec::new();
        for entry in entries {
            let name = entry.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let size_bytes = entry.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
            let source_path = entry.get("source_path").and_then(|v| v.as_str()).unwrap_or_default().to_string();

            file_items.push(json!({
                "name": name,
                "size": crate::clipboard::snapshot::human_kb(size_bytes),
                "path": source_path
            }));
        }
        Ok(json!({ "files": file_items }))
    }

    fn is_editable(&self) -> bool {
        false
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
