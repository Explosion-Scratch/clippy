use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItem {
    pub id: u64,
    pub text: Option<String>, // Searchable text field (can be null)
    pub timestamp: u64, // Last copied timestamp
    pub first_copied: u64, // First time this content was copied
    pub copies: u64, // Number of times this content has been copied
    pub byte_size: u64,
    pub content_hash: String, // Hash based solely on content data
    pub formats: ClipboardFormats,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardFormats {
    pub txt: Option<String>,
    pub html: Option<String>,
    pub rtf: Option<String>,
    pub image_data: Option<String>, // Base64 encoded PNG (full resolution)
    pub image_preview: Option<String>, // Base64 encoded PNG (120px height preview)
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
    pub timestamp: u64, // Last copied timestamp
    pub first_copied: u64, // First time this content was copied
    pub copies: u64, // Number of times this content has been copied
    pub byte_size: u64,
    pub content_hash: String, // Hash based solely on content data
    pub formats: ClipboardFormats,
}

impl From<ClipboardItem> for DatabaseItem {
    fn from(item: ClipboardItem) -> Self {
        DatabaseItem {
            id: item.id,
            text: item.text,
            timestamp: item.timestamp,
            first_copied: item.first_copied,
            copies: item.copies,
            byte_size: item.byte_size,
            content_hash: item.content_hash,
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
            first_copied: item.first_copied,
            copies: item.copies,
            byte_size: item.byte_size,
            content_hash: item.content_hash,
            formats: item.formats,
        }
    }
}

/// Generate a hash based solely on the content data (text, image data, or file paths)
/// This hash will be the same for identical content regardless of when/where it was copied
pub fn hash_content(formats: &ClipboardFormats) -> Result<String, Box<dyn std::error::Error>> {
    let mut hasher = Sha256::new();
    
    // Include text content if available
    if let Some(txt) = &formats.txt {
        hasher.update(txt.as_bytes());
    }
    
    // Include HTML content if available
    if let Some(html) = &formats.html {
        hasher.update(html.as_bytes());
    }
    
    // Include RTF content if available
    if let Some(rtf) = &formats.rtf {
        hasher.update(rtf.as_bytes());
    }
    
    // Include image data if available (extract raw bytes from data URI)
    if let Some(image_data) = &formats.image_data {
        // Parse data URI: data:image/png;base64,xxxxx
        if let Some(base64_data) = image_data.strip_prefix("data:image/png;base64,") {
            match general_purpose::STANDARD.decode(base64_data) {
                Ok(image_bytes) => {
                    hasher.update(&image_bytes);
                }
                Err(_) => {
                    // If decoding fails, include the raw string as fallback
                    hasher.update(image_data.as_bytes());
                }
            }
        } else {
            // Not a standard data URI, include raw string
            hasher.update(image_data.as_bytes());
        }
    }
    
    // Include file paths if available (sorted for consistency)
    if let Some(files) = &formats.files {
        let mut sorted_files = files.clone();
        sorted_files.sort();
        for file_path in sorted_files {
            hasher.update(file_path.as_bytes());
        }
    }
    
    // Include custom formats if available (sorted by key for consistency)
    if let Some(custom_formats) = &formats.custom_formats {
        let mut sorted_keys: Vec<String> = custom_formats.keys().cloned().collect();
        sorted_keys.sort();
        
        for key in sorted_keys {
            if let Some(value) = custom_formats.get(&key) {
                hasher.update(key.as_bytes());
                hasher.update(value.as_bytes());
            }
        }
    }
    
    // Return hex-encoded hash
    Ok(hex::encode(hasher.finalize()))
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SaveResult {
    pub success: bool,
    pub id: Option<u64>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structs::{ClipboardFormats, hash_content};

    #[test]
    fn test_hash_text_content() {
        let formats = ClipboardFormats {
            txt: Some("Hello, World!".to_string()),
            html: None,
            rtf: None,
            image_data: None,
            image_preview: None,
            files: None,
            custom_formats: None,
        };

        let hash1 = hash_content(&formats).unwrap();
        let hash2 = hash_content(&formats).unwrap();
        
        // Same content should produce same hash
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex length
    }

    #[test]
    fn test_hash_different_text() {
        let formats1 = ClipboardFormats {
            txt: Some("Hello, World!".to_string()),
            html: None,
            rtf: None,
            image_data: None,
            files: None,
            custom_formats: None,
        };

        let formats2 = ClipboardFormats {
            txt: Some("Hello, Universe!".to_string()),
            html: None,
            rtf: None,
            image_data: None,
            image_preview: None,
            files: None,
            custom_formats: None,
        };

        let hash1 = hash_content(&formats1).unwrap();
        let hash2 = hash_content(&formats2).unwrap();
        
        // Different content should produce different hashes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_image_data() {
        let base64_image = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
        let data_uri = format!("data:image/png;base64,{}", base64_image);

        let formats = ClipboardFormats {
            txt: None,
            html: None,
            rtf: None,
            image_data: Some(data_uri),
            image_preview: None,
            files: None,
            custom_formats: None,
        };

        let hash1 = hash_content(&formats).unwrap();
        let hash2 = hash_content(&formats).unwrap();
        
        // Same image should produce same hash
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_files() {
        let files1 = vec!["/path/to/file1.txt".to_string(), "/path/to/file2.txt".to_string()];
        let files2 = vec!["/path/to/file2.txt".to_string(), "/path/to/file1.txt".to_string()]; // Different order

        let formats1 = ClipboardFormats {
            txt: None,
            html: None,
            rtf: None,
            image_data: None,
            image_preview: None,
            files: Some(files1),
            custom_formats: None,
        };

        let formats2 = ClipboardFormats {
            txt: None,
            html: None,
            rtf: None,
            image_data: None,
            image_preview: None,
            files: Some(files2),
            custom_formats: None,
        };

        let hash1 = hash_content(&formats1).unwrap();
        let hash2 = hash_content(&formats2).unwrap();
        
        // Same files in different order should produce same hash (sorted internally)
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_mixed_content() {
        let formats1 = ClipboardFormats {
            txt: Some("Sample text".to_string()),
            html: Some("<p>Sample HTML</p>".to_string()),
            rtf: None,
            image_data: None,
            image_preview: None,
            files: None,
            custom_formats: None,
        };

        let formats2 = ClipboardFormats {
            txt: Some("Sample text".to_string()),
            html: Some("<p>Sample HTML</p>".to_string()),
            rtf: None,
            image_data: None,
            image_preview: None,
            files: None,
            custom_formats: None,
        };

        let hash1 = hash_content(&formats1).unwrap();
        let hash2 = hash_content(&formats2).unwrap();
        
        // Same mixed content should produce same hash
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_empty_content() {
        let formats = ClipboardFormats::default();

        let hash1 = hash_content(&formats).unwrap();
        let hash2 = hash_content(&formats).unwrap();
        
        // Empty content should produce consistent hash
        assert_eq!(hash1, hash2);
    }
}
