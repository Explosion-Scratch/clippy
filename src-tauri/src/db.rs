use crate::structs::{ClipboardItem, DatabaseItem, SaveResult};
use rusqlite::{Connection, params};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};
use anyhow::Result;

// Global database instance (singleton)
static DB_INSTANCE: Mutex<Option<Arc<Mutex<ClipboardDatabase>>>> = Mutex::new(None);

pub struct ClipboardDatabase {
    conn: Connection,
}

impl ClipboardDatabase {
    /// Initialize the database with the given path
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        // Get app data directory
        let app_data_dir = app_handle.path().app_data_dir()?;
        std::fs::create_dir_all(&app_data_dir)?;

        // Create database path
        let db_path = app_data_dir.join("clipboard.db");

        // Open SQLite database
        let conn = Connection::open(&db_path)?;

        let db = ClipboardDatabase { conn };

        // Initialize database schema
        db.init_schema()?;

        Ok(db)
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT,
                timestamp INTEGER NOT NULL,
                byte_size INTEGER NOT NULL,
                formats TEXT NOT NULL
            )",
            [],
        )?;

        // Create indexes for performance
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_items_timestamp ON items(timestamp)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_items_text ON items(text)",
            [],
        )?;

        Ok(())
    }

    /// Get the global database instance (singleton pattern)
    pub fn get_instance(app_handle: &AppHandle) -> Result<Arc<Mutex<Self>>> {
        let mut db_instance = DB_INSTANCE.lock().unwrap();

        if let Some(ref db) = *db_instance {
            return Ok(Arc::clone(db));
        }

        // Initialize the database
        let db = Self::new(app_handle)?;
        let arc_db = Arc::new(Mutex::new(db));
        *db_instance = Some(Arc::clone(&arc_db));
        Ok(arc_db)
    }

    /// Save a clipboard item to the database
    pub fn save_item(&mut self, item: ClipboardItem) -> SaveResult {
        // Log the item being saved
        println!("=== SAVING CLIPBOARD ITEM TO DATABASE ===");
        println!("Item ID: {}", item.id);
        println!("Timestamp: {}", item.timestamp);
        println!("Byte Size: {}", item.byte_size);
        println!("Text: {:?}", item.text);
        println!("==========================================");
        let mut db_item = DatabaseItem::from(item);

        // Serialize the formats
        let serialized_formats = match serde_json::to_string(&db_item.formats) {
            Ok(data) => data,
            Err(e) => return SaveResult {
                success: false,
                id: None,
                error: Some(format!("Failed to serialize formats: {}", e)),
            },
        };

        // Start a transaction
        let tx = match self.conn.transaction() {
            Ok(tx) => tx,
            Err(e) => return SaveResult {
                success: false,
                id: None,
                error: Some(format!("Failed to start transaction: {}", e)),
            },
        };

        let result = tx.execute(
            "INSERT INTO items (text, timestamp, byte_size, formats) VALUES (?1, ?2, ?3, ?4)",
            params![
                db_item.text,
                db_item.timestamp,
                db_item.byte_size,
                serialized_formats
            ],
        );

        let id = match result {
            Ok(_) => tx.last_insert_rowid() as u64,
            Err(e) => {
                return SaveResult {
                    success: false,
                    id: None,
                    error: Some(format!("Insert failed: {}", e)),
                };
            }
        };

        // Commit the transaction
        if let Err(e) = tx.commit() {
            return SaveResult {
                success: false,
                id: Some(id),
                error: Some(format!("Commit failed: {}", e)),
            };
        }

        db_item.id = id;

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
    }

    /// Get recent items with pagination
    pub fn recent_items(&self, count: usize, offset: usize) -> Result<Vec<ClipboardItem>> {
        println!("=== GETTING RECENT ITEMS FROM DATABASE ===");
        println!("Count: {}, Offset: {}", count, offset);

        let mut stmt = self.conn.prepare(
            "SELECT id, text, timestamp, byte_size, formats
             FROM items
             ORDER BY timestamp DESC
             LIMIT ?1 OFFSET ?2"
        )?;

        let item_iter = stmt.query_map(
            params![count as i64, offset as i64],
            |row| {
                let id: u64 = row.get(0)?;
                let text: Option<String> = row.get(1)?;
                let timestamp: u64 = row.get(2)?;
                let byte_size: u64 = row.get(3)?;
                let formats_json: String = row.get(4)?;

                let formats: crate::structs::ClipboardFormats = serde_json::from_str(&formats_json)
                    .map_err(|_e| rusqlite::Error::InvalidColumnType(4, "formats".to_string(), rusqlite::types::Type::Text))?;

                Ok(DatabaseItem {
                    id,
                    text,
                    timestamp,
                    byte_size,
                    formats,
                })
            }
        )?;

        let mut items = Vec::new();
        for item in item_iter {
            let db_item = item?;
            items.push(ClipboardItem::from(db_item));
        }

        Ok(items)
    }

    /// Search items by text content
    pub fn search(&self, query: &str, count: usize) -> Result<Vec<ClipboardItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, text, timestamp, byte_size, formats
             FROM items
             WHERE LOWER(text) LIKE LOWER(?1)
             ORDER BY timestamp DESC
             LIMIT ?2"
        )?;

        let search_pattern = format!("%{}%", query);

        let item_iter = stmt.query_map(
            params![search_pattern, count as i64],
            |row| {
                let id: u64 = row.get(0)?;
                let text: Option<String> = row.get(1)?;
                let timestamp: u64 = row.get(2)?;
                let byte_size: u64 = row.get(3)?;
                let formats_json: String = row.get(4)?;

                let formats: crate::structs::ClipboardFormats = serde_json::from_str(&formats_json)
                    .map_err(|_e| rusqlite::Error::InvalidColumnType(4, "formats".to_string(), rusqlite::types::Type::Text))?;

                Ok(DatabaseItem {
                    id,
                    text,
                    timestamp,
                    byte_size,
                    formats,
                })
            }
        )?;

        let mut items = Vec::new();
        for item in item_iter {
            let db_item = item?;
            items.push(ClipboardItem::from(db_item));
        }

        Ok(items)
    }

    /// Delete an item by ID
    pub fn delete_item(&mut self, id: u64) -> SaveResult {
        match self.conn.execute(
            "DELETE FROM items WHERE id = ?1",
            params![id],
        ) {
            Ok(rows_affected) => {
                if rows_affected > 0 {
                    SaveResult {
                        success: true,
                        id: Some(id),
                        error: None,
                    }
                } else {
                    SaveResult {
                        success: false,
                        id: Some(id),
                        error: Some("Item not found".to_string()),
                    }
                }
            },
            Err(e) => SaveResult {
                success: false,
                id: Some(id),
                error: Some(format!("Delete failed: {}", e)),
            },
        }
    }

    /// Get total count of items
    pub fn get_count(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM items",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Flush all pending writes to disk
    pub fn flush(&mut self) -> Result<()> {
        // SQLite handles automatic durability, but we can run WAL checkpoint
        self.conn.execute("PRAGMA wal_checkpoint(TRUNCATE)", [])?;
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
    let db_mutex = ClipboardDatabase::get_instance(&app_handle)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    let mut db = db_mutex.lock()
        .map_err(|e| format!("Failed to lock database: {}", e))?;

    Ok(db.save_item(item))
}

#[tauri::command]
pub fn db_recent_items(
    app_handle: AppHandle,
    count: usize,
    offset: usize,
) -> Result<Vec<ClipboardItem>, String> {
    let db_mutex = ClipboardDatabase::get_instance(&app_handle)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    let db = db_mutex.lock()
        .map_err(|e| format!("Failed to lock database: {}", e))?;

    db.recent_items(count, offset)
        .map_err(|e| format!("Failed to get recent items: {}", e))
}

#[tauri::command]
pub fn db_search(
    app_handle: AppHandle,
    query: String,
    count: usize,
) -> Result<Vec<ClipboardItem>, String> {
    let db_mutex = ClipboardDatabase::get_instance(&app_handle)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    let db = db_mutex.lock()
        .map_err(|e| format!("Failed to lock database: {}", e))?;

    db.search(&query, count)
        .map_err(|e| format!("Failed to search items: {}", e))
}

#[tauri::command]
pub fn db_delete_item(
    app_handle: AppHandle,
    id: u64,
) -> Result<SaveResult, String> {
    let db_mutex = ClipboardDatabase::get_instance(&app_handle)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    let mut db = db_mutex.lock()
        .map_err(|e| format!("Failed to lock database: {}", e))?;

    Ok(db.delete_item(id))
}

#[tauri::command]
pub fn db_get_count(app_handle: AppHandle) -> Result<usize, String> {
    let db_mutex = ClipboardDatabase::get_instance(&app_handle)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    let db = db_mutex.lock()
        .map_err(|e| format!("Failed to lock database: {}", e))?;

    db.get_count()
        .map_err(|e| format!("Failed to get item count: {}", e))
}

#[tauri::command]
pub fn db_flush(app_handle: AppHandle) -> Result<String, String> {
    let db_mutex = ClipboardDatabase::get_instance(&app_handle)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    let mut db = db_mutex.lock()
        .map_err(|e| format!("Failed to lock database: {}", e))?;

    db.flush()
        .map_err(|e| format!("Failed to flush database: {}", e))?;

    Ok("Database flushed successfully".to_string())
}
