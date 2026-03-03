use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    #[serde(default)]
    pub search_text: Option<String>,
    pub version: String,
    pub relative_path: String,
    pub content_filename: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub extra: Value,
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
    #[serde(default)]
    pub search_text: Option<String>,
    pub detected_formats: Vec<String>,
    pub byte_size: u64,
    pub relative_path: String,
}

pub type SearchIndex = HashMap<String, SearchIndexRecord>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum JournalEntry {
    #[serde(rename = "add")]
    Add {
        hash: String,
        #[serde(with = "timestamp")]
        last_seen: OffsetDateTime,
        kind: EntryKind,
        copy_count: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        search_text: Option<String>,
        detected_formats: Vec<String>,
        byte_size: u64,
    },
    #[serde(rename = "del")]
    Delete {
        hash: String,
    },
}

impl JournalEntry {
    pub fn from_record(record: &SearchIndexRecord) -> Self {
        JournalEntry::Add {
            hash: record.hash.clone(),
            last_seen: record.last_seen,
            kind: record.kind.clone(),
            copy_count: record.copy_count,
            summary: record.summary.clone(),
            search_text: record.search_text.clone(),
            detected_formats: record.detected_formats.clone(),
            byte_size: record.byte_size,
        }
    }

    pub fn delete(hash: &str) -> Self {
        JournalEntry::Delete {
            hash: hash.to_string(),
        }
    }

    pub fn to_record(&self) -> Option<SearchIndexRecord> {
        match self {
            JournalEntry::Add {
                hash,
                last_seen,
                kind,
                copy_count,
                summary,
                search_text,
                detected_formats,
                byte_size,
            } => Some(SearchIndexRecord {
                hash: hash.clone(),
                last_seen: *last_seen,
                kind: kind.clone(),
                copy_count: *copy_count,
                summary: summary.clone(),
                search_text: search_text.clone(),
                detected_formats: detected_formats.clone(),
                byte_size: *byte_size,
                relative_path: crate::fs::layout::relative_path_for_hash(hash),
            }),
            JournalEntry::Delete { .. } => None,
        }
    }

    pub fn hash(&self) -> &str {
        match self {
            JournalEntry::Add { hash, .. } => hash,
            JournalEntry::Delete { hash } => hash,
        }
    }
}
