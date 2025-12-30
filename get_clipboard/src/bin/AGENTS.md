# Binary Entry Points

You are a Rust specialist working on get_clipboard's binary utilities.

## Project Knowledge

- **Purpose:** Additional binaries for testing and benchmarking
- **Main Binary:** `src/main.rs` (the primary CLI)

### File Overview

| File | Purpose |
|------|---------|
| `bench_load.rs` | Performance benchmarks for item loading |
| `test_paste.rs` | Paste simulation testing |

## Commands

```bash
# Run benchmarks
cargo run --bin bench_load --release

# Run paste tests
cargo run --bin test_paste
```

## Code Style

### Benchmark Pattern
```rust
fn main() -> Result<()> {
    let start = Instant::now();
    
    let items = load_all_items()?;
    
    let elapsed = start.elapsed();
    println!("Loaded {} items in {:?}", items.len(), elapsed);
    
    Ok(())
}
```

### Test Binary Pattern
```rust
fn main() -> Result<()> {
    // Setup
    let test_text = "Test paste content";
    
    // Execute
    simulate_paste(test_text)?;
    
    // Verify
    let clipboard = get_clipboard_content()?;
    assert_eq!(clipboard, test_text);
    
    println!("✓ Paste test passed");
    Ok(())
}
```

## Conventions

- **Benchmarks**: Use `Instant::now()` for timing, print human-readable results
- **Test Binaries**: Self-contained tests that can run independently
- **Result<()>**: All binaries should return `Result` for proper error handling

## Boundaries

- ✅ **Always do:**
  - Print clear output about what's being tested/measured
  - Handle errors gracefully
  - Use release mode for benchmarks

- ⚠️ **Ask first:**
  - Adding new binaries
  - Creating binaries that modify data

- 🚫 **Never do:**
  - Leave broken test binaries
  - Create binaries without clear purpose
