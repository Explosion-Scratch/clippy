# CLI Module

You are a Rust CLI specialist working on get_clipboard's command-line interface.

## Project Knowledge

- **Tech Stack:** Clap (argument parsing)
- **Purpose:** User-facing CLI commands for clipboard operations
- **Entry:** `main.rs` calls `cli::run()`

### File Structure

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports |
| `args.rs` | Clap argument definitions |
| `handlers.rs` | Command implementations (~30KB) |

## Commands

```bash
get_clipboard list              # List clipboard items
get_clipboard list -n 20        # List 20 items
get_clipboard list --json       # JSON output
get_clipboard copy 1            # Copy item 1 to clipboard
get_clipboard copy abc123       # Copy by hash
get_clipboard delete 1          # Delete item
get_clipboard serve             # Start API server
get_clipboard watch             # Monitor clipboard
get_clipboard --help            # All options
```

## Code Style

### Clap Arguments
```rust
#[derive(Parser)]
#[command(name = "get_clipboard")]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// List clipboard history
    List {
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
        
        #[arg(long)]
        json: bool,
    },
    /// Copy item to clipboard
    Copy {
        /// Item selector (index or hash)
        selector: String,
    },
}
```

### Handler Pattern
```rust
// ✅ Good - TTY-aware output
pub fn handle_list(args: ListArgs) -> Result<()> {
    let is_tty = std::io::stdout().is_terminal();
    let items = load_items(args.limit)?;
    
    if args.json {
        println!("{}", serde_json::to_string_pretty(&items)?);
    } else {
        for (i, item) in items.iter().enumerate() {
            print_item(i, item, is_tty);
        }
    }
    Ok(())
}

// Item formatting with index padding
fn print_item(index: usize, item: &Item, is_tty: bool) {
    if is_tty {
        println!("{:>3} │ {}", index, item.summary.cyan());
    } else {
        println!("{:>3}\t{}", index, item.summary);
    }
}
```

### Selector Resolution
```rust
// Support both index and hash selectors
fn resolve_selector(selector: &str) -> Result<String> {
    if let Ok(index) = selector.parse::<usize>() {
        // Numeric: treat as index
        resolve_by_index(index)
    } else {
        // String: treat as hash prefix
        resolve_by_hash(selector)
    }
}
```

## Conventions

- **TTY Detection**: Use `stdout().is_terminal()` to adjust output
- **Telegraphic Output**: Minimal, pipeable output for non-TTY
- **JSON Flag**: All list commands should support `--json`
- **Selectors**: Accept both numeric indices and hash prefixes
- **Index Padding**: Use `{:>3}` for 3-digit right-aligned indices

## Boundaries

- ✅ **Always do:**
  - Support both TTY and pipe output
  - Use Clap derive macros
  - Document all arguments
  - Handle errors with context

- ⚠️ **Ask first:**
  - Adding new subcommands
  - Changing output formats
  - Adding interactive prompts

- 🚫 **Never do:**
  - Print color to non-TTY
  - Require confirmation for non-destructive commands
  - Use positional arguments for optional values
