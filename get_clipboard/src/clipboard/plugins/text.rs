use std::fs;

use anyhow::{Result, anyhow};
use serde_json::json;

use crate::clipboard::snapshot::ClipboardSnapshot;
use crate::clipboard::snapshot::{FileOutput, truncate_summary};
use crate::data::model::EntryKind;

use super::{
    ClipboardJsonFormat, ClipboardPlugin, DisplayContent, PluginCapture, PluginContext,
    PluginImport,
};

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

    fn import_json(&self, format: &ClipboardJsonFormat) -> Result<PluginImport> {
        let text = format
            .data
            .as_str()
            .map(|value| value.to_string())
            .ok_or_else(|| anyhow!("text plugin expects string data"))?;

        let files = vec![FileOutput {
            filename: "text__content.txt".to_string(),
            bytes: text.clone().into_bytes(),
        }];

        let mut capture = PluginCapture {
            plugin_id: self.id(),
            kind: self.kind(),
            entry_kind: self.entry_kind(),
            priority: self.priority(),
            summary: Some(truncate_summary(&text)),
            search_text: Some(text.clone()),
            files,
            metadata: json!({
                "length": text.chars().count(),
            }),
            byte_size: text.len() as u64,
            sources: Vec::new(),
        };
        capture.finalize_metadata();

        Ok(PluginImport {
            capture,
            clipboard_contents: vec![clipboard_rs::common::ClipboardContent::Text(text)],
        })
    }

    fn detail_log(&self, ctx: &PluginContext<'_>) -> Result<Vec<(String, String)>> {
        let length = read_text(ctx)?.chars().count();
        Ok(vec![
            ("kind".into(), self.kind().into()),
            ("length".into(), length.to_string()),
        ])
    }

    fn get_preview_data(&self, ctx: &PluginContext<'_>) -> Result<serde_json::Value> {
        let text_content = read_text(ctx)?;
        let trimmed = text_content.trim();
        let is_svg = trimmed.starts_with("<svg") && trimmed.ends_with("</svg>");

        let color_re = regex::Regex::new(r"(?i)^#([0-9a-f]{3}|[0-9a-f]{6})$|^rgb\s*\(|^rgba\s*\(|^hsl\s*\(|^hsla\s*\(").unwrap();
        let is_color = color_re.is_match(trimmed);

        let content = if is_svg {
            text_content.clone()
        } else {
            html_escape::encode_text(&text_content).to_string()
        };

        let is_url = !is_svg && !is_color && (trimmed.starts_with("http://") || trimmed.starts_with("https://"));

        let mut result = json!({
            "content": content,
            "raw_text": text_content,
            "is_svg": is_svg,
            "is_color": is_color,
            "color_value": if is_color { trimmed } else { "" },
            "is_url": is_url,
            "url": if is_url { trimmed } else { "" }
        });

        if is_url {
            if let Ok(url) = url::Url::parse(trimmed) {
                if let Ok(preview) = crate::website_fetcher::fetch_website_data(&url) {
                    if let Some(obj) = result.as_object_mut() {
                        obj.insert("link_preview".to_string(), json!({
                            "title": preview.title,
                            "description": preview.description,
                            "image": preview.og_image,
                            "favicon": preview.favicon,
                            "url": trimmed
                        }));
                    }
                }
            }
        }

        Ok(result)
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
