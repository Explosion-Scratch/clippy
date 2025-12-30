# Data Storage Module

You are a Rust specialist working on get_clipboard's file-system storage layer.

## Project Knowledge

- **Purpose:** Persistent storage for clipboard items using file system
- **Location:** `~/Library/Application Support/com.clipboard/data/`
- **Architecture:** SHA-256 hash-based directory structure with in-memory index

### File Structure

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports |
| `model.rs` | Data structures (EntryMetadata, SearchIndex) |
| `store.rs` | Storage operations (~1150 lines) |

### Storage Layout
```
data/
├── ab/
│   └── cd/
│       └── abcd1234.../
│           ├── metadata.json
│           ├── text.txt
│           └── image.png
└── index.json (search index cache)
```

## Code Style

### Entry Metadata
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryMetadata {
    pub hash: String,
    pub timestamp: OffsetDateTime,
    pub first_seen: OffsetDateTime,
    pub copy_count: u64,
    pub summary: String,
    pub detected_formats: Vec<String>,
    pub sources: Vec<String>,
}
```

### Index Pattern (OnceCell + RwLock)
```rust
static INDEX_CACHE: OnceCell<RwLock<SearchIndex>> = OnceCell::new();

pub fn load_index() -> Result<SearchIndex> {
    let guard = index_cell().read().map_err(|e| anyhow!("Lock poisoned: {e}"))?;
    Ok(guard.clone())
}

fn index_cell() -> &'static RwLock<SearchIndex> {
    INDEX_CACHE.get_or_init(|| {
        RwLock::new(load_index_from_disk().unwrap_or_default())
    })
}
```

### Storage Operations
```rust
// Store new entry
pub fn store_snapshot(snapshot: ClipboardSnapshot) -> Result<EntryMetadata> {
    let hash = compute_hash(&snapshot);
    let entry_dir = entry_path(&hash)?;
    
    fs::create_dir_all(&entry_dir)?;
    
    // Write plugin files...
    // Write metadata.json...
    // Update index...
    
    Ok(metadata)
}

// Load by selector (index or hash)
pub fn resolve_selector(selector: &str) -> Result<String> {
    if let Ok(index) = selector.parse::<usize>() {
        resolve_by_index(index)
    } else {
        resolve_by_hash_prefix(selector)
    }
}
```

### Hash-Based Path
```rust
fn entry_path(hash: &str) -> Result<PathBuf> {
    let data_dir = ensure_data_dir()?;
    // ab/cd/abcd1234...
    Ok(data_dir
        .join(&hash[0..2])
        .join(&hash[2..4])
        .join(hash))
}
```

## Conventions

- **Hash-Based Dedup**: Same content produces same hash, enabling deduplication
- **Nested Directories**: First 4 chars of hash create 2-level nesting (ab/cd/)
- **Index Cache**: In-memory index for fast lookups, persisted to disk
- **Copy Count**: Track how many times an item has been copied

## Boundaries

- ✅ **Always do:**
  - Use hash-based paths for entries
  - Update index after storage operations
  - Use `RwLock` for index access
  - Preserve `first_seen` timestamp on re-copy

- ⚠️ **Ask first:**
  - Changing storage layout
  - Modifying metadata schema
  - Adding new index fields

- 🚫 **Never do:**
  - Use SQLite or any database
  - Store items outside the hash-based structure
  - Skip index updates after modifications
  - Hold index lock during I/O
