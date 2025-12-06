# get_clipboard

A fast, local-first clipboard manager CLI for macOS with a built-in web dashboard.

**[Download Latest Release](https://github.com/Explosion-Scratch/clippy/releases/latest)**

## Installation

```bash
# Download and extract
curl -LO https://github.com/Explosion-Scratch/clippy/releases/latest/download/get_clipboard
chmod +x get_clipboard
mv get_clipboard ~/.local/bin
```

## Quick Start

```bash
# Start the background service (monitors clipboard)
get_clipboard service install
get_clipboard service start

# Open dashboard at http://127.0.0.1:3016/dashboard/
get_clipboard api # --port ___
```

---

## Storage

Items are stored in `~/Library/Application Support/clippy/data/` organized by date and content hash:

```
data/
├── 2025/12/
│   ├── a1/b2/                          # First 4 chars of hash (2+2)
│   │   └── a1b2c3d4e5f6.../            # Full SHA-256 hash
│   │       ├── metadata.json           # Item metadata
│   │       ├── text.txt                # Plain text
│   │       ├── html.html               # HTML content (if copied)
│   │       └── image.png               # Image (if applicable)
```

**Hash computation**: SHA-256 of all clipboard content (text + HTML + RTF + image bytes + file paths/sizes). This deduplicates identical copies—copying the same text twice updates `copy_count` and `last_seen` rather than creating duplicates.

---

## Commands

### Core

| Command | Description |
|---------|-------------|
| `history` | List clipboard items (default command) |
| `search <query>` | Full-text search |
| `show <selector>` | Display item details |
| `copy <selector>` | Copy item to clipboard |
| `paste <selector>` | Copy + simulate Cmd+V |
| `delete <selector>` | Remove item |
| `interactive` | TUI mode with live filtering |

**Selectors**: Use index (`0` = most recent) or hash (`a1b2c3...`, min 6 chars).

### Service

```bash
get_clipboard service install    # Create launchd plist
get_clipboard service start      # Start background monitor
get_clipboard service stop       # Stop monitor
get_clipboard service status     # Check if running
get_clipboard service logs -f    # Tail logs
get_clipboard service uninstall  # Remove plist
```

### Data Management

```bash
get_clipboard dir get                        # Print data directory
get_clipboard dir set /path/to/new           # Change directory (no move)
get_clipboard dir move /path/to/new          # Move data to new location
get_clipboard export ./backup.json           # Export all items
get_clipboard import ./backup.json           # Import items
get_clipboard stats                          # Storage statistics
```

### API & Dashboard

```bash
get_clipboard api --port 3016    # Start REST API + dashboard
```
Dashboard: `http://127.0.0.1:3016/dashboard/`

### Permissions

```bash
get_clipboard permissions check     # Verify accessibility access
get_clipboard permissions request   # Open System Settings
```

---

## Usage Examples

### Basic

```bash
# Show last 10 items
get_clipboard history -l 10

# Copy most recent item
get_clipboard copy 0

# Show 5th most recent
get_clipboard show 4

# Delete by hash
get_clipboard delete deadbeef
```

### Search & Filtering

```bash
# Search text content
get_clipboard search "meeting notes"

# Regex search
get_clipboard search --regex "^\d{4}-\d{2}-\d{2}"

# Search by shortcut - @email, @link, @image, @file, @html, @color, @path supported
get_clipboard search "@link"

# Filter by type
get_clipboard history --text        # Text only
get_clipboard history --image       # Images only
get_clipboard history --file        # Files only
get_clipboard history --html        # HTML only

# Date range
get_clipboard history --from 2025-12-01 --to 2025-12-05

# Sort options
get_clipboard history --sort copies   # Most copied first
get_clipboard search "api" --sort relevance
```

### JSON Output

```bash
# Simple JSON (metadata only)
get_clipboard history --json

# Full JSON (includes content)
get_clipboard history --json --full

# Pipe to jq
get_clipboard history --json | jq '.[0].id'

# Export specific items
get_clipboard show 0 --json > item.json
```

### Interactive TUI

```bash
# Open interactive mode
get_clipboard interactive

# Pre-filter with query
get_clipboard interactive -q "password"
```

**TUI Controls:**
- `↑/↓` Navigate items
- `Enter` Copy selected
- `Ctrl+D` Delete
- `/` Focus search
- `q/Esc` Quit

### Scripting

```bash
# Get latest text item content directly
get_clipboard show 0 2>/dev/null | head -n -10

# Find items containing URLs
get_clipboard search --regex "https?://" --json | jq -r '.[].id'

# Auto-copy most copied item
get_clipboard history --sort copies --json | jq -r '.[0].id' | xargs get_clipboard copy

# Grep through all text items
get_clipboard history --text --full --json | jq -r '.[] | select(.formats[]?.data | strings | test("TODO"))'

# Monitor clipboard changes
get_clipboard watch

# Pipe clipboard to file
get_clipboard show 0 > /tmp/clipboard.txt
```

### Statistics

```bash
get_clipboard stats

# Output:
# Clipboard Statistics
# ====================
# Total items:    1523
# Reported size:  45.2 MB
# Storage size:   52.1 MB
#
# By type:
#   text       1203
#   image      245
#   file       75
#
# Top 20 Largest Items (by storage):
# Index    Type       Size         Summary
# ----------------------------------------------------------------------
# 42       image      2.34 MB      Image 2096x1084
# 156      file       1.87 MB      3 files in ~/Downloads
```

---

## Global Flags

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |
| `--text` | Filter to text items |
| `--image` | Filter to images |
| `--file` | Filter to files |
| `--html` | Filter to HTML |
| `--rtf` | Filter to RTF |

---

## Dashboard

Start with `get_clipboard api --port 3016`, then open `http://127.0.0.1:3016/dashboard/`.

**Features:**
- Browse/search all clipboard items
- View different formats (text, HTML, image, files)
- Multi-select with Cmd+Click
- Statistics with charts
- Import/export data
- Configure data directory

**Keyboard shortcuts:**
- `/` Focus search
- `Esc` Clear selection

---

## API

REST API served on the same port as dashboard. See [API.md](API.md) for full documentation.

Quick reference:
```bash
curl localhost:3016/items?count=10              # List items
curl localhost:3016/item/0                      # Get item metadata
curl localhost:3016/item/0/data                 # Get full item data
curl localhost:3016/search?query=test           # Search
curl -X POST localhost:3016/item/0/copy         # Copy to clipboard
curl -X DELETE localhost:3016/item/a1b2c3       # Delete item
```

---

## Formats Supported

| Format | Storage | Searchable |
|--------|---------|------------|
| Plain text | `text.txt` | ✓ |
| HTML | `html.html` | ✓ |
| RTF | `rtf.rtf` | ✓ |
| Images | `image.png` | — |
| Files | `files.json` | Paths only |

Items can contain multiple formats simultaneously (e.g., copying from a browser gives both text and HTML).

---

## License

MIT
