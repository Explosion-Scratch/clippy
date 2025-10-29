use crate::clipboard::{ClipboardSnapshot, FileOutput, plugins};
use crate::config::{ensure_data_dir, load_config};
use crate::data::model::{EntryKind, EntryMetadata, SearchIndex, SearchIndexRecord};
use crate::fs::{EntryPaths, entry_paths};
use crate::util::time::{self, OffsetDateTime};
use anyhow::{Context, Result, anyhow};
use clipboard_rs::{Clipboard, ClipboardContext};
use image::io::Reader as ImageReader;
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use serde_json::{self, Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

static INDEX_CACHE: OnceCell<RwLock<SearchIndex>> = OnceCell::new();

pub fn ensure_index() -> Result<SearchIndex> {
    let config = load_config()?;
    let data_path = ensure_data_dir(&config)?;
    load_index_from_disk(&data_path)
}

fn index_cell() -> &'static RwLock<SearchIndex> {
    INDEX_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn load_index() -> Result<SearchIndex> {
    Ok(index_cell().read().clone())
}

pub fn refresh_index() -> Result<()> {
    let config = load_config()?;
    let data_dir = ensure_data_dir(&config)?;
    let new_index = load_index_from_disk(&data_dir)?;
    *index_cell().write() = new_index;
    Ok(())
}

fn load_index_from_disk(data_dir: &Path) -> Result<SearchIndex> {
    let mut index = HashMap::new();
    if !data_dir.exists() {
        return Ok(index);
    }
    for year in read_dir_sorted(data_dir)? {
        let year_path = year.path();
        if !year_path.is_dir() {
            continue;
        }
        for month in read_dir_sorted(&year_path)? {
            let month_path = month.path();
            if !month_path.is_dir() {
                continue;
            }
            for first in read_dir_sorted(&month_path)? {
                let first_path = first.path();
                if !first_path.is_dir() {
                    continue;
                }
                for second in read_dir_sorted(&first_path)? {
                    let second_path = second.path();
                    if !second_path.is_dir() {
                        continue;
                    }
                    for item in read_dir_sorted(&second_path)? {
                        let item_dir = item.path();
                        if !item_dir.is_dir() {
                            continue;
                        }
                        let metadata_path = item_dir.join("metadata.json");
                        if !metadata_path.exists() {
                            continue;
                        }
                        let meta: EntryMetadata = serde_json::from_slice(&fs::read(
                            &metadata_path,
                        )?)
                        .with_context(|| {
                            format!("Failed to parse metadata at {}", metadata_path.display())
                        })?;
                        index.insert(
                            meta.hash.clone(),
                            SearchIndexRecord {
                                hash: meta.hash.clone(),
                                last_seen: meta.last_seen,
                                kind: meta.kind.clone(),
                                copy_count: meta.copy_count,
                                summary: meta.summary.clone(),
                                detected_formats: meta.detected_formats.clone(),
                                byte_size: meta.byte_size,
                            },
                        );
                    }
                }
            }
        }
    }
    Ok(index)
}

fn read_dir_sorted(path: &Path) -> Result<Vec<fs::DirEntry>> {
    if !path.is_dir() {
        return Ok(Vec::new());
    }
    let mut entries: Vec<_> = fs::read_dir(path)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    Ok(entries)
}

pub fn store_snapshot(snapshot: ClipboardSnapshot) -> Result<EntryMetadata> {
    let plugin_captures = plugins::capture_plugins(&snapshot);
    anyhow::ensure!(
        !plugin_captures.is_empty(),
        "No clipboard plugins matched snapshot"
    );

    let hash = snapshot.compute_hash();
    let config = load_config()?;
    let timestamp = time::now();
    let paths = entry_paths(&config, &hash, timestamp, None)?;
    crate::fs::layout::ensure_dir(&paths.item_dir)?;

    let outputs: Vec<FileOutput> = plugin_captures
        .iter()
        .flat_map(|capture| capture.files.clone())
        .collect();

    anyhow::ensure!(!outputs.is_empty(), "Snapshot produced no files to persist");

    for output in &outputs {
        let dest = paths.item_dir.join(&output.filename);
        if let Some(parent) = dest.parent() {
            crate::fs::layout::ensure_dir(parent)?;
        }
        fs::write(&dest, &output.bytes)
            .with_context(|| format!("Failed to write snapshot content to {}", dest.display()))?;
    }

    let prioritized = plugins::prioritized_capture(&plugin_captures).unwrap_or(&plugin_captures[0]);

    let primary = prioritized
        .files
        .first()
        .map(|f| f.filename.clone())
        .or_else(|| outputs.first().map(|f| f.filename.clone()))
        .unwrap_or_else(|| "item.bin".into());

    let summary = prioritized
        .summary
        .clone()
        .or_else(|| snapshot.summary.clone())
        .unwrap_or_else(|| prioritized.plugin_type.as_str().to_string());

    let total_byte_size: u64 = plugin_captures
        .iter()
        .map(|capture| capture.byte_size)
        .sum();

    let mut source_set = HashSet::new();
    let mut combined_sources = Vec::new();
    for capture in &plugin_captures {
        for source in &capture.sources {
            if source_set.insert(source.clone()) {
                combined_sources.push(source.clone());
            }
        }
    }
    for source in snapshot.sources() {
        if source_set.insert(source.clone()) {
            combined_sources.push(source);
        }
    }

    let plugin_order = plugins::plugin_order(&plugin_captures);
    let mut plugin_meta_map = Map::new();
    for capture in &plugin_captures {
        plugin_meta_map.insert(capture.plugin_id.to_string(), capture.metadata.clone());
    }

    let mut extra_root = Map::new();
    extra_root.insert("plugins".into(), Value::Object(plugin_meta_map));
    extra_root.insert(
        "pluginOrder".into(),
        Value::Array(plugin_order.into_iter().map(Value::String).collect()),
    );
    let extra = Value::Object(extra_root);

    let entry_kind = match prioritized.plugin_type {
        plugins::PluginType::File => EntryKind::File,
        plugins::PluginType::Image => EntryKind::Image,
        plugins::PluginType::Text | plugins::PluginType::Html | plugins::PluginType::Rtf => {
            EntryKind::Text
        }
    };

    let metadata = if paths.metadata.exists() {
        let mut existing: EntryMetadata = serde_json::from_slice(&fs::read(&paths.metadata)?)?;
        existing.copy_count += 1;
        existing.last_seen = timestamp;
        existing.byte_size = total_byte_size;
        existing.summary = Some(summary.clone());
        existing.detected_formats = snapshot.detected_formats.clone();
        existing.sources = combined_sources.clone();
        existing.files = combined_sources.clone();
        existing.content_filename = primary.clone();
        existing.extra = extra.clone();
        existing.kind = entry_kind;
        existing
    } else {
        EntryMetadata {
            hash: hash.clone(),
            kind: entry_kind,
            detected_formats: snapshot.detected_formats.clone(),
            copy_count: 1,
            first_seen: timestamp,
            last_seen: timestamp,
            byte_size: total_byte_size,
            sources: combined_sources.clone(),
            summary: Some(summary.clone()),
            version: env!("CARGO_PKG_VERSION").to_string(),
            relative_path: relative_item_path(&paths)?,
            content_filename: primary.clone(),
            files: combined_sources.clone(),
            extra: extra.clone(),
        }
    };
    fs::write(&paths.metadata, serde_json::to_vec_pretty(&metadata)?)?;
    update_index(metadata.clone());
    Ok(metadata)
}

fn relative_item_path(paths: &EntryPaths) -> Result<String> {
    let relative = paths
        .item_dir
        .strip_prefix(&paths.base_dir)
        .map_err(|_| anyhow!("Failed to compute relative path"))?;
    Ok(relative.to_string_lossy().to_string())
}

fn update_index(metadata: EntryMetadata) {
    let mut guard = index_cell().write();
    guard.insert(
        metadata.hash.clone(),
        SearchIndexRecord {
            hash: metadata.hash,
            last_seen: metadata.last_seen,
            kind: metadata.kind,
            copy_count: metadata.copy_count,
            summary: metadata.summary,
            detected_formats: metadata.detected_formats,
            byte_size: metadata.byte_size,
        },
    );
}

pub struct HistoryItem {
    pub summary: String,
    pub kind: String,
    pub metadata: EntryMetadata,
    pub offset: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SelectionFilter {
    pub include_text: bool,
    pub include_image: bool,
    pub include_file: bool,
    pub include_other: bool,
    pub require_html: bool,
    pub require_rtf: bool,
}

impl SelectionFilter {
    pub fn matches(&self, record: &SearchIndexRecord) -> bool {
        let kind_match =
            if self.include_text || self.include_image || self.include_file || self.include_other {
                (self.include_text && record.kind == EntryKind::Text)
                    || (self.include_image && record.kind == EntryKind::Image)
                    || (self.include_file && record.kind == EntryKind::File)
                    || (self.include_other && record.kind == EntryKind::Other)
            } else {
                true
            };

        let html_match = if self.require_html {
            contains_format(&record.detected_formats, "html")
        } else {
            true
        };

        let rtf_match = if self.require_rtf {
            contains_format(&record.detected_formats, "rtf")
        } else {
            true
        };

        kind_match && html_match && rtf_match
    }
}

fn contains_format(formats: &[String], needle: &str) -> bool {
    formats
        .iter()
        .any(|f| f.to_ascii_lowercase().contains(needle))
}

#[derive(Debug, Clone)]
pub struct FileDescriptor {
    pub filename: String,
    pub path: PathBuf,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct ItemPreview {
    pub content_path: Option<PathBuf>,
    pub text: Option<String>,
    pub files: Vec<FileDescriptor>,
    pub dimensions: Option<(u32, u32)>,
}

pub fn history_stream(
    index: &SearchIndex,
    limit: Option<usize>,
    query: Option<String>,
    filter: &SelectionFilter,
    from: Option<OffsetDateTime>,
    to: Option<OffsetDateTime>,
) -> Result<impl Iterator<Item = HistoryItem>> {
    let mut sorted: Vec<_> = index
        .values()
        .filter(|record| match (&from, &to) {
            (Some(start), Some(end)) => record.last_seen >= *start && record.last_seen <= *end,
            (Some(start), None) => record.last_seen >= *start,
            (None, Some(end)) => record.last_seen <= *end,
            (None, None) => true,
        })
        .filter(|record| filter.matches(record))
        .collect();
    sorted.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));
    let offsets = build_offsets(&sorted);

    let iter = sorted
        .into_iter()
        .filter(move |record| match (&query, &record.summary) {
            (Some(q), Some(summary)) => summary.to_lowercase().contains(&q.to_lowercase()),
            (Some(_), None) => false,
            (None, _) => true,
        })
        .take(limit.unwrap_or(usize::MAX))
        .filter_map(move |record| match load_metadata(&record.hash) {
            Ok(metadata) => Some(HistoryItem {
                summary: record
                    .summary
                    .clone()
                    .unwrap_or_else(|| summarize_kind(record.kind.clone(), record.byte_size)),
                kind: format!("{:?}", record.kind),
                metadata,
                offset: *offsets.get(&record.hash).unwrap_or(&0),
            }),
            Err(e) => {
                eprintln!(
                    "Warning: Failed to load metadata for {}: {}",
                    record.hash, e
                );
                None
            }
        });
    Ok(iter)
}

fn build_offsets(records: &[&SearchIndexRecord]) -> HashMap<String, usize> {
    records
        .iter()
        .enumerate()
        .map(|(index, record)| (record.hash.clone(), index))
        .collect()
}

fn summarize_kind(kind: EntryKind, byte_size: u64) -> String {
    match kind {
        EntryKind::Image => format!("Image [{}]", human_kb(byte_size)),
        EntryKind::File => format!("File [{}]", human_kb(byte_size)),
        EntryKind::Text => String::from("(text item)"),
        EntryKind::Other => String::from("(binary item)"),
    }
}

fn human_kb(size: u64) -> String {
    format!("{:.1}KB", size as f64 / 1024.0)
}

pub fn load_metadata(hash: &str) -> Result<EntryMetadata> {
    let config = load_config()?;
    let data_dir = ensure_data_dir(&config)?;
    let mut target_meta = None;
    visit_metadata(&data_dir, |meta| {
        if meta.hash == hash {
            target_meta = Some(meta);
        }
    })?;
    target_meta.ok_or_else(|| anyhow!("Metadata not found for {hash}"))
}

fn visit_metadata<F>(data_dir: &Path, mut visitor: F) -> Result<()>
where
    F: FnMut(EntryMetadata),
{
    for year in read_dir_sorted(data_dir)? {
        for month in read_dir_sorted(&year.path())? {
            for first in read_dir_sorted(&month.path())? {
                for second in read_dir_sorted(&first.path())? {
                    for item in read_dir_sorted(&second.path())? {
                        let item_dir = item.path();
                        let metadata_path = item_dir.join("metadata.json");
                        if metadata_path.exists() {
                            let meta: EntryMetadata =
                                serde_json::from_slice(&fs::read(&metadata_path)?)?;
                            visitor(meta);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn resolve_selector(
    index: &SearchIndex,
    selector: &str,
    filter: &SelectionFilter,
) -> Result<String> {
    if selector.len() >= 6 {
        if let Some(record) = index.get(selector) {
            if !filter.matches(record) {
                anyhow::bail!("Selector did not match active filters");
            }
            return Ok(record.hash.clone());
        }
    }
    let offset: usize = selector
        .parse()
        .with_context(|| format!("Invalid selector {selector}"))?;
    let mut records: Vec<_> = index
        .values()
        .filter(|record| filter.matches(record))
        .collect();
    records.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));
    if let Some(record) = records.get(offset) {
        Ok(record.hash.clone())
    } else {
        Err(anyhow!("No entry at offset {offset}"))
    }
}

pub fn copy_by_selector(hash: &str) -> Result<EntryMetadata> {
    let metadata = load_metadata(hash)?;
    let config = load_config()?;
    let data_dir = ensure_data_dir(&config)?;
    let item_dir = data_dir.join(&metadata.relative_path);
    let contents = plugins::rebuild_clipboard_contents(&metadata, &item_dir)?;
    let ctx = ClipboardContext::new().map_err(|e| anyhow!("Failed to access clipboard: {e}"))?;
    ctx.set(contents)
        .map_err(|e| anyhow!("Failed to set clipboard: {e}"))?;
    Ok(metadata)
}

pub fn delete_entry(hash: &str) -> Result<()> {
    let metadata = load_metadata(hash)?;
    let config = load_config()?;
    let data_dir = ensure_data_dir(&config)?;
    let item_dir = data_dir.join(metadata.relative_path);
    if item_dir.exists() {
        fs::remove_dir_all(&item_dir)?;
    }
    index_cell().write().remove(hash);
    Ok(())
}

pub fn load_item_preview(metadata: &EntryMetadata) -> Result<ItemPreview> {
    let config = load_config()?;
    let data_dir = ensure_data_dir(&config)?;
    let item_dir = data_dir.join(&metadata.relative_path);

    let mut files = Vec::new();
    if item_dir.exists() {
        for entry in fs::read_dir(&item_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let filename = entry.file_name().to_string_lossy().to_string();
                if filename == "metadata.json" {
                    continue;
                }
                let size = entry.metadata()?.len();
                files.push(FileDescriptor {
                    filename,
                    path,
                    size,
                });
            }
        }
    }
    files.sort_by(|a, b| a.filename.cmp(&b.filename));

    let content_path = item_dir.join(&metadata.content_filename);
    let text = read_text_preview(&content_path);
    let dimensions = if metadata.kind == EntryKind::Image {
        image_dimensions(&content_path)
    } else {
        None
    };
    let content_path = if content_path.exists() {
        Some(content_path)
    } else {
        None
    };

    Ok(ItemPreview {
        content_path,
        text,
        files,
        dimensions,
    })
}

pub fn preview_snippet(preview: &ItemPreview, metadata: &EntryMetadata) -> String {
    const MAX_PREVIEW_CHARS: usize = 160;
    if let Some(text) = &preview.text {
        return truncate_for_preview(text, MAX_PREVIEW_CHARS);
    }

    if metadata.kind == EntryKind::File {
        let count = metadata.sources.len().max(preview.files.len());
        let descriptor = if count == 1 { "file" } else { "files" };
        let location = narrowest_folder(&metadata.sources)
            .or_else(|| Some(String::from("(multiple locations)")))
            .unwrap_or_else(|| String::from("(unknown location)"));
        return format!(
            "[{} {} in {} - total {}]",
            count,
            descriptor,
            location,
            human_size(metadata.byte_size)
        );
    }

    if metadata.kind == EntryKind::Image {
        let (width_text, height_text) = match preview.dimensions {
            Some((w, h)) if w > 0 && h > 0 => (w.to_string(), h.to_string()),
            _ => ("?".into(), "?".into()),
        };
        return format!(
            "(Image item [{} x {}] [{}])",
            width_text,
            height_text,
            human_size(metadata.byte_size)
        );
    }

    metadata
        .summary
        .clone()
        .unwrap_or_else(|| format!("{:?}", metadata.kind))
}

pub fn human_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let value = bytes as f64;
    if value >= GB {
        format!("{:.2} GB", value / GB)
    } else if value >= MB {
        format!("{:.2} MB", value / MB)
    } else if value >= KB {
        format!("{:.2} KB", value / KB)
    } else {
        format!("{} B", bytes)
    }
}

fn truncate_for_preview(text: &str, max_len: usize) -> String {
    let mut clean = text.trim().replace('\r', "");
    if clean.len() > max_len {
        clean.truncate(max_len.saturating_sub(3));
        clean.push_str("...");
    }
    clean
}

fn read_text_preview(path: &Path) -> Option<String> {
    if !path.exists() {
        return None;
    }
    let mut file = fs::File::open(path).ok()?;
    let mut buffer = Vec::new();
    file.by_ref()
        .take(64 * 1024)
        .read_to_end(&mut buffer)
        .ok()?;
    if buffer.is_empty() {
        return None;
    }
    String::from_utf8(buffer.clone()).ok().or_else(|| {
        if buffer.iter().all(|b| b.is_ascii()) {
            Some(String::from_utf8_lossy(&buffer).to_string())
        } else {
            None
        }
    })
}

pub fn narrowest_folder(paths: &[String]) -> Option<String> {
    let mut iter = paths.iter().filter_map(|raw| {
        let path = Path::new(raw);
        path.parent().map(|parent| parent.to_path_buf())
    });
    let mut common = iter.next()?;
    for path in iter {
        while !path.starts_with(&common) {
            if !common.pop() {
                return Some(String::from("/"));
            }
        }
    }
    Some(common.to_string_lossy().to_string())
}

fn image_dimensions(path: &Path) -> Option<(u32, u32)> {
    if !path.exists() {
        return None;
    }
    let reader = ImageReader::open(path).ok()?;
    let reader = reader.with_guessed_format().ok()?;
    reader.into_dimensions().ok()
}
