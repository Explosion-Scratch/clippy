use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItem {
    pub id: u64,
    pub text: Option<String>, // Searchable text field (can be null)
    pub timestamp: u64,
    pub byte_size: usize,
    pub formats: ClipboardFormats,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardFormats {
    pub txt: Option<String>,
    pub html: Option<String>,
    pub rtf: Option<String>,
    pub image_data: Option<String>, // Base64 encoded PNG
    pub files: Option<Vec<String>>, // List of file paths
    pub custom_formats: Option<std::collections::HashMap<String, String>>, // Other formats as strings
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardChangeEvent {
    pub item: ClipboardItem,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DatabaseItem {
    pub id: u64,
    pub text: Option<String>,
    pub timestamp: u64,
    pub byte_size: usize,
    pub formats: ClipboardFormats,
}

impl From<ClipboardItem> for DatabaseItem {
    fn from(item: ClipboardItem) -> Self {
        DatabaseItem {
            id: item.id,
            text: item.text,
            timestamp: item.timestamp,
            byte_size: item.byte_size,
            formats: item.formats,
        }
    }
}

impl From<DatabaseItem> for ClipboardItem {
    fn from(item: DatabaseItem) -> Self {
        ClipboardItem {
            id: item.id,
            text: item.text,
            timestamp: item.timestamp,
            byte_size: item.byte_size,
            formats: item.formats,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SaveResult {
    pub success: bool,
    pub id: Option<u64>,
    pub error: Option<String>,
}
