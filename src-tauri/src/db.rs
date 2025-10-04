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
                first_copied INTEGER NOT NULL,
                copies INTEGER NOT NULL DEFAULT 1,
                byte_size INTEGER NOT NULL,
                formats TEXT NOT NULL,
                content_hash TEXT
            )",
            [],
        )?;

        // Migration: Add new columns if they don't exist (for backward compatibility)
        self.migrate_schema()?;

        // Create indexes for performance
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_items_timestamp ON items(timestamp)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_items_text ON items(text)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_items_content_hash ON items(content_hash)",
            [],
        )?;

        Ok(())
    }

    /// Migrate existing database schema to include new columns
    fn migrate_schema(&self) -> Result<()> {
        // Check if first_copied column exists
        let mut stmt = self.conn.prepare("PRAGMA table_info(items)")?;
        let columns: Vec<String> = stmt.query_map([], |row| {
            let name: String = row.get(1)?;
            Ok(name)
        })?.collect::<Result<Vec<_>, _>>()?;

        // Add first_copied column if it doesn't exist
        if !columns.contains(&"first_copied".to_string()) {
            println!("Adding first_copied column to existing database");
            self.conn.execute(
                "ALTER TABLE items ADD COLUMN first_copied INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
            // Set first_copied to timestamp for existing records
            self.conn.execute(
                "UPDATE items SET first_copied = timestamp WHERE first_copied = 0",
                [],
            )?;
        }

        // Add copies column if it doesn't exist
        if !columns.contains(&"copies".to_string()) {
            println!("Adding copies column to existing database");
            self.conn.execute(
                "ALTER TABLE items ADD COLUMN copies INTEGER NOT NULL DEFAULT 1",
                [],
            )?;
        }

        // Add content_hash column if it doesn't exist
        if !columns.contains(&"content_hash".to_string()) {
            println!("Adding content_hash column to existing database");
            self.conn.execute(
                "ALTER TABLE items ADD COLUMN content_hash TEXT",
                [],
            )?;
            // Generate hashes for existing records
            self.conn.execute(
                "UPDATE items SET content_hash = CASE
                    WHEN text IS NOT NULL THEN substr(hex(md5(text || formats)), 1, 16)
                    ELSE substr(hex(md5(formats)), 1, 16)
                END WHERE content_hash IS NULL",
                [],
            )?;
        }

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
        println!("First Copied: {}", item.first_copied);
        println!("Copies: {}", item.copies);
        println!("Byte Size: {}", item.byte_size);
        println!("Text: {:?}", item.text);
        println!("Formats: {:?}", item.formats);
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

        // Generate content hash for duplicate detection
        let content_hash = match self.generate_content_hash(&db_item.text, &serialized_formats) {
            Ok(hash) => hash,
            Err(e) => return SaveResult {
                success: false,
                id: None,
                error: Some(format!("Failed to generate content hash: {}", e)),
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

        // Check for duplicate content
        let existing_id = match tx.query_row(
            "SELECT id FROM items WHERE content_hash = ?1",
            params![content_hash],
            |row| row.get::<_, u64>(0),
        ) {
            Ok(id) => Some(id),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => {
                return SaveResult {
                    success: false,
                    id: None,
                    error: Some(format!("Failed to check for duplicates: {}", e)),
                };
            }
        };

        let id = if let Some(existing_id) = existing_id {
            // Update existing item: increment copies and update timestamp
            println!("Found duplicate content, updating existing item ID: {}", existing_id);
            match tx.execute(
                "UPDATE items SET timestamp = ?1, copies = copies + 1 WHERE id = ?2",
                params![db_item.timestamp, existing_id],
            ) {
                Ok(_) => existing_id,
                Err(e) => {
                    return SaveResult {
                        success: false,
                        id: None,
                        error: Some(format!("Failed to update existing item: {}", e)),
                    };
                }
            }
        } else {
            // Insert new item
            println!("No duplicate found, inserting new item");
            let result = tx.execute(
                "INSERT INTO items (text, timestamp, first_copied, copies, byte_size, formats, content_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    db_item.text,
                    db_item.timestamp,
                    db_item.first_copied,
                    db_item.copies,
                    db_item.byte_size,
                    serialized_formats,
                    content_hash
                ],
            );

            match result {
                Ok(_) => tx.last_insert_rowid() as u64,
                Err(e) => {
                    return SaveResult {
                        success: false,
                        id: None,
                        error: Some(format!("Insert failed: {}", e)),
                    };
                }
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
        println!("Saved item with ID: {} ({} copies)", id, if existing_id.is_some() { "updated existing" } else { "new" });

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

    /// Generate content hash for duplicate detection
    fn generate_content_hash(&self, text: &Option<String>, formats: &str) -> Result<String> {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();

        // Hash the text content
        if let Some(t) = text {
            t.hash(&mut hasher);
        }

        // Hash the serialized formats
        formats.hash(&mut hasher);

        Ok(format!("{:x}", hasher.finish()))
    }

    /// Get recent items with pagination
    pub fn recent_items(&self, count: usize, offset: usize) -> Result<Vec<ClipboardItem>> {

        let mut stmt = self.conn.prepare(
            "SELECT id, text, timestamp, first_copied, copies, byte_size, formats
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
                let first_copied: u64 = row.get(3)?;
                let copies: u64 = row.get(4)?;
                let byte_size: u64 = row.get(5)?;
                let formats_json: String = row.get(6)?;

                let formats: crate::structs::ClipboardFormats = serde_json::from_str(&formats_json)
                    .map_err(|_e| rusqlite::Error::InvalidColumnType(6, "formats".to_string(), rusqlite::types::Type::Text))?;

      Ok(DatabaseItem {
            id,
            text,
            timestamp,
            first_copied,
            copies,
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
            "SELECT id, text, timestamp, first_copied, copies, byte_size, formats
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
                let first_copied: u64 = row.get(3)?;
                let copies: u64 = row.get(4)?;
                let byte_size: u64 = row.get(5)?;
                let formats_json: String = row.get(6)?;

                let formats: crate::structs::ClipboardFormats = serde_json::from_str(&formats_json)
                    .map_err(|_e| rusqlite::Error::InvalidColumnType(6, "formats".to_string(), rusqlite::types::Type::Text))?;

      Ok(DatabaseItem {
            id,
            text,
            timestamp,
            first_copied,
            copies,
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

    /// Get database file size in bytes
    pub fn get_database_size(&self, app_handle: &AppHandle) -> Result<u64> {
        use std::path::Path;
        
        let app_data_dir = app_handle.path().app_data_dir()?;
        let db_path = app_data_dir.join("clipboard.db");
        
        if db_path.exists() {
            let metadata = std::fs::metadata(&db_path)?;
            Ok(metadata.len())
        } else {
            Ok(0)
        }
    }

    /// Get an item by ID
    pub fn get_item_by_id(&self, id: u64) -> Result<ClipboardItem> {
        let mut stmt = self.conn.prepare(
            "SELECT id, text, timestamp, first_copied, copies, byte_size, formats
             FROM items
             WHERE id = ?1"
        )?;

        let item = stmt.query_row(
            params![id],
            |row| {
                let id: u64 = row.get(0)?;
                let text: Option<String> = row.get(1)?;
                let timestamp: u64 = row.get(2)?;
                let first_copied: u64 = row.get(3)?;
                let copies: u64 = row.get(4)?;
                let byte_size: u64 = row.get(5)?;
                let formats_json: String = row.get(6)?;

                let formats: crate::structs::ClipboardFormats = serde_json::from_str(&formats_json)
                    .map_err(|_e| rusqlite::Error::InvalidColumnType(6, "formats".to_string(), rusqlite::types::Type::Text))?;

      Ok(DatabaseItem {
            id,
            text,
            timestamp,
            first_copied,
            copies,
            byte_size,
            formats,
        })
            }
        )?;

        Ok(ClipboardItem::from(item))
    }

    /// Increment the copies counter for an item by ID
    pub fn increment_copies(&mut self, id: u64) -> SaveResult {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        match self.conn.execute(
            "UPDATE items SET copies = copies + 1, timestamp = ?1 WHERE id = ?2",
            params![current_time, id],
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
                error: Some(format!("Failed to increment copies: {}", e)),
            },
        }
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
pub fn db_get_size(app_handle: AppHandle) -> Result<u64, String> {
    let db_mutex = ClipboardDatabase::get_instance(&app_handle)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    let db = db_mutex.lock()
        .map_err(|e| format!("Failed to lock database: {}", e))?;

    db.get_database_size(&app_handle)
        .map_err(|e| format!("Failed to get database size: {}", e))
}

#[tauri::command]
pub fn db_get_item_by_id(
    app_handle: AppHandle,
    id: u64,
) -> Result<ClipboardItem, String> {
    let db_mutex = ClipboardDatabase::get_instance(&app_handle)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    let db = db_mutex.lock()
        .map_err(|e| format!("Failed to lock database: {}", e))?;

    db.get_item_by_id(id)
        .map_err(|e| format!("Failed to get item by ID: {}", e))
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

/// Export all database items to JSON
#[tauri::command]
pub fn db_export_all(app_handle: AppHandle) -> Result<String, String> {
    let db_mutex = ClipboardDatabase::get_instance(&app_handle)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    let db = db_mutex.lock()
        .map_err(|e| format!("Failed to lock database: {}", e))?;

    // Get all items from database
    let mut stmt = db.conn.prepare(
        "SELECT id, text, timestamp, first_copied, copies, byte_size, formats
         FROM items
         ORDER BY timestamp DESC"
    ).map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let item_iter = stmt.query_map([], |row| {
        let id: u64 = row.get(0)?;
        let text: Option<String> = row.get(1)?;
        let timestamp: u64 = row.get(2)?;
        let first_copied: u64 = row.get(3)?;
        let copies: u64 = row.get(4)?;
        let byte_size: u64 = row.get(5)?;
        let formats_json: String = row.get(6)?;

        let formats: crate::structs::ClipboardFormats = serde_json::from_str(&formats_json)
            .map_err(|_e| rusqlite::Error::InvalidColumnType(6, "formats".to_string(), rusqlite::types::Type::Text))?;

        Ok(DatabaseItem {
            id,
            text,
            timestamp,
            first_copied,
            copies,
            byte_size,
            formats,
        })
    }).map_err(|e| format!("Failed to query items: {}", e))?;

    let mut items = Vec::new();
    for item in item_iter {
        let db_item = item.map_err(|e| format!("Failed to process item: {}", e))?;
        items.push(ClipboardItem::from(db_item));
    }

    // Convert to JSON
    serde_json::to_string_pretty(&items)
        .map_err(|e| format!("Failed to serialize items: {}", e))
}

/// Import items from JSON data
#[tauri::command]
pub fn db_import_all(app_handle: AppHandle, json_data: String) -> Result<String, String> {
    let db_mutex = ClipboardDatabase::get_instance(&app_handle)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    let mut db = db_mutex.lock()
        .map_err(|e| format!("Failed to lock database: {}", e))?;

    // Parse JSON data
    let items: Vec<ClipboardItem> = serde_json::from_str(&json_data)
        .map_err(|e| format!("Failed to parse JSON data: {}", e))?;

    let mut imported_count = 0;
    let mut skipped_count = 0;

    // Start transaction
    let tx = db.conn.transaction()
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    for item in items {
        let db_item = DatabaseItem::from(item);

        // Serialize the formats
        let serialized_formats = serde_json::to_string(&db_item.formats)
            .map_err(|e| format!("Failed to serialize formats: {}", e))?;

        // Generate content hash using a separate function to avoid borrowing issues
        let content_hash = {
            use std::hash::{Hash, Hasher};
            use std::collections::hash_map::DefaultHasher;

            let mut hasher = DefaultHasher::new();

            // Hash the text content
            if let Some(ref t) = db_item.text {
                t.hash(&mut hasher);
            }

            // Hash the serialized formats
            serialized_formats.hash(&mut hasher);

            format!("{:x}", hasher.finish())
        };

        // Check for duplicate content
        let existing_id = tx.query_row(
            "SELECT id FROM items WHERE content_hash = ?1",
            params![content_hash],
            |row| row.get::<_, u64>(0),
        );

        match existing_id {
            Ok(_) => {
                skipped_count += 1;
                continue;
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // Insert new item
                let result = tx.execute(
                    "INSERT INTO items (text, timestamp, first_copied, copies, byte_size, formats, content_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        db_item.text,
                        db_item.timestamp,
                        db_item.first_copied,
                        db_item.copies,
                        db_item.byte_size,
                        serialized_formats,
                        content_hash
                    ],
                );

                match result {
                    Ok(_) => imported_count += 1,
                    Err(e) => {
                        return Err(format!("Failed to insert item: {}", e));
                    }
                }
            }
            Err(e) => {
                return Err(format!("Failed to check for duplicates: {}", e));
            }
        }
    }

    // Commit transaction
    tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    Ok(format!("Successfully imported {} items (skipped {} duplicates)", imported_count, skipped_count))
}

/// Delete all items from database
#[tauri::command]
pub fn db_increment_copies(
    app_handle: AppHandle,
    id: u64,
) -> Result<SaveResult, String> {
    let db_mutex = ClipboardDatabase::get_instance(&app_handle)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    let mut db = db_mutex.lock()
        .map_err(|e| format!("Failed to lock database: {}", e))?;

    Ok(db.increment_copies(id))
}

#[tauri::command]
pub fn db_delete_all(app_handle: AppHandle) -> Result<String, String> {
    let db_mutex = ClipboardDatabase::get_instance(&app_handle)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    let db = db_mutex.lock()
        .map_err(|e| format!("Failed to lock database: {}", e))?;

    // Get count before deletion
    let _count = db.get_count()
        .map_err(|e| format!("Failed to get item count: {}", e))?;

    // Delete all items
    let rows_affected = db.conn.execute("DELETE FROM items", [])
        .map_err(|e| format!("Failed to delete items: {}", e))?;

    Ok(format!("Successfully deleted {} items from database", rows_affected))
}
