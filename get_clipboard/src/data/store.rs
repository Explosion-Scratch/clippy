use crate::config::{ensure_data_dir, load_config};
use crate::data::model::{EntryKind, EntryMetadata, SearchIndex, SearchIndexRecord};
use crate::fs::{EntryPaths, entry_paths};
use crate::util::time::OffsetDateTime;
use crate::util::{hash, time};
use anyhow::{Context, Result, anyhow};
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ClipboardEntry {
    pub metadata: EntryMetadata,
    pub content_path: ContentPath,
}

#[derive(Debug, Clone)]
pub enum ContentPath {
    Text(PathBuf),
    Binary(PathBuf),
    FilePath(PathBuf),
}

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
    let cell = index_cell();
    Ok(cell.read().clone())
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

pub fn store_text(content: &str, formats: &[String]) -> Result<ClipboardEntry> {
    persist_entry(
        content.as_bytes(),
        "text/plain",
        formats,
        ContentFlavor::Text(content.to_string()),
    )
}

pub fn store_binary(data: &[u8], mime: &str, formats: &[String]) -> Result<ClipboardEntry> {
    persist_entry(data, mime, formats, ContentFlavor::Binary)
}

pub fn store_file_path(path: &Path, formats: &[String]) -> Result<ClipboardEntry> {
    let data =
        fs::read(path).with_context(|| format!("Failed to read file at {}", path.display()))?;
    persist_entry(
        &data,
        "application/octet-stream",
        formats,
        ContentFlavor::File(path.to_path_buf()),
    )
}

enum ContentFlavor {
    Text(String),
    Binary,
    File(PathBuf),
}

fn persist_entry(
    bytes: &[u8],
    mime: &str,
    formats: &[String],
    flavor: ContentFlavor,
) -> Result<ClipboardEntry> {
    let hash = hash::sha256_bytes(bytes);
    let config = load_config()?;
    let timestamp = time::now();
    let ext = crate::fs::layout::determine_extension(mime);
    let paths = entry_paths(&config, &hash, timestamp, ext)?;
    crate::fs::layout::ensure_dir(&paths.item_dir)?;
    if !paths.metadata.exists() {
        let mut file = fs::File::create(&paths.content).with_context(|| {
            format!(
                "Failed to create content file at {}",
                paths.content.display()
            )
        })?;
        file.write_all(bytes)?;
        let detected_formats = formats.to_vec();
        let kind = EntryKind::from_formats(mime, &detected_formats);
        let entry = ClipboardEntry {
            metadata: EntryMetadata {
                hash: hash.clone(),
                kind: kind.clone(),
                detected_formats: detected_formats.clone(),
                copy_count: 1,
                first_seen: timestamp,
                last_seen: timestamp,
                byte_size: bytes.len() as u64,
                sources: vec![],
                summary: match flavor {
                    ContentFlavor::Text(ref text) => Some(text.chars().take(120).collect()),
                    ContentFlavor::File(ref path) => Some(path.display().to_string()),
                    ContentFlavor::Binary => None,
                },
                version: env!("CARGO_PKG_VERSION").to_string(),
                relative_path: relative_item_path(&paths)?,
                content_filename: paths
                    .content
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "item.bin".into()),
            },
            content_path: match flavor {
                ContentFlavor::Text(_) => ContentPath::Text(paths.content.clone()),
                ContentFlavor::Binary => ContentPath::Binary(paths.content.clone()),
                ContentFlavor::File(_) => ContentPath::FilePath(paths.content.clone()),
            },
        };
        let json = serde_json::to_vec_pretty(&entry.metadata)?;
        fs::write(&paths.metadata, json)?;
        update_index(entry.metadata.clone());
        Ok(entry)
    } else {
        let mut metadata: EntryMetadata = serde_json::from_slice(&fs::read(&paths.metadata)?)?;
        metadata.copy_count += 1;
        metadata.last_seen = timestamp;
        metadata.kind = EntryKind::from_formats(mime, &metadata.detected_formats);
        if let ContentFlavor::Text(ref text) = flavor {
            metadata.summary = Some(text.chars().take(120).collect());
        }
        fs::write(&paths.metadata, serde_json::to_vec_pretty(&metadata)?)?;
        update_index(metadata.clone());
        Ok(ClipboardEntry {
            metadata,
            content_path: match flavor {
                ContentFlavor::Text(_) => ContentPath::Text(paths.content.clone()),
                ContentFlavor::Binary => ContentPath::Binary(paths.content.clone()),
                ContentFlavor::File(_) => ContentPath::FilePath(paths.content.clone()),
            },
        })
    }
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

fn summarize_kind(kind: EntryKind, byte_size: u64) -> String {
    match kind {
        EntryKind::Image => format!("[Image {:.1}KB]", byte_size as f64 / 1024.0),
        EntryKind::File => format!("[File {:.1}KB]", byte_size as f64 / 1024.0),
        EntryKind::Text => String::from("(text item)"),
        EntryKind::Other => String::from("(binary item)"),
    }
}

pub struct HistoryItem {
    pub summary: String,
    pub kind: String,
    pub metadata: EntryMetadata,
}

pub fn history_stream(
    index: &SearchIndex,
    limit: Option<usize>,
    query: Option<String>,
    kind: Option<crate::cli::args::EntryKind>,
    from: Option<OffsetDateTime>,
    to: Option<OffsetDateTime>,
) -> Result<impl Iterator<Item = HistoryItem>> {
    let mut entries: Vec<_> = index
        .values()
        .filter(|record| match (&from, &to) {
            (Some(start), Some(end)) => record.last_seen >= *start && record.last_seen <= *end,
            (Some(start), None) => record.last_seen >= *start,
            (None, Some(end)) => record.last_seen <= *end,
            (None, None) => true,
        })
        .collect();
    entries.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));

    let iter = entries
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
        .filter_map(|record| match load_metadata(&record.hash) {
            Ok(metadata) => Some(HistoryItem {
                summary: record
                    .summary
                    .clone()
                    .unwrap_or_else(|| summarize_kind(record.kind.clone(), record.byte_size)),
                kind: format!("{:?}", record.kind),
                metadata,
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
    let text_path = content_path(&metadata);
    if let Some(path) = text_path {
        let bytes = fs::read(&path)?;
        crate::clipboard::mac::set_clipboard_from_bytes(&bytes, &metadata.detected_formats)
    } else {
        Err(anyhow!("Unsupported content type"))
    }
}

fn content_path(metadata: &EntryMetadata) -> Option<PathBuf> {
    let config = load_config().ok()?;
    let data_dir = config.data_dir();
    Some(
        data_dir
            .join(&metadata.relative_path)
            .join(&metadata.content_filename),
    )
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
