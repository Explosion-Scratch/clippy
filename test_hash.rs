// Add these tests to src-tauri/src/lib.rs or src-tauri/src/structs.rs
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
            files: Some(files1),
            custom_formats: None,
        };

        let formats2 = ClipboardFormats {
            txt: None,
            html: None,
            rtf: None,
            image_data: None,
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
            files: None,
            custom_formats: None,
        };

        let formats2 = ClipboardFormats {
            txt: Some("Sample text".to_string()),
            html: Some("<p>Sample HTML</p>".to_string()),
            rtf: None,
            image_data: None,
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