# Performance Optimization Results

## Problem
Loading items from the database, even as shallow copies, was extremely slow. The root cause was that `load_metadata()` walked the entire directory tree for every single metadata lookup, resulting in O(n) complexity per lookup where n is the total number of entries.

## Solution
Added `relative_path` to the `SearchIndexRecord` so metadata files can be accessed directly using the in-memory index, reducing lookup complexity from O(n) to O(1).

### Changes Made

1. **Updated `SearchIndexRecord`** (`src/data/model.rs`)
   - Added `relative_path: String` field to store the path to each entry's directory

2. **Updated Index Building** (`src/data/store.rs`)
   - Modified `load_index_from_disk()` to include `relative_path` when building the index
   - Modified `update_index()` to include `relative_path` when updating the index

3. **Optimized Metadata Loading** (`src/data/store.rs`)
   - Rewrote `load_metadata()` to use the index for direct file access
   - Falls back to directory scan only if entry is not in index (rare edge case)

## Performance Results

### Before Optimization
- Loading 10 items: **2.59 seconds** (259,286 μs/item)
- Loading 25 items: **3.25 seconds** (129,924 μs/item)
- Loading 100 items: **8.48 seconds** (84,773 μs/item)
- Index loading: ~400ms

### After Optimization
- Loading 10 items: **541 μs** (54 μs/item) - **4,800x faster**
- Loading 25 items: **1.19 ms** (47 μs/item) - **2,700x faster**
- Loading 100 items: **4.50 ms** (44 μs/item) - **1,880x faster**
- Loading 200 items: **9.31 ms** (46 μs/item)
- Index loading: ~163ms (60% faster)
- Direct metadata access: **46 μs/entry**
- Search queries: 6-10ms for 36-50 results

## Benchmark Tool

A benchmark binary is available to test database access performance:

```bash
cargo build --release --bin bench_load
./target/release/bench_load
```

The benchmark tests:
1. Index loading from disk
2. Shallow history loads (various sizes)
3. Direct metadata access
4. Search queries

## Impact

The TUI and CLI are now extremely responsive when browsing history, with near-instantaneous page loads even with hundreds of entries in the database.

