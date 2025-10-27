use crate::config::AppConfig;
use crate::util::time::OffsetDateTime;
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct EntryPaths {
    pub base_dir: PathBuf,
    pub item_dir: PathBuf,
    pub metadata: PathBuf,
    pub content: PathBuf,
}

pub fn entry_paths(
    config: &AppConfig,
    hash: &str,
    timestamp: OffsetDateTime,
    ext: Option<&str>,
) -> Result<EntryPaths> {
    let data_dir = config.data_dir();
    let year = timestamp.year();
    let month = timestamp.month() as u8;
    let chars: Vec<char> = hash.chars().collect();
    let first = chars.get(0).copied().unwrap_or('0');
    let next_two: String = chars.get(1..3).unwrap_or(&[]).iter().collect();
    let rest: String = chars.get(3..).unwrap_or(&[]).iter().collect();
    let item_dir = data_dir
        .join(format!("{year:04}"))
        .join(format!("{month:02}"))
        .join(first.to_string())
        .join(next_two)
        .join(rest);
    let metadata = item_dir.join("metadata.json");
    let content = item_dir.join(match ext {
        Some(extension) => format!("item.{extension}"),
        None => "item.bin".to_string(),
    });
    Ok(EntryPaths {
        base_dir: data_dir,
        item_dir,
        metadata,
        content,
    })
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
