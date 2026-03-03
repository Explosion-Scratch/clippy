use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn objects_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("objects")
}

pub fn item_dir(data_dir: &Path, hash: &str) -> PathBuf {
    objects_dir(data_dir)
        .join(&hash[..2])
        .join(&hash[2..4])
        .join(hash)
}

pub fn metadata_path(data_dir: &Path, hash: &str) -> PathBuf {
    item_dir(data_dir, hash).join("metadata.json")
}

pub fn relative_path_for_hash(hash: &str) -> String {
    format!("objects/{}/{}/{}", &hash[..2], &hash[2..4], hash)
}

pub fn journal_path(data_dir: &Path) -> PathBuf {
    data_dir.join("journal.jsonl")
}

pub fn snapshot_path(data_dir: &Path) -> PathBuf {
    data_dir.join("journal.snapshot")
}

pub fn legacy_index_path(data_dir: &Path) -> PathBuf {
    data_dir.join("index.json")
}

#[derive(Debug, Clone)]
pub struct EntryPaths {
    pub base_dir: PathBuf,
    pub item_dir: PathBuf,
    pub metadata: PathBuf,
    pub content: PathBuf,
}

pub fn entry_paths_for_hash(data_dir: &Path, hash: &str, content_filename: &str) -> EntryPaths {
    let dir = item_dir(data_dir, hash);
    let metadata = dir.join("metadata.json");
    let content = dir.join(content_filename);
    EntryPaths {
        base_dir: data_dir.to_path_buf(),
        item_dir: dir,
        metadata,
        content,
    }
}

pub fn determine_extension(content_type: &str) -> Option<&'static str> {
    match content_type {
        "text/plain" => Some("txt"),
        "text/html" => Some("html"),
        "text/rtf" => Some("rtf"),
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/gif" => Some("gif"),
        "image/tiff" => Some("tiff"),
        _ => None,
    }
}

pub fn ensure_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)?;
    Ok(())
}
