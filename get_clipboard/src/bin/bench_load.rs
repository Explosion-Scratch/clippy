use anyhow::Result;
use get_clipboard::data::store::{load_index, load_history_items, refresh_index, load_metadata};
use get_clipboard::search::SearchOptions;
use std::time::Instant;

fn main() -> Result<()> {
    println!("Benchmark: Database Access Performance");
    println!("========================================\n");

    println!("Phase 1: Index Loading");
    println!("----------------------");
    let start = Instant::now();
    refresh_index()?;
    let index = load_index()?;
    let index_time = start.elapsed();
    println!("Index loaded: {} items in {:?}\n", index.len(), index_time);

    println!("Phase 2: Shallow History Loads (using index)");
    println!("---------------------------------------------");
    let test_sizes = [10, 25, 50, 100, 200];
    
    for &limit in &test_sizes {
        let mut options = SearchOptions::default();
        options.limit = Some(limit);

        let start = Instant::now();
        let (items, _) = load_history_items(&index, &options)?;
        let elapsed = start.elapsed();
        
        let per_item = if !items.is_empty() {
            elapsed.as_micros() / items.len() as u128
        } else {
            0
        };

        println!(
            "Loaded {} items in {:?} ({} μs/item)",
            items.len(),
            elapsed,
            per_item
        );
    }

    println!("\nPhase 3: Direct Metadata Access");
    println!("--------------------------------");
    let sample_hashes: Vec<String> = index.keys().take(20).cloned().collect();
    
    let start = Instant::now();
    for hash in &sample_hashes {
        let _ = load_metadata(hash)?;
    }
    let elapsed = start.elapsed();
    let per_load = if !sample_hashes.is_empty() {
        elapsed.as_micros() / sample_hashes.len() as u128
    } else {
        0
    };
    println!(
        "Loaded {} metadata entries in {:?} ({} μs/entry)",
        sample_hashes.len(),
        elapsed,
        per_load
    );

    println!("\nPhase 4: Search with Query");
    println!("---------------------------");
    let queries = ["test", "image", "file"];
    for query in queries {
        let mut options = SearchOptions::default();
        options.limit = Some(50);
        options.query = Some(query.to_string());

        let start = Instant::now();
        let (items, _) = load_history_items(&index, &options)?;
        let elapsed = start.elapsed();
        
        println!(
            "Query '{}': {} results in {:?}",
            query,
            items.len(),
            elapsed
        );
    }

    println!("\n✓ Benchmark complete!");
    Ok(())
}

