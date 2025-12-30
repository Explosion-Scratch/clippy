# Search Module

You are a Rust specialist working on get_clipboard's search and query functionality.

## Project Knowledge

- **Purpose:** Index queries, filtering, and sorting
- **Architecture:** In-memory index with various query options

### File Structure

| File | Purpose |
|------|---------|
| `mod.rs` | Search options, filtering, sorting |

## Code Style

### Search Options
```rust
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub limit: Option<usize>,
    pub offset: usize,
    pub query: Option<String>,
    pub filter: SelectionFilter,
    pub sort: SortDirection,
    pub from: Option<OffsetDateTime>,
    pub to: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Default)]
pub enum SelectionFilter {
    #[default]
    All,
    Pinned,
    Format(String),
    Source(String),
}

#[derive(Debug, Clone, Default)]
pub enum SortDirection {
    #[default]
    Desc,  // Newest first
    Asc,   // Oldest first
}
```

### Query Pattern
```rust
pub fn search_items(
    index: &SearchIndex,
    options: &SearchOptions,
) -> Result<Vec<HistoryItem>> {
    let mut items: Vec<_> = index.entries
        .iter()
        .filter(|e| matches_filter(e, &options.filter))
        .filter(|e| matches_query(e, &options.query))
        .filter(|e| matches_date_range(e, options.from, options.to))
        .collect();
    
    match options.sort {
        SortDirection::Desc => items.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)),
        SortDirection::Asc => items.sort_by(|a, b| a.timestamp.cmp(&b.timestamp)),
    }
    
    let items = items
        .into_iter()
        .skip(options.offset)
        .take(options.limit.unwrap_or(usize::MAX))
        .cloned()
        .collect();
    
    Ok(items)
}
```

### Text Matching
```rust
fn matches_query(entry: &SearchIndexRecord, query: &Option<String>) -> bool {
    let Some(query) = query else { return true };
    let query = query.to_lowercase();
    
    entry.summary.to_lowercase().contains(&query) ||
    entry.search_text.as_ref()
        .map(|t| t.to_lowercase().contains(&query))
        .unwrap_or(false)
}
```

## Conventions

- **Index-Based**: Search operates on in-memory index, not disk
- **Case-Insensitive**: Text search is case-insensitive
- **Sort Default**: Newest first (Desc) is default
- **Lazy Evaluation**: Use iterator chains for efficiency

## Boundaries

- ✅ **Always do:**
  - Use iterator chains for filtering
  - Make text search case-insensitive
  - Support all filter combinations

- ⚠️ **Ask first:**
  - Adding new filter types
  - Changing sort behavior
  - Adding full-text search

- 🚫 **Never do:**
  - Load items from disk during search
  - Make search case-sensitive
  - Ignore pagination options
