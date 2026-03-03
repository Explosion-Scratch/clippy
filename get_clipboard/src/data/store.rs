use crate::clipboard::{plugins, ClipboardSnapshot};
use crate::clipboard::plugins::PluginCapture;
use crate::config::{ensure_data_dir, load_config};
use crate::data::model::{EntryKind, EntryMetadata, JournalEntry, SearchIndex, SearchIndexRecord};
use crate::fs::layout;
pub use crate::search::SelectionFilter;
use crate::search::{SearchOptions, search};
use crate::util::time::{self, OffsetDateTime};
use anyhow::{Context, Result, anyhow};
use clipboard_rs::{Clipboard, ClipboardContext};
use image::ImageReader;
use parking_lot::RwLock;
use serde_json::{self, Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

struct SharedState {
    index: Arc<SearchIndex>,
    sorted_hashes: Vec<String>,
    sorted_valid: bool,
    journal_len: u64,
}

static STATE: parking_lot::Once = parking_lot::Once::new();
static STATE_LOCK: RwLock<Option<SharedState>> = RwLock::new(None);

const COMPACT_THRESHOLD: u64 = 500;
const MAX_SEARCH_TEXT_CHARS: usize = 65536;
const MAX_SEARCH_TEXT_SEGMENTS: usize = 4;

enum CopyCountMode {
    Increment,
    Override(u64),
}

fn init_state() {
    STATE.call_once(|| {
        let index = match load_from_journal() {
            Ok(idx) => idx,
            Err(e) => {
                eprintln!("Warning: Failed to load journal, starting fresh: {e}");
                HashMap::new()
            }
        };
        let mut guard = STATE_LOCK.write();
        *guard = Some(SharedState {
            index: Arc::new(index),
            sorted_hashes: Vec::new(),
            sorted_valid: false,
            journal_len: 0,
        });
    });
}

fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&SharedState) -> R,
{
    init_state();
    let guard = STATE_LOCK.read();
    f(guard.as_ref().expect("state initialized"))
}

fn with_state_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut SharedState) -> R,
{
    init_state();
    let mut guard = STATE_LOCK.write();
    f(guard.as_mut().expect("state initialized"))
}

pub fn load_index() -> Result<Arc<SearchIndex>> {
    Ok(with_state(|s| Arc::clone(&s.index)))
}

pub fn ensure_index() -> Result<Arc<SearchIndex>> {
    load_index()
}

pub fn sorted_hashes() -> Vec<String> {
    with_state_mut(|state| {
        if !state.sorted_valid {
            let idx = &state.index;
            let mut hashes: Vec<String> = idx.keys().cloned().collect();
            hashes.sort_by(|a, b| {
                let ra = idx.get(a).unwrap();
                let rb = idx.get(b).unwrap();
                rb.last_seen.cmp(&ra.last_seen)
            });
            state.sorted_hashes = hashes;
            state.sorted_valid = true;
        }
        state.sorted_hashes.clone()
    })
}

pub fn refresh_index() -> Result<()> {
    let new_index = load_from_journal()?;
    with_state_mut(|state| {
        state.index = Arc::new(new_index);
        state.sorted_valid = false;
    });
    Ok(())
}

fn mutate_index<F>(f: F)
where
    F: FnOnce(&mut SearchIndex),
{
    with_state_mut(|state| {
        let mut new_map = (*state.index).clone();
        f(&mut new_map);
        state.index = Arc::new(new_map);
        state.sorted_valid = false;
    });
}

// --- Journal persistence ---

fn load_from_journal() -> Result<SearchIndex> {
    let config = load_config()?;
    let data_dir = ensure_data_dir(&config)?;

    migrate_legacy_data(&data_dir)?;

    let snapshot_file = layout::snapshot_path(&data_dir);
    let journal_file = layout::journal_path(&data_dir);

    let mut index: SearchIndex = if snapshot_file.exists() {
        let bytes = fs::read(&snapshot_file)
            .context("Failed to read journal snapshot")?;
        serde_json::from_slice(&bytes)
            .context("Failed to parse journal snapshot")?
    } else {
        HashMap::new()
    };

    if journal_file.exists() {
        let file = fs::File::open(&journal_file)
            .context("Failed to open journal")?;
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<JournalEntry>(trimmed) {
                Ok(entry) => apply_journal_entry(&mut index, &entry),
                Err(e) => {
                    eprintln!("Warning: Skipping corrupt journal line: {e}");
                }
            }
        }
    }

    Ok(index)
}

fn apply_journal_entry(index: &mut SearchIndex, entry: &JournalEntry) {
    match entry {
        JournalEntry::Add { hash, .. } => {
            if let Some(record) = entry.to_record() {
                index.insert(hash.clone(), record);
            }
        }
        JournalEntry::Delete { hash } => {
            index.remove(hash);
        }
    }
}

fn append_journal(entry: &JournalEntry) {
    if let Ok(config) = load_config() {
        if let Ok(data_dir) = ensure_data_dir(&config) {
            let journal_file = layout::journal_path(&data_dir);
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&journal_file)
            {
                if let Ok(line) = serde_json::to_string(entry) {
                    let _ = writeln!(file, "{}", line);
                }
            }

            let should_compact = with_state_mut(|state| {
                state.journal_len += 1;
                state.journal_len >= COMPACT_THRESHOLD
            });
            if should_compact {
                let _ = compact_journal(&data_dir);
            }
        }
    }
}

fn compact_journal(data_dir: &Path) -> Result<()> {
    let index = with_state(|s| Arc::clone(&s.index));
    let snapshot_file = layout::snapshot_path(data_dir);
    let journal_file = layout::journal_path(data_dir);

    let tmp_snapshot = snapshot_file.with_extension("snapshot.tmp");
    let bytes = serde_json::to_vec(&*index)?;
    fs::write(&tmp_snapshot, bytes)?;
    fs::rename(&tmp_snapshot, &snapshot_file)?;

    let _ = fs::write(&journal_file, b"");

    with_state_mut(|state| {
        state.journal_len = 0;
    });
    Ok(())
}

// --- Legacy migration ---

fn migrate_legacy_data(data_dir: &Path) -> Result<()> {
    let objects_dir = layout::objects_dir(data_dir);
    if objects_dir.exists() {
        return Ok(());
    }

    let legacy_index = layout::legacy_index_path(data_dir);
    let has_legacy = legacy_index.exists() || has_year_dirs(data_dir);
    if !has_legacy {
        return Ok(());
    }

    eprintln!("Migrating clipboard data to new storage format...");

    let old_index: SearchIndex = if legacy_index.exists() {
        match fs::read(&legacy_index) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            Err(_) => HashMap::new(),
        }
    } else {
        scan_legacy_metadata(data_dir)?
    };

    let mut journal_entries = Vec::new();
    let mut migrated = 0;

    for (hash, record) in &old_index {
        let old_dir = data_dir.join(&record.relative_path);
        if !old_dir.exists() {
            continue;
        }

        let new_dir = layout::item_dir(data_dir, hash);
        if new_dir.exists() {
            continue;
        }

        if let Some(parent) = new_dir.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::rename(&old_dir, &new_dir).or_else(|_| copy_dir_recursive(&old_dir, &new_dir))?;

        let meta_path = new_dir.join("metadata.json");
        if meta_path.exists() {
            if let Ok(bytes) = fs::read(&meta_path) {
                if let Ok(mut meta) = serde_json::from_slice::<EntryMetadata>(&bytes) {
                    meta.relative_path = layout::relative_path_for_hash(hash);
                    if let Ok(new_bytes) = serde_json::to_vec_pretty(&meta) {
                        let _ = fs::write(&meta_path, new_bytes);
                    }
                }
            }
        }

        journal_entries.push(JournalEntry::from_record(record));
        migrated += 1;
    }

    if !journal_entries.is_empty() {
        let snapshot_file = layout::snapshot_path(data_dir);
        let mut new_index: SearchIndex = HashMap::new();
        for entry in &journal_entries {
            if let Some(mut record) = entry.to_record() {
                record.relative_path = layout::relative_path_for_hash(&record.hash);
                new_index.insert(record.hash.clone(), record);
            }
        }
        let bytes = serde_json::to_vec(&new_index)?;
        fs::write(&snapshot_file, bytes)?;
    }

    cleanup_legacy_dirs(data_dir);
    let _ = fs::remove_file(&legacy_index);

    eprintln!("Migration complete: {migrated} items moved to new format.");
    Ok(())
}

fn has_year_dirs(data_dir: &Path) -> bool {
    if let Ok(entries) = fs::read_dir(data_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.len() == 4 && name.chars().all(|c| c.is_ascii_digit()) && entry.path().is_dir() {
                return true;
            }
        }
    }
    false
}

fn scan_legacy_metadata(data_dir: &Path) -> Result<SearchIndex> {
    let mut index = HashMap::new();
    for year in read_dir_sorted(data_dir)? {
        let year_path = year.path();
        if !year_path.is_dir() { continue; }
        for month in read_dir_sorted(&year_path)? {
            let month_path = month.path();
            if !month_path.is_dir() { continue; }
            for first in read_dir_sorted(&month_path)? {
                let first_path = first.path();
                if !first_path.is_dir() { continue; }
                for second in read_dir_sorted(&first_path)? {
                    let second_path = second.path();
                    if !second_path.is_dir() { continue; }
                    for item in read_dir_sorted(&second_path)? {
                        let item_dir = item.path();
                        if !item_dir.is_dir() { continue; }
                        let metadata_path = item_dir.join("metadata.json");
                        if !metadata_path.exists() { continue; }
                        let bytes = match fs::read(&metadata_path) {
                            Ok(b) => b,
                            Err(_) => continue,
                        };
                        let meta: EntryMetadata = match serde_json::from_slice(&bytes) {
                            Ok(m) => m,
                            Err(_) => continue,
                        };
                        index.insert(meta.hash.clone(), SearchIndexRecord {
                            hash: meta.hash.clone(),
                            last_seen: meta.last_seen,
                            kind: meta.kind.clone(),
                            copy_count: meta.copy_count,
                            summary: meta.summary.clone(),
                            search_text: meta.search_text.clone(),
                            detected_formats: meta.detected_formats.clone(),
                            byte_size: meta.byte_size,
                            relative_path: meta.relative_path.clone(),
                        });
                    }
                }
            }
        }
    }
    Ok(index)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

fn cleanup_legacy_dirs(data_dir: &Path) {
    if let Ok(entries) = fs::read_dir(data_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.len() == 4 && name.chars().all(|c| c.is_ascii_digit()) && entry.path().is_dir() {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }
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

// --- Core storage operations ---

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
    let data_dir = ensure_data_dir(&config)?;
    let item_dir = layout::item_dir(&data_dir, hash);
    layout::ensure_dir(&item_dir)?;

    let mut wrote_file = false;
    for capture in plugin_captures {
        for output in &capture.files {
            let dest = item_dir.join(&output.filename);
            if let Some(parent) = dest.parent() {
                layout::ensure_dir(parent)?;
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
    let relative_path = layout::relative_path_for_hash(hash);
    let metadata_path = item_dir.join("metadata.json");

    let metadata = if metadata_path.exists() {
        let mut existing: EntryMetadata = serde_json::from_slice(&fs::read(&metadata_path)?)?;
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
        existing.relative_path = relative_path;
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
            relative_path,
            content_filename: primary.clone(),
            files: combined_sources.clone(),
            extra: extra.clone(),
        }
    };

    fs::write(&metadata_path, serde_json::to_vec_pretty(&metadata)?)?;

    let record = SearchIndexRecord {
        hash: metadata.hash.clone(),
        last_seen: metadata.last_seen,
        kind: metadata.kind.clone(),
        copy_count: metadata.copy_count,
        summary: metadata.summary.clone(),
        search_text: metadata.search_text.clone(),
        detected_formats: metadata.detected_formats.clone(),
        byte_size: metadata.byte_size,
        relative_path: metadata.relative_path.clone(),
    };
    let journal_entry = JournalEntry::from_record(&record);
    mutate_index(|idx| {
        idx.insert(record.hash.clone(), record);
    });
    append_journal(&journal_entry);

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

// --- Query operations ---

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

/// O(1) metadata lookup — constructs path directly from hash
pub fn load_metadata(hash: &str) -> Result<EntryMetadata> {
    let config = load_config()?;
    let data_dir = ensure_data_dir(&config)?;
    let meta_path = layout::metadata_path(&data_dir, hash);

    if meta_path.exists() {
        let meta: EntryMetadata = serde_json::from_slice(&fs::read(&meta_path)?)
            .with_context(|| format!("Failed to parse metadata for {hash}"))?;
        return Ok(meta);
    }

    Err(anyhow!("Metadata not found for {hash}"))
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

pub fn copy_plain_by_selector(hash: &str) -> Result<EntryMetadata> {
    let metadata = load_metadata(hash)?;
    let config = load_config()?;
    let data_dir = ensure_data_dir(&config)?;
    let item_dir = data_dir.join(&metadata.relative_path);

    let (order, map) = match plugins::extract_plugin_meta(&metadata)? {
        Some(x) => x,
        None => return Err(anyhow!("No plugin metadata found")),
    };

    let has_text = order.iter().any(|id| id == "text");
    let has_image = order.iter().any(|id| id == "image");
    let has_html = order.iter().any(|id| id == "html");
    let has_rtf = order.iter().any(|id| id == "rtf");
    let has_files = order.iter().any(|id| id == "files");

    if has_image && !has_text && !has_html && !has_rtf && !has_files {
        let contents = plugins::rebuild_clipboard_contents(&metadata, &item_dir)?;
        let ctx =
            ClipboardContext::new().map_err(|e| anyhow!("Failed to access clipboard: {e}"))?;
        ctx.set(contents)
            .map_err(|e| anyhow!("Failed to set clipboard: {e}"))?;
        return Ok(metadata);
    }

    let mut text_content: Option<String> = None;

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

    if text_content.is_none() && has_files {
        if let Some(plugin_meta) = map.get("files") {
            if let Some(entries) = plugin_meta.get("entries").and_then(|v| v.as_array()) {
                let paths: Vec<String> = entries
                    .iter()
                    .filter_map(|entry| {
                        entry
                            .get("source_path")
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

    if let Some(text) = text_content {
        let ctx =
            ClipboardContext::new().map_err(|e| anyhow!("Failed to access clipboard: {e}"))?;
        ctx.set_text(text)
            .map_err(|e| anyhow!("Failed to set clipboard text: {e}"))?;
        return Ok(metadata);
    }

    let contents = plugins::rebuild_clipboard_contents(&metadata, &item_dir)?;
    let ctx = ClipboardContext::new().map_err(|e| anyhow!("Failed to access clipboard: {e}"))?;
    ctx.set(contents)
        .map_err(|e| anyhow!("Failed to set clipboard: {e}"))?;
    Ok(metadata)
}

pub fn delete_entry(hash: &str) -> Result<()> {
    let config = load_config()?;
    let data_dir = ensure_data_dir(&config)?;
    let item_dir = layout::item_dir(&data_dir, hash);
    if item_dir.exists() {
        fs::remove_dir_all(&item_dir)?;
    }
    mutate_index(|idx| {
        idx.remove(hash);
    });
    append_journal(&JournalEntry::delete(hash));
    Ok(())
}

pub fn increment_copy_count(hash: &str) -> Result<EntryMetadata> {
    let mut metadata = load_metadata(hash)?;
    metadata.copy_count = metadata.copy_count.saturating_add(1);
    let config = load_config()?;
    let data_dir = ensure_data_dir(&config)?;
    let item_dir = layout::item_dir(&data_dir, hash);
    let metadata_path = item_dir.join("metadata.json");
    fs::write(&metadata_path, serde_json::to_vec_pretty(&metadata)?)?;

    let record = SearchIndexRecord {
        hash: metadata.hash.clone(),
        last_seen: metadata.last_seen,
        kind: metadata.kind.clone(),
        copy_count: metadata.copy_count,
        summary: metadata.summary.clone(),
        search_text: metadata.search_text.clone(),
        detected_formats: metadata.detected_formats.clone(),
        byte_size: metadata.byte_size,
        relative_path: metadata.relative_path.clone(),
    };
    let journal_entry = JournalEntry::from_record(&record);
    mutate_index(|idx| {
        idx.insert(record.hash.clone(), record);
    });
    append_journal(&journal_entry);
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
    Read::by_ref(&mut file)
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
