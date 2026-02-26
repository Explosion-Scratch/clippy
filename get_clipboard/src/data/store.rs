use crate::clipboard::{plugins, ClipboardSnapshot};
use crate::clipboard::plugins::PluginCapture;
use crate::config::{ensure_data_dir, load_config};
use crate::data::model::{EntryKind, EntryMetadata, SearchIndex, SearchIndexRecord};
use crate::fs::{EntryPaths, entry_paths};
pub use crate::search::SelectionFilter;
use crate::search::{SearchOptions, search};
use crate::util::time::{self, OffsetDateTime};
use anyhow::{Context, Result, anyhow};
use clipboard_rs::{Clipboard, ClipboardContext};
use image::ImageReader;
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use serde_json::{self, Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

struct IndexCacheData {
    pub index: SearchIndex,
    pub mtime: Option<std::time::SystemTime>,
}

static INDEX_CACHE: OnceCell<RwLock<IndexCacheData>> = OnceCell::new();
const MAX_SEARCH_TEXT_CHARS: usize = 65536;
const MAX_SEARCH_TEXT_SEGMENTS: usize = 4;

enum CopyCountMode {
    Increment,
    Override(u64),
}

pub fn ensure_index() -> Result<SearchIndex> {
    let config = load_config()?;
    let data_path = ensure_data_dir(&config)?;
    load_index_from_disk(&data_path)
}

fn write_index_file(path: &Path, index: &SearchIndex) {
    if let Ok(bytes) = serde_json::to_vec_pretty(index) {
        let _ = std::fs::write(path, bytes);
    }
}

fn index_cell() -> &'static RwLock<IndexCacheData> {
    INDEX_CACHE.get_or_init(|| RwLock::new(IndexCacheData {
        index: HashMap::new(),
        mtime: None,
    }))
}

pub fn load_index() -> Result<SearchIndex> {
    {
        // Try reading fast without a write lock first
        let guard = index_cell().read();
        if let Ok(config) = load_config() {
            if let Ok(data_dir) = ensure_data_dir(&config) {
                let index_file = data_dir.join("index.json");
                let current_mtime = std::fs::metadata(&index_file).and_then(|m| m.modified()).ok();
                if !guard.index.is_empty() && current_mtime == guard.mtime {
                    return Ok(guard.index.clone());
                }
            }
        }
    }

    let config = load_config()?;
    let data_dir = ensure_data_dir(&config)?;
    let index_file = data_dir.join("index.json");
    
    let mut guard = index_cell().write();
    let current_mtime = std::fs::metadata(&index_file).and_then(|m| m.modified()).ok();
    
    let needs_reload = guard.index.is_empty() || (current_mtime.is_some() && current_mtime != guard.mtime);
    
    if needs_reload {
        if index_file.exists() {
            match std::fs::read(&index_file) {
                Ok(bytes) => {
                    match serde_json::from_slice::<SearchIndex>(&bytes) {
                        Ok(new_index) => {
                            guard.index = new_index;
                            guard.mtime = current_mtime;
                        }
                        Err(e) => {
                            eprintln!("Warning: Corrupted index.json: {}", e);
                            guard.index = load_index_from_disk(&data_dir)?;
                            write_index_file(&index_file, &guard.index);
                            guard.mtime = std::fs::metadata(&index_file).and_then(|m| m.modified()).ok();
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to read index.json: {}", e);
                    guard.index = load_index_from_disk(&data_dir)?;
                    write_index_file(&index_file, &guard.index);
                    guard.mtime = std::fs::metadata(&index_file).and_then(|m| m.modified()).ok();
                }
            }
        } else {
            guard.index = load_index_from_disk(&data_dir)?;
            write_index_file(&index_file, &guard.index);
            guard.mtime = std::fs::metadata(&index_file).and_then(|m| m.modified()).ok();
        }
    }
    
    Ok(guard.index.clone())
}

pub fn refresh_index() -> Result<()> {
    let config = load_config()?;
    let data_dir = ensure_data_dir(&config)?;
    let new_index = load_index_from_disk(&data_dir)?;
    let index_file = data_dir.join("index.json");
    write_index_file(&index_file, &new_index);
    let mut guard = index_cell().write();
    guard.index = new_index;
    guard.mtime = std::fs::metadata(&index_file).and_then(|m| m.modified()).ok();
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
                        let bytes = match fs::read(&metadata_path) {
                            Ok(b) => b,
                            Err(e) => {
                                eprintln!("Warning: Failed to read {}: {}", metadata_path.display(), e);
                                continue;
                            }
                        };
                        let meta: EntryMetadata = match serde_json::from_slice(&bytes) {
                            Ok(m) => m,
                            Err(e) => {
                                eprintln!("Warning: Corrupt metadata at {}: {}", metadata_path.display(), e);
                                continue;
                            }
                        };
                        index.insert(
                            meta.hash.clone(),
                            SearchIndexRecord {
                                hash: meta.hash.clone(),
                                last_seen: meta.last_seen,
                                kind: meta.kind.clone(),
                                copy_count: meta.copy_count,
                                summary: meta.summary.clone(),
                                search_text: meta.search_text.clone(),
                                detected_formats: meta.detected_formats.clone(),
                                byte_size: meta.byte_size,
                                relative_path: meta.relative_path.clone(),
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
    let timestamp = time::now();
    let sources = snapshot.sources();
    let detected_formats = snapshot.detected_formats.clone();
    let summary_hint = snapshot.summary.clone();

    persist_entry(
        &hash,
        timestamp,
        &plugin_captures,
        summary_hint,
        detected_formats,
        sources,
        CopyCountMode::Increment,
        None,
        None,
    )
}

pub fn store_json_item(item: &plugins::ClipboardJsonFullItem) -> Result<EntryMetadata> {
    let import = plugins::prepare_import(item)?;
    let timestamp = match item.date.as_ref() {
        Some(raw) => crate::util::time::parse_date(raw).unwrap_or_else(|_| time::now()),
        None => time::now(),
    };
    let first_seen = match item.first_date.as_ref() {
        Some(raw) => crate::util::time::parse_date(raw).ok(),
        None => None,
    };
    let hash = if let Some(existing) = item
        .id
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        existing.to_string()
    } else {
        plugins::compute_json_item_hash(item)?
    };
    let detected_formats = if !item.detected_formats.is_empty() {
        item.detected_formats.clone()
    } else {
        inferred_detected_formats(&import)
    };
    let summary = item.summary.clone();
    let copy_count = item.copy_count.unwrap_or(1);
    let sources = item.sources.clone();
    let search_override = item.search_text.clone();

    persist_entry(
        &hash,
        timestamp,
        &import.captures,
        summary,
        detected_formats,
        sources,
        CopyCountMode::Override(copy_count),
        search_override,
        first_seen,
    )
}

pub fn copy_json_item(item: &plugins::ClipboardJsonFullItem) -> Result<()> {
    let import = plugins::prepare_import(item)?;
    anyhow::ensure!(
        !import.clipboard_contents.is_empty(),
        "Clipboard payload included no formats"
    );
    let ctx = ClipboardContext::new().map_err(|e| anyhow!("Failed to access clipboard: {e}"))?;
    ctx.set(import.clipboard_contents)
        .map_err(|e| anyhow!("Failed to set clipboard: {e}"))?;
    Ok(())
}

fn inferred_detected_formats(import: &plugins::ClipboardJsonImport) -> Vec<String> {
    let mut formats = Vec::new();
    for capture in &import.captures {
        let label = match capture.plugin_id {
            "text" => "public.utf8-plain-text",
            "html" => "public.html",
            "rtf" => "public.rtf",
            "image" => "public.png",
            "files" => "public.file-url",
            _ => "public.data",
        };
        if !formats.iter().any(|existing| existing == label) {
            formats.push(label.to_string());
        }
    }
    formats
}

fn build_search_text(
    captures: &[PluginCapture],
    summary: &str,
    override_text: Option<&str>,
) -> Option<String> {
    if let Some(override_text) = override_text {
        let clipped = clip_search_text(override_text);
        if !clipped.is_empty() {
            return Some(clipped);
        }
    }

    let mut seen_search = HashSet::new();
    let mut search_segments = Vec::new();
    for capture in captures {
        if let Some(text) = capture.search_text.as_ref() {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }
            if seen_search.insert(trimmed.to_string()) {
                let segment_max = MAX_SEARCH_TEXT_CHARS / MAX_SEARCH_TEXT_SEGMENTS.max(1);
                let clipped = clip_search_text_to_max(trimmed, segment_max);
                if !clipped.is_empty() {
                    search_segments.push(clipped);
                }
                if search_segments.len() >= MAX_SEARCH_TEXT_SEGMENTS {
                    break;
                }
            }
        }
    }

    if search_segments.is_empty() {
        let fallback = clip_search_text(summary);
        if fallback.is_empty() {
            None
        } else {
            Some(fallback)
        }
    } else {
        let joined = search_segments.join("\n\n");
        let clipped = clip_search_text(&joined);
        if clipped.is_empty() {
            None
        } else {
            Some(clipped)
        }
    }
}

fn combine_sources(captures: &[PluginCapture], base_sources: Vec<String>) -> Vec<String> {
    let mut source_set = HashSet::new();
    let mut combined = Vec::new();
    for capture in captures {
        for source in &capture.sources {
            if source_set.insert(source.clone()) {
                combined.push(source.clone());
            }
        }
    }
    for source in base_sources {
        if source_set.insert(source.clone()) {
            combined.push(source);
        }
    }
    combined
}

fn persist_entry(
    hash: &str,
    timestamp: OffsetDateTime,
    plugin_captures: &[PluginCapture],
    summary_hint: Option<String>,
    detected_formats: Vec<String>,
    base_sources: Vec<String>,
    copy_mode: CopyCountMode,
    search_override: Option<String>,
    first_seen_override: Option<OffsetDateTime>,
) -> Result<EntryMetadata> {
    anyhow::ensure!(!plugin_captures.is_empty(), "No plugin captures available");

    let config = load_config()?;
    let paths = entry_paths(&config, hash, timestamp, None)?;
    crate::fs::layout::ensure_dir(&paths.item_dir)?;

    let mut wrote_file = false;
    for capture in plugin_captures {
        for output in &capture.files {
            let dest = paths.item_dir.join(&output.filename);
            if let Some(parent) = dest.parent() {
                crate::fs::layout::ensure_dir(parent)?;
            }
            fs::write(&dest, &output.bytes).with_context(|| {
                format!("Failed to write snapshot content to {}", dest.display())
            })?;
            wrote_file = true;
        }
    }
    anyhow::ensure!(wrote_file, "No plugin produced persisted files");

    let prioritized = plugins::prioritized_capture(plugin_captures).unwrap_or(&plugin_captures[0]);
    let primary = prioritized
        .files
        .first()
        .map(|file| file.filename.clone())
        .or_else(|| {
            plugin_captures
                .iter()
                .flat_map(|capture| capture.files.first())
                .map(|file| file.filename.clone())
                .next()
        })
        .unwrap_or_else(|| "item.bin".into());

    let summary = prioritized
        .summary
        .clone()
        .or(summary_hint)
        .or_else(|| {
            plugin_captures
                .iter()
                .find_map(|capture| capture.summary.clone())
        })
        .unwrap_or_else(|| prioritized.kind.to_string());

    let search_text = build_search_text(plugin_captures, &summary, search_override.as_deref());

    let total_byte_size: u64 = plugin_captures
        .iter()
        .map(|capture| capture.byte_size)
        .sum();

    let combined_sources = combine_sources(plugin_captures, base_sources);

    let plugin_order = plugins::plugin_order(plugin_captures);
    let mut plugin_meta_map = Map::new();
    for capture in plugin_captures {
        plugin_meta_map.insert(capture.plugin_id.to_string(), capture.metadata.clone());
    }

    let mut extra_root = Map::new();
    extra_root.insert("plugins".into(), Value::Object(plugin_meta_map));
    extra_root.insert(
        "pluginOrder".into(),
        Value::Array(plugin_order.into_iter().map(Value::String).collect()),
    );
    let extra = Value::Object(extra_root);

    let entry_kind = prioritized.entry_kind.clone();

    let metadata = if paths.metadata.exists() {
        let mut existing: EntryMetadata = serde_json::from_slice(&fs::read(&paths.metadata)?)?;
        existing.last_seen = timestamp;
        existing.byte_size = total_byte_size;
        existing.summary = Some(summary.clone());
        existing.search_text = search_text.clone();
        existing.detected_formats = detected_formats.clone();
        existing.sources = combined_sources.clone();
        existing.files = combined_sources.clone();
        existing.content_filename = primary.clone();
        existing.extra = extra.clone();
        existing.kind = entry_kind.clone();
        match copy_mode {
            CopyCountMode::Increment => {
                existing.copy_count = existing.copy_count.saturating_add(1);
            }
            CopyCountMode::Override(value) => {
                existing.copy_count = value.max(1);
            }
        }
        existing
    } else {
        let copy_count = match copy_mode {
            CopyCountMode::Increment => 1,
            CopyCountMode::Override(value) => value.max(1),
        };
        EntryMetadata {
            hash: hash.to_string(),
            kind: entry_kind.clone(),
            detected_formats: detected_formats.clone(),
            copy_count,
            first_seen: first_seen_override.unwrap_or(timestamp),
            last_seen: timestamp,
            byte_size: total_byte_size,
            sources: combined_sources.clone(),
            summary: Some(summary.clone()),
            search_text: search_text.clone(),
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

fn clip_search_text(input: &str) -> String {
    clip_search_text_to_max(input, MAX_SEARCH_TEXT_CHARS)
}

fn clip_search_text_to_max(input: &str, max_chars: usize) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    trimmed.chars().take(max_chars).collect::<String>()
}

fn relative_item_path(paths: &EntryPaths) -> Result<String> {
    let relative = paths
        .item_dir
        .strip_prefix(&paths.base_dir)
        .map_err(|_| anyhow!("Failed to compute relative path"))?;
    Ok(relative.to_string_lossy().to_string())
}

fn update_index(metadata: EntryMetadata) {
    let _ = load_index(); // Ensure cache is populated before we mutate it
    let mut guard = index_cell().write();
    guard.index.insert(
        metadata.hash.clone(),
        SearchIndexRecord {
            hash: metadata.hash,
            last_seen: metadata.last_seen,
            kind: metadata.kind,
            copy_count: metadata.copy_count,
            summary: metadata.summary,
            search_text: metadata.search_text,
            detected_formats: metadata.detected_formats,
            byte_size: metadata.byte_size,
            relative_path: metadata.relative_path,
        },
    );
    if let Ok(config) = load_config() {
        if let Ok(data_dir) = ensure_data_dir(&config) {
            let index_file = data_dir.join("index.json");
            write_index_file(&index_file, &guard.index);
            guard.mtime = std::fs::metadata(&index_file).and_then(|m| m.modified()).ok();
        }
    }
}

#[derive(Clone)]
pub struct HistoryItem {
    pub summary: String,
    pub kind: String,
    pub metadata: EntryMetadata,
    pub offset: usize,
    pub global_offset: usize,
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

pub fn load_history_items(
    index: &SearchIndex,
    options: &SearchOptions,
) -> Result<(Vec<HistoryItem>, bool)> {
    let result = search(index, options);
    let mut items = Vec::new();
    for hit in result.hits {
        match load_metadata(&hit.hash) {
            Ok(metadata) => {
                let summary = hit
                    .summary
                    .clone()
                    .or_else(|| metadata.summary.clone())
                    .unwrap_or_else(|| String::from("Clipboard item"));
                items.push(HistoryItem {
                    summary,
                    kind: format!("{:?}", hit.kind),
                    metadata,
                    offset: hit.offset,
                    global_offset: hit.global_offset,
                });
            }
            Err(error) => {
                eprintln!(
                    "Warning: Failed to load metadata for {}: {}",
                    hit.hash, error
                );
            }
        }
    }
    Ok((items, result.has_more))
}

pub fn stream_history_items<F>(
    index: &SearchIndex,
    options: &SearchOptions,
    mut callback: F,
) -> Result<()>
where
    F: FnMut(&HistoryItem) -> Result<bool>,
{
    let result = search(index, options);
    for hit in result.hits {
        match load_metadata(&hit.hash) {
            Ok(metadata) => {
                let summary = hit
                    .summary
                    .clone()
                    .or_else(|| metadata.summary.clone())
                    .unwrap_or_else(|| String::from("Clipboard item"));
                let item = HistoryItem {
                    summary,
                    kind: format!("{:?}", hit.kind),
                    metadata,
                    offset: hit.offset,
                    global_offset: hit.global_offset,
                };
                if !callback(&item)? {
                    break;
                }
            }
            Err(error) => {
                eprintln!(
                    "Warning: Failed to load metadata for {}: {}",
                    hit.hash, error
                );
            }
        }
    }
    Ok(())
}

pub fn history_stream(
    index: &SearchIndex,
    limit: Option<usize>,
    query: Option<String>,
    filter: &SelectionFilter,
    from: Option<OffsetDateTime>,
    to: Option<OffsetDateTime>,
) -> Result<impl Iterator<Item = HistoryItem>> {
    let mut options = SearchOptions::default();
    options.limit = limit;
    options.query = query;
    options.filter = filter.clone();
    options.from = from;
    options.to = to;

    let (items, _) = load_history_items(index, &options)?;
    Ok(items.into_iter())
}

pub fn load_metadata(hash: &str) -> Result<EntryMetadata> {
    let guard = index_cell().read();
    if let Some(record) = guard.index.get(hash) {
        let config = load_config()?;
        let data_dir = ensure_data_dir(&config)?;
        let metadata_path = data_dir.join(&record.relative_path).join("metadata.json");
        if metadata_path.exists() {
            let meta: EntryMetadata = serde_json::from_slice(&fs::read(&metadata_path)?)
                .with_context(|| format!("Failed to parse metadata at {}", metadata_path.display()))?;
            return Ok(meta);
        }
    }
    
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
                            let bytes = match fs::read(&metadata_path) {
                                Ok(b) => b,
                                Err(e) => {
                                    eprintln!("Warning: Failed to read {}: {}", metadata_path.display(), e);
                                    continue;
                                }
                            };
                            match serde_json::from_slice::<EntryMetadata>(&bytes) {
                                Ok(meta) => visitor(meta),
                                Err(e) => {
                                    eprintln!("Warning: Corrupt metadata at {}: {}", metadata_path.display(), e);
                                    continue;
                                }
                            }
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
    let start = std::time::Instant::now();
    let metadata = load_metadata(hash)?;
    println!("load_metadata took: {:?}", start.elapsed());
    
    let t2 = std::time::Instant::now();
    let config = load_config()?;
    let data_dir = ensure_data_dir(&config)?;
    println!("config/dir took: {:?}", t2.elapsed());
    
    let t3 = std::time::Instant::now();
    let item_dir = data_dir.join(&metadata.relative_path);
    let contents = plugins::rebuild_clipboard_contents(&metadata, &item_dir)?;
    println!("rebuild_clipboard_contents took: {:?}", t3.elapsed());
    
    let t4 = std::time::Instant::now();
    let ctx = ClipboardContext::new().map_err(|e| anyhow!("Failed to access clipboard: {e}"))?;
    println!("ClipboardContext::new took: {:?}", t4.elapsed());
    
    let t5 = std::time::Instant::now();
    ctx.set(contents)
        .map_err(|e| anyhow!("Failed to set clipboard: {e}"))?;
    println!("ctx.set took: {:?}", t5.elapsed());
    
    Ok(metadata)
}

pub fn copy_plain_by_selector(hash: &str) -> Result<EntryMetadata> {
    let metadata = load_metadata(hash)?;
    let config = load_config()?;
    let data_dir = ensure_data_dir(&config)?;
    let item_dir = data_dir.join(&metadata.relative_path);

    let (order, map) = match plugins::extract_plugin_meta(&metadata)? {
        Some(x) => x,
        None => return Err(anyhow!("No plugin metadata found")),
    };

    // Check what plugins are available
    let has_text = order.iter().any(|id| id == "text");
    let has_image = order.iter().any(|id| id == "image");
    let has_html = order.iter().any(|id| id == "html");
    let has_rtf = order.iter().any(|id| id == "rtf");
    let has_files = order.iter().any(|id| id == "files");

    // If item is an image-only item, paste as normal (no text available)
    if has_image && !has_text && !has_html && !has_rtf && !has_files {
        // Fall back to normal paste for image-only items
        let contents = plugins::rebuild_clipboard_contents(&metadata, &item_dir)?;
        let ctx = ClipboardContext::new().map_err(|e| anyhow!("Failed to access clipboard: {e}"))?;
        ctx.set(contents).map_err(|e| anyhow!("Failed to set clipboard: {e}"))?;
        return Ok(metadata);
    }

    let mut text_content: Option<String> = None;

    // Priority 1: Prefer "text" plugin if available
    if has_text {
        if let Some(plugin_meta) = map.get("text") {
            if let Some(plugin) = plugins::plugin_by_id("text") {
                let stored_files = plugins::load_plugin_files(&item_dir, plugin_meta)?;
                let ctx = plugins::PluginContext {
                    metadata: &metadata,
                    plugin_meta,
                    item_dir: &item_dir,
                    stored_files: &stored_files,
                };
                if let Ok(content) = plugin.to_clipboard_items(&ctx) {
                    for item in content {
                        if let clipboard_rs::common::ClipboardContent::Text(t) = item {
                            text_content = Some(t);
                            break;
                        }
                    }
                }
            }
        }
    }

    // Priority 2: If no text but has HTML, extract HTML content as plain text
    if text_content.is_none() && has_html {
        if let Some(plugin_meta) = map.get("html") {
            if let Some(plugin) = plugins::plugin_by_id("html") {
                let stored_files = plugins::load_plugin_files(&item_dir, plugin_meta)?;
                let ctx = plugins::PluginContext {
                    metadata: &metadata,
                    plugin_meta,
                    item_dir: &item_dir,
                    stored_files: &stored_files,
                };
                if let Ok(content) = plugin.to_clipboard_items(&ctx) {
                    for item in content {
                        if let clipboard_rs::common::ClipboardContent::Html(h) = item {
                            text_content = Some(h);
                            break;
                        }
                    }
                }
            }
        }
    }

    // Priority 3: If no text/HTML but has RTF, extract RTF content as plain text
    if text_content.is_none() && has_rtf {
        if let Some(plugin_meta) = map.get("rtf") {
            if let Some(plugin) = plugins::plugin_by_id("rtf") {
                let stored_files = plugins::load_plugin_files(&item_dir, plugin_meta)?;
                let ctx = plugins::PluginContext {
                    metadata: &metadata,
                    plugin_meta,
                    item_dir: &item_dir,
                    stored_files: &stored_files,
                };
                if let Ok(content) = plugin.to_clipboard_items(&ctx) {
                    for item in content {
                        if let clipboard_rs::common::ClipboardContent::Rtf(r) = item {
                            text_content = Some(r);
                            break;
                        }
                    }
                }
            }
        }
    }

    // Priority 4: If still no text and has files, paste file paths as text
    if text_content.is_none() && has_files {
        if let Some(plugin_meta) = map.get("files") {
            // Extract file paths from plugin metadata
            if let Some(entries) = plugin_meta.get("entries").and_then(|v| v.as_array()) {
                let paths: Vec<String> = entries
                    .iter()
                    .filter_map(|entry| {
                        entry.get("source_path")
                            .or_else(|| entry.get("path"))
                            .and_then(|v| v.as_str())
                            .map(String::from)
                    })
                    .collect();
                if !paths.is_empty() {
                    text_content = Some(paths.join("\n"));
                }
            }
        }
    }

    // If we found text content, set it as plain text
    if let Some(text) = text_content {
        let ctx = ClipboardContext::new().map_err(|e| anyhow!("Failed to access clipboard: {e}"))?;
        ctx.set_text(text).map_err(|e| anyhow!("Failed to set clipboard text: {e}"))?;
        return Ok(metadata);
    }

    // Last resort: fall back to normal paste if nothing else worked
    let contents = plugins::rebuild_clipboard_contents(&metadata, &item_dir)?;
    let ctx = ClipboardContext::new().map_err(|e| anyhow!("Failed to access clipboard: {e}"))?;
    ctx.set(contents).map_err(|e| anyhow!("Failed to set clipboard: {e}"))?;
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
    let _ = load_index(); // Ensure cache is populated before we mutate and flush
    let mut guard = index_cell().write();
    guard.index.remove(hash);
    let index_file = data_dir.join("index.json");
    write_index_file(&index_file, &guard.index);
    guard.mtime = std::fs::metadata(&index_file).and_then(|m| m.modified()).ok();
    Ok(())
}

pub fn increment_copy_count(hash: &str) -> Result<EntryMetadata> {
    let mut metadata = load_metadata(hash)?;
    metadata.copy_count = metadata.copy_count.saturating_add(1);
    let config = load_config()?;
    let data_dir = ensure_data_dir(&config)?;
    let item_dir = data_dir.join(&metadata.relative_path);
    let metadata_path = item_dir.join("metadata.json");
    fs::write(&metadata_path, serde_json::to_vec_pretty(&metadata)?)?;
    update_index(metadata.clone());
    Ok(metadata)
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
        let resolved_paths = resolved_file_paths(metadata);
        let count = if !resolved_paths.is_empty() {
            resolved_paths.len()
        } else {
            metadata.sources.len().max(preview.files.len())
        };
        let descriptor = if count == 1 { "file" } else { "files" };
        let location_hint = if !resolved_paths.is_empty() {
            narrowest_folder(resolved_paths.as_slice())
        } else if !metadata.sources.is_empty() {
            narrowest_folder(metadata.sources.as_slice())
        } else {
            None
        };
        let location = location_hint
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
    let clean = text.trim().replace('\r', "");
    let char_count = clean.chars().count();
    if char_count > max_len {
        let truncated: String = clean.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    } else {
        clean
    }
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

pub fn resolved_file_paths(metadata: &EntryMetadata) -> Vec<String> {
    let plugin_paths = file_plugin_paths(metadata);
    if !plugin_paths.is_empty() {
        return dedupe_preserve_order(plugin_paths);
    }
    if !metadata.files.is_empty() {
        return dedupe_preserve_order(metadata.files.clone());
    }
    if !metadata.sources.is_empty() {
        return dedupe_preserve_order(metadata.sources.clone());
    }
    Vec::new()
}

pub fn saved_format_labels(metadata: &EntryMetadata) -> Vec<String> {
    let mut labels = Vec::new();
    if let Some(root) = metadata.extra.as_object() {
        if let Some(plugins) = root.get("plugins").and_then(Value::as_object) {
            let mut ordered_ids: Vec<String> = root
                .get("pluginOrder")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .map(String::from)
                        .collect::<Vec<String>>()
                })
                .unwrap_or_else(|| plugins.keys().cloned().collect());
            if ordered_ids.is_empty() {
                ordered_ids = plugins.keys().cloned().collect();
            }
            for plugin_id in ordered_ids {
                if let Some(plugin) = plugins.get(plugin_id.as_str()).and_then(Value::as_object) {
                    let plugin_type = plugin
                        .get("pluginType")
                        .and_then(Value::as_str)
                        .unwrap_or(plugin_id.as_str());
                    let label = normalize_plugin_label(plugin_type);
                    if !label.is_empty() && !labels.contains(&label) {
                        labels.push(label);
                    }
                }
            }
        }
    }
    if labels.is_empty() {
        fallback_format_labels(metadata)
    } else {
        labels
    }
}

fn file_plugin_paths(metadata: &EntryMetadata) -> Vec<String> {
    metadata
        .extra
        .as_object()
        .and_then(|root| root.get("plugins"))
        .and_then(Value::as_object)
        .and_then(|plugins| plugins.get("files"))
        .and_then(Value::as_object)
        .and_then(|file_plugin| file_plugin.get("entries"))
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    entry
                        .get("source_path")
                        .or_else(|| entry.get("path"))
                        .and_then(Value::as_str)
                        .map(String::from)
                })
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

fn dedupe_preserve_order(items: Vec<String>) -> Vec<String> {
    let mut seen = Vec::new();
    for item in items {
        if !seen.contains(&item) {
            seen.push(item);
        }
    }
    seen
}

fn normalize_plugin_label(plugin_type: &str) -> String {
    match plugin_type {
        "file" | "files" => String::from("Files"),
        "text" => String::from("Text"),
        "html" => String::from("HTML"),
        "rtf" => String::from("RTF"),
        "image" => String::from("Image"),
        other => capitalize_label(other),
    }
}

fn capitalize_label(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut label = String::new();
    for upper in first.to_uppercase() {
        label.push(upper);
    }
    label.push_str(chars.as_str());
    label
}

fn fallback_format_labels(metadata: &EntryMetadata) -> Vec<String> {
    let mut labels = Vec::new();
    for format in &metadata.detected_formats {
        let lower = format.to_ascii_lowercase();
        let label = if lower.contains("html") {
            "HTML"
        } else if lower.contains("rtf") {
            "RTF"
        } else if lower.contains("text") || lower.contains("utf8") {
            "Text"
        } else if lower.contains("image") || lower.contains("png") || lower.contains("jpeg") {
            "Image"
        } else if lower.contains("file") {
            "Files"
        } else {
            continue;
        };
        let label_string = String::from(label);
        if !labels.contains(&label_string) {
            labels.push(label_string);
        }
    }
    labels
}
