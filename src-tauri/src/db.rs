use crate::structs::{ClipboardItem, DatabaseItem, SaveResult};
use sled::{Db, Tree};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};
use anyhow::Result;

// Database tree names
const ITEMS_TREE: &str = "items";
const TEXT_INDEX_TREE: &str = "text_index";
const TIMESTAMP_INDEX_TREE: &str = "timestamp_index";

// Global ID counter
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

// Global database instance (singleton)
static DB_INSTANCE: Mutex<Option<Arc<ClipboardDatabase>>> = Mutex::new(None);

pub struct ClipboardDatabase {
    db: Db,
    items_tree: Tree,
    text_index_tree: Tree,
    timestamp_index_tree: Tree,
}

impl ClipboardDatabase {
    /// Initialize the database with the given path
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        // Get app data directory
        let app_data_dir = app_handle.path().app_data_dir()?;
        std::fs::create_dir_all(&app_data_dir)?;

        // Create database path
        let db_path = app_data_dir.join("clipboard_db");

        // Open sled database
        let db = sled::open(&db_path)?;

        // Open trees
        let items_tree = db.open_tree(ITEMS_TREE)?;
        let text_index_tree = db.open_tree(TEXT_INDEX_TREE)?;
        let timestamp_index_tree = db.open_tree(TIMESTAMP_INDEX_TREE)?;

        // Initialize next_id from database
        if let Some(max_id_bytes_result) = items_tree.iter().keys().rev().next() {
            if let Ok(max_id_bytes) = max_id_bytes_result {
                if let Ok(max_id_str) = String::from_utf8(max_id_bytes.to_vec()) {
                    if let Ok(max_id) = max_id_str.parse::<u64>() {
                        NEXT_ID.store(max_id + 1, Ordering::Relaxed);
                    }
                }
            }
        }

        Ok(ClipboardDatabase {
            db,
            items_tree,
            text_index_tree,
            timestamp_index_tree,
        })
    }

    /// Get the global database instance (singleton pattern)
    pub fn get_instance(app_handle: &AppHandle) -> Result<Arc<Self>> {
        // Use Once to ensure initialization happens only once
        let mut db_instance = DB_INSTANCE.lock().unwrap();

        if let Some(ref db) = *db_instance {
            return Ok(Arc::clone(db));
        }

        // Initialize the database
        let db = Self::new(app_handle)?;
        let arc_db = Arc::new(db);
        *db_instance = Some(Arc::clone(&arc_db));
        Ok(arc_db)
    }

    /// Save a clipboard item to the database
    pub fn save_item(&self, item: ClipboardItem) -> SaveResult {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let mut db_item = DatabaseItem::from(item);
        db_item.id = id;

        // Serialize the item
        let serialized = match serde_json::to_vec(&db_item) {
            Ok(data) => data,
            Err(e) => return SaveResult {
                success: false,
                id: None,
                error: Some(format!("Failed to serialize item: {}", e)),
            },
        };

        let id_key = id.to_string();

        // Start a transaction
        let tx_result: Result<(), sled::transaction::TransactionError> = self.db.transaction(|db| {
            // Save the main item
            db.insert(id_key.as_bytes(), serialized.clone())?;

            // Create text index if searchable text exists
            if let Some(ref text) = db_item.text {
                if !text.is_empty() {
                    let lower_text = text.to_lowercase();
                    let index_key = format!("{}:{}", lower_text, id);
                    db.insert(index_key.as_bytes(), b"")?;
                }
            }

            // Create timestamp index for fast recent queries
            let timestamp_key = format!("{:020}:{}", db_item.timestamp, id);
            db.insert(timestamp_key.as_bytes(), b"")?;

            Ok(())
        });

        match tx_result {
            Ok(_) => {
                // Log the saved item ID
                println!("Saved item with ID: {}", id);

                // Log the total item count
                match self.get_count() {
                    Ok(count) => println!("Total item count after save: {}", count),
                    Err(e) => println!("Failed to get item count after save: {}", e),
                }

                SaveResult {
                    success: true,
                    id: Some(id),
                    error: None,
                }
            },
            Err(e) => SaveResult {
                success: false,
                id: Some(id),
                error: Some(format!("Transaction failed: {:?}", e)),
            },
        }
    }


    /// Get recent items with pagination
    pub fn recent_items(&self, count: usize, offset: usize) -> Result<Vec<ClipboardItem>> {
        println!("Getting recent items {count}, off {offset}");
        let mut items = Vec::with_capacity(count);
        let mut collected = 0;

        // Iterate timestamp index in reverse (newest first)
        let iter = self.timestamp_index_tree
            .iter()
            .rev()
            .skip(offset);

        for item_result in iter {
            if collected >= count {
                break;
            }

            let (key, _) = item_result?;
            let key_str = String::from_utf8(key.to_vec())?;

            // Extract ID from timestamp key (format: "{timestamp}:{id}")
            if let Some(id_str) = key_str.split(':').nth(1) {
                if let Ok(_id) = id_str.parse::<u64>() {
                    if let Some(item_bytes) = self.items_tree.get(id_str.as_bytes())? {
                        if let Ok(db_item) = serde_json::from_slice::<DatabaseItem>(&item_bytes) {
                            items.push(ClipboardItem::from(db_item));
                            collected += 1;
                        }
                    }
                }
            }
        }

        Ok(items)
    }

    /// Search items by text content
    pub fn search(&self, query: &str, count: usize) -> Result<Vec<ClipboardItem>> {
        let mut items = Vec::with_capacity(count);
        let query_lower = query.to_lowercase();
        let mut collected = 0;

        // Search through text index
        let prefix = format!("{}:", query_lower);

        for item_result in self.text_index_tree.scan_prefix(prefix.as_bytes()) {
            if collected >= count {
                break;
            }

            let (key, _) = item_result?;
            let key_str = String::from_utf8(key.to_vec())?;

            // Extract ID from index key (format: "{text}:{id}")
            if let Some(id_str) = key_str.split(':').nth(1) {
                if let Ok(_id) = id_str.parse::<u64>() {
                    if let Some(item_bytes) = self.items_tree.get(id_str.as_bytes())? {
                        if let Ok(db_item) = serde_json::from_slice::<DatabaseItem>(&item_bytes) {
                            items.push(ClipboardItem::from(db_item));
                            collected += 1;
                        }
                    }
                }
            }
        }

        Ok(items)
    }

    /// Delete an item by ID
    pub fn delete_item(&self, id: u64) -> SaveResult {
        let id_str = id.to_string();

        // First, get the item to extract text for index cleanup
        let item = match self.items_tree.get(id_str.as_bytes()) {
            Ok(Some(item_bytes)) => {
                match serde_json::from_slice::<DatabaseItem>(&item_bytes) {
                    Ok(item) => item,
                    Err(e) => return SaveResult {
                        success: false,
                        id: Some(id),
                        error: Some(format!("Failed to deserialize item: {}", e)),
                    },
                }
            }
            Ok(None) => return SaveResult {
                success: false,
                id: Some(id),
                error: Some("Item not found".to_string()),
            },
            Err(e) => return SaveResult {
                success: false,
                id: Some(id),
                error: Some(format!("Failed to get item: {}", e)),
            },
        };

        // Start a transaction to delete the item and its indexes
        let tx_result: Result<(), sled::transaction::TransactionError> = self.db.transaction(|db| {
            // Delete the main item
            db.remove(id_str.as_bytes())?;

            // Delete text index if searchable text exists
            if let Some(ref text) = item.text {
                if !text.is_empty() {
                    let lower_text = text.to_lowercase();
                    let index_key = format!("{}:{}", lower_text, id);
                    db.remove(index_key.as_bytes())?;
                }
            }

            // Delete timestamp index
            let timestamp_key = format!("{:020}:{}", item.timestamp, id);
            db.remove(timestamp_key.as_bytes())?;

            Ok(())
        });

        match tx_result {
            Ok(_) => SaveResult {
                success: true,
                id: Some(id),
                error: None,
            },
            Err(e) => SaveResult {
                success: false,
                id: Some(id),
                error: Some(format!("Delete transaction failed: {:?}", e)),
            },
        }
    }

    /// Get total count of items
    pub fn get_count(&self) -> Result<usize> {
        Ok(self.items_tree.len())
    }

    /// Flush all pending writes to disk
    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }
}

// Tauri commands for database operations
#[tauri::command]
pub fn db_save_item(
    app_handle: AppHandle,
    item: ClipboardItem,
) -> Result<SaveResult, String> {
    println!("Saving item");
    let db = ClipboardDatabase::get_instance(&app_handle)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    Ok(db.save_item(item))
}

#[tauri::command]
pub fn db_recent_items(
    app_handle: AppHandle,
    count: usize,
    offset: usize,
) -> Result<Vec<ClipboardItem>, String> {
    let db = ClipboardDatabase::get_instance(&app_handle)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    db.recent_items(count, offset)
        .map_err(|e| format!("Failed to get recent items: {}", e))
}

#[tauri::command]
pub fn db_search(
    app_handle: AppHandle,
    query: String,
    count: usize,
) -> Result<Vec<ClipboardItem>, String> {
    let db = ClipboardDatabase::get_instance(&app_handle)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    db.search(&query, count)
        .map_err(|e| format!("Failed to search items: {}", e))
}

#[tauri::command]
pub fn db_delete_item(
    app_handle: AppHandle,
    id: u64,
) -> Result<SaveResult, String> {
    let db = ClipboardDatabase::get_instance(&app_handle)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    Ok(db.delete_item(id))
}

#[tauri::command]
pub fn db_get_count(app_handle: AppHandle) -> Result<usize, String> {
    let db = ClipboardDatabase::get_instance(&app_handle)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    db.get_count()
        .map_err(|e| format!("Failed to get item count: {}", e))
}
