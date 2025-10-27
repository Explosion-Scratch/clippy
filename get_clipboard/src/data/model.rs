use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;
use time::serde::timestamp;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryMetadata {
    pub hash: String,
    pub kind: EntryKind,
    pub detected_formats: Vec<String>,
    pub copy_count: u64,
    #[serde(with = "timestamp")]
    pub first_seen: OffsetDateTime,
    #[serde(with = "timestamp")]
    pub last_seen: OffsetDateTime,
    pub byte_size: u64,
    pub sources: Vec<String>,
    pub summary: Option<String>,
    pub version: String,
    pub relative_path: String,
    pub content_filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EntryKind {
    Text,
    Image,
    File,
    Other,
}

impl EntryKind {
    pub fn from_formats(mime: &str, formats: &[String]) -> Self {
        if formats
            .iter()
            .any(|f| f.contains("public.file-url") || f.contains("file"))
        {
            EntryKind::File
        } else if formats.iter().any(|f| f.contains("public.png")) || mime.contains("image/png") {
            EntryKind::Image
        } else if formats.iter().any(|f| f.contains("public.tiff")) || mime.contains("image/tiff") {
            EntryKind::Image
        } else if formats.iter().any(|f| f.contains("image")) || mime.starts_with("image/") {
            EntryKind::Image
        } else if formats.iter().any(|f| f.contains("public.html")) || mime.contains("text/html") {
            EntryKind::Text
        } else if formats.iter().any(|f| f.contains("public.utf8-plain-text"))
            || formats.iter().any(|f| f.contains("public.text"))
            || mime.starts_with("text/")
        {
            EntryKind::Text
        } else {
            EntryKind::Other
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIndexRecord {
    pub hash: String,
    #[serde(with = "timestamp")]
    pub last_seen: OffsetDateTime,
    pub kind: EntryKind,
    pub copy_count: u64,
    pub summary: Option<String>,
    pub detected_formats: Vec<String>,
    pub byte_size: u64,
}

pub type SearchIndex = HashMap<String, SearchIndexRecord>;
