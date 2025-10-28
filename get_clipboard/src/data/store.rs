use crate::clipboard::{ClipboardClass, ClipboardSnapshot, handler_from_metadata};
use crate::config::{ensure_data_dir, load_config};
use crate::data::model::{EntryKind, EntryMetadata, SearchIndex, SearchIndexRecord};
use crate::fs::{EntryPaths, entry_paths};
use crate::util::time::{self, OffsetDateTime};
use anyhow::{Context, Result, anyhow};
use clipboard_rs::{Clipboard, ClipboardContext};
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use serde_json::{self, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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
    let handler = snapshot.classify()?;
    let hash = snapshot.compute_hash();
    let config = load_config()?;
    let timestamp = time::now();
    let paths = entry_paths(&config, &hash, timestamp, None)?;
    crate::fs::layout::ensure_dir(&paths.item_dir)?;

    let outputs = handler
        .to_files()
        .context("Snapshot handler produced no file outputs")?;
    anyhow::ensure!(!outputs.is_empty(), "Snapshot produced no files to persist");

    for output in &outputs {
        let dest = paths.item_dir.join(&output.filename);
        if let Some(parent) = dest.parent() {
            crate::fs::layout::ensure_dir(parent)?;
        }
        fs::write(&dest, &output.bytes)
            .with_context(|| format!("Failed to write snapshot content to {}", dest.display()))?;
    }

    let primary = outputs
        .first()
        .map(|f| f.filename.clone())
        .unwrap_or_else(|| "item.bin".into());
    let summary = snapshot
        .summary
        .clone()
        .unwrap_or_else(|| handler.to_string());

    let mut metadata = if paths.metadata.exists() {
        let mut existing: EntryMetadata = serde_json::from_slice(&fs::read(&paths.metadata)?)?;
        existing.copy_count += 1;
        existing.last_seen = timestamp;
        existing.byte_size = snapshot.total_size();
        existing.summary = Some(summary.clone());
        existing.detected_formats = handler.detected_formats().to_vec();
        existing.sources = handler.sources();
        existing.files = snapshot.sources();
        existing.content_filename = primary.clone();
        existing.extra = merge_metadata(existing.extra, handler.to_metadata());
        existing
    } else {
        EntryMetadata {
            hash: hash.clone(),
            kind: snapshot.kind.clone(),
            detected_formats: handler.detected_formats().to_vec(),
            copy_count: 1,
            first_seen: timestamp,
            last_seen: timestamp,
            byte_size: snapshot.total_size(),
            sources: handler.sources(),
            summary: Some(summary.clone()),
            version: env!("CARGO_PKG_VERSION").to_string(),
            relative_path: relative_item_path(&paths)?,
            content_filename: primary.clone(),
            files: snapshot.sources(),
            extra: handler.to_metadata(),
        }
    };

    metadata.extra = enrich_display(metadata.extra.clone(), &summary);
    fs::write(&paths.metadata, serde_json::to_vec_pretty(&metadata)?)?;
    update_index(metadata.clone());
    Ok(metadata)
}

fn merge_metadata(existing: Value, new_value: Value) -> Value {
    match (existing, new_value) {
        (Value::Object(mut left), Value::Object(right)) => {
            for (key, value) in right {
                left.insert(key, value);
            }
            Value::Object(left)
        }
        (_, Value::Null) => Value::Null,
        (_, replacement) => replacement,
    }
}

fn enrich_display(extra: Value, summary: &str) -> Value {
    let mut map = extra.as_object().cloned().unwrap_or_default();
    map.insert("summary".into(), Value::String(summary.to_string()));
    Value::Object(map)
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

pub fn history_stream(
    index: &SearchIndex,
    limit: Option<usize>,
    query: Option<String>,
    kind: Option<crate::cli::args::EntryKind>,
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
        .filter(move |record| match (&kind, &record.kind) {
            (Some(expected), EntryKind::Text) => {
                matches!(expected, crate::cli::args::EntryKind::Text)
            }
            (Some(expected), EntryKind::Image) => {
                matches!(expected, crate::cli::args::EntryKind::Image)
            }
            (Some(expected), EntryKind::File) => {
                matches!(expected, crate::cli::args::EntryKind::File)
            }
            (Some(expected), EntryKind::Other) => {
                matches!(expected, crate::cli::args::EntryKind::Other)
            }
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

pub fn resolve_selector(index: &SearchIndex, selector: &str) -> Result<String> {
    if selector.len() >= 6 {
        if let Some(record) = index.get(selector) {
            return Ok(record.hash.clone());
        }
    }
    let offset: usize = selector
        .parse()
        .with_context(|| format!("Invalid selector {selector}"))?;
    let mut records: Vec<_> = index.values().collect();
    records.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));
    if let Some(record) = records.get(offset) {
        Ok(record.hash.clone())
    } else {
        Err(anyhow!("No entry at offset {offset}"))
    }
}

pub fn copy_by_selector(_index: &SearchIndex, hash: &str) -> Result<()> {
    let metadata = load_metadata(hash)?;
    let config = load_config()?;
    let data_dir = ensure_data_dir(&config)?;
    let item_dir = data_dir.join(&metadata.relative_path);
    let handler = handler_from_metadata(&metadata, &item_dir)?;
    let contents = handler.to_clipboard_item(&metadata, &item_dir)?;
    let ctx = ClipboardContext::new().map_err(|e| anyhow!("Failed to access clipboard: {e}"))?;
    ctx.set(contents)
        .map_err(|e| anyhow!("Failed to set clipboard: {e}"))?;
    Ok(())
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
