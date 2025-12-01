# Clipboard Manager API Documentation

## Overview

The Clipboard Manager API is a RESTful HTTP API that provides programmatic access to clipboard history management. It allows you to retrieve, search, copy, save, and delete clipboard items through a simple HTTP interface.

**Base URL:** `{{URL}}`  
**Default Port:** Configurable via command line arguments  
**Response Format:** JSON  
**Authentication:** None (local access only)

---

## Table of Contents

1. [Core Concepts](#core-concepts)
2. [Data Models](#data-models)
3. [Endpoints](#endpoints)
   - [Root](#get-)
   - [Version](#get-version)
   - [Items Management](#items-management)
   - [Search](#search)
   - [Configuration](#configuration)
   - [Clipboard Operations](#clipboard-operations)
   - [Statistics](#statistics)
4. [Error Handling](#error-handling)
5. [Examples](#examples)
6. [Plugin System](#plugin-system)

---

## Core Concepts

### Item Identification

Items can be referenced using two types of selectors:

1. **Hash Selector**: A unique SHA-256 hash (minimum 6 characters, typically full 64-character hex string)
   - Example: `a1b2c3d4e5f6...` or `a1b2c3` (minimum)
   
2. **Offset Selector**: A numeric index based on chronological order (0 = most recent)
   - Example: `0`, `1`, `42`

The API automatically distinguishes between hash and offset selectors:
- Strings with 6+ characters are treated as hash selectors
- Numeric strings are treated as offset selectors
- Hashes provide stable references that don't change as new items are added
- Offsets are useful for accessing recent items but change as the history grows

### Item Types

The system supports multiple clipboard content types through a plugin architecture:

- **Text**: Plain text content
- **HTML**: Rich HTML content with formatting
- **RTF**: Rich Text Format documents
- **Image**: Image data (PNG, JPEG, etc.)
- **File**: File system references and file lists

Each item can contain multiple formats simultaneously (e.g., both text and HTML representations).

### Metadata Structure

Every clipboard item includes:
- **Hash**: Unique identifier based on content
- **Timestamp**: When the item was first captured
- **Last Seen**: Most recent observation time
- **Copy Count**: Number of times the item has been copied back to clipboard
- **Kind**: Primary content type (text, image, file, other)
- **Sources**: Original data sources (file paths, URLs, etc.)
- **Plugin Data**: Format-specific metadata stored by plugins

---

## Data Models

### ClipboardJsonItem (Summary)

Compact representation of a clipboard item without full content data:

```json
{
  "hash": "a1b2c3d4e5f6...",
  "offset": 0,
  "timestamp": "2025-11-02T10:30:00Z",
  "lastSeen": "2025-11-02T10:30:00Z",
  "copyCount": 0,
  "kind": "text",
  "summary": "First 200 characters of content...",
  "byteSize": 1024,
  "sources": ["clipboard"],
  "plugins": ["text", "html"],
  "metadata": {
    "text": {
      "length": 1024,
      "lines": 15
    }
  }
}
```

**Fields:**
- `hash` (string): Unique SHA-256 hash identifier
- `offset` (number): Chronological position (0 = most recent)
- `timestamp` (string): ISO 8601 creation timestamp
- `lastSeen` (string): ISO 8601 last observation timestamp
- `copyCount` (number): Times copied to clipboard
- `kind` (string): Primary type - "text", "image", "file", or "other"
- `summary` (string): Brief content preview
- `byteSize` (number): Total size in bytes
- `sources` (array): List of data source identifiers
- `plugins` (array): Active plugin IDs for this item
- `metadata` (object): Plugin-specific metadata

### ClipboardJsonFullItem (Complete)

Full representation including all content data:

```json
{
  "hash": "a1b2c3d4e5f6...",
  "offset": 0,
  "timestamp": "2025-11-02T10:30:00Z",
  "lastSeen": "2025-11-02T10:30:00Z",
  "copyCount": 0,
  "kind": "text",
  "summary": "First 200 characters of content...",
  "byteSize": 1024,
  "sources": ["clipboard"],
  "plugins": [
    {
      "id": "text",
      "data": "Full text content here..."
    },
    {
      "id": "html",
      "data": "<p>Full HTML content here...</p>"
    }
  ]
}
```

**Additional Fields:**
- `plugins` (array of objects): Full plugin data with content
  - `id` (string): Plugin identifier
  - `data` (any): Format-specific data structure

---

## Endpoints

### GET /

Returns this API documentation as plain text.

### GET /version

Returns the current version of get_clipboard and API server start time information.

**Response:**
```json
{
  "version": "0.1.0",
  "apiStartTime": 1764457053,
  "apiStartTimeIso": "2025-11-29T22:57:33.000000000Z"
}
```

**Fields:**
- `version` (string): Current version of get_clipboard from Cargo.toml
- `apiStartTime` (number|null): Unix timestamp when API server started (seconds since epoch)
- `apiStartTimeIso` (string|null): ISO 8601 formatted timestamp when API server started

**Example:**
```bash
curl {{URL}}/version
```

**Use Cases:**
- Version checking for client compatibility
- Monitoring API server uptime
- Debugging and troubleshooting
- Cache invalidation based on server restart

**Note:** If the API server has not been started properly, `apiStartTime` and `apiStartTimeIso` will be `null`.

### GET /dashboard/

Serves the static Vue.js dashboard application. This is a full-featured web interface for browsing, searching, and managing clipboard items.

**Features:**
- Interactive item browsing with infinite scroll
- Real-time search with live results
- Multi-select and bulk operations
- Format tabs for different data types
- Statistics with interactive charts
- Import/export functionality
- Settings management

**Response:** HTML application with embedded JavaScript and CSS

**Example:**
```bash
curl {{URL}}/
```

---

### Items Management

#### GET /items

Retrieve a list of clipboard items with optional filtering.

**Query Parameters:**
- `offset` (number, optional): Skip N most recent items (default: 0)
- `count` (number, optional): Maximum items to return (default: all)
- `ids` (string, optional): Comma-separated list of selectors to retrieve specific items
- `sort` (string, optional): Sort order (`date`, `copies`, `type`). Default: `date`

**Response:** Array of `ClipboardJsonItem` objects

**Examples:**

Get 10 most recent items:
```bash
curl "{{URL}}/items?count=10"
```

Get items starting from offset 20:
```bash
curl "{{URL}}/items?offset=20&count=10"
```

Get specific items by hash or offset:
```bash
curl "{{URL}}/items?ids=0,1,a1b2c3d4e5f6"
```

**Response Example:**
```json
[
  {
    "hash": "abc123...",
    "offset": 0,
    "timestamp": "2025-11-02T10:30:00Z",
    "lastSeen": "2025-11-02T10:30:00Z",
    "copyCount": 0,
    "kind": "text",
    "summary": "Sample clipboard content",
    "byteSize": 23,
    "sources": ["clipboard"],
    "plugins": ["text"],
    "metadata": {
      "text": {"length": 23, "lines": 1}
    }
  }
]
```

---

#### GET /item/:selector

Retrieve metadata and summary for a single clipboard item.

**Path Parameters:**
- `selector` (string): Hash or offset identifier

**Response:** `ClipboardJsonItem` object

**Examples:**

Get most recent item:
```bash
curl {{URL}}/item/0
```

Get item by hash:
```bash
curl {{URL}}/item/abc123def456
```

**Error Responses:**
- `404 Not Found`: Item doesn't exist
- `500 Internal Server Error`: Failed to load item

---

#### GET /item/:selector/data

Retrieve complete data for a single clipboard item, including all content.

**Path Parameters:**
- `selector` (string): Hash or offset identifier

**Response:** `ClipboardJsonFullItem` object with complete plugin data

**Example:**
```bash
curl {{URL}}/item/0/data
```

**Response Example:**
```json
{
  "hash": "abc123...",
  "offset": 0,
  "timestamp": "2025-11-02T10:30:00Z",
  "lastSeen": "2025-11-02T10:30:00Z",
  "copyCount": 0,
  "kind": "text",
  "summary": "Sample clipboard content",
  "byteSize": 23,
  "sources": ["clipboard"],
  "plugins": [
    {
      "id": "text",
      "data": "Sample clipboard content"
    }
  ]
}
```

**Use Cases:**
- Retrieving full content for display
- Exporting clipboard items
- Copying items to clipboard programmatically

---

#### GET /item/:selector/preview

Retrieve an HTML preview for a single clipboard item.

**Path Parameters:**
- `selector` (string): Hash or offset identifier

**Response:** HTML content (text/html)

**Example:**
```bash
curl {{URL}}/item/0/preview
```

**Use Cases:**
- Displaying item previews in the dashboard
- Embedding previews in other applications

---

#### DELETE /item/:selector

Delete a clipboard item from history.

**Path Parameters:**
- `selector` (string): Hash or offset identifier

**Response:** 204 No Content (success), no body

**Example:**
```bash
curl -X DELETE {{URL}}/item/0
```

**Error Responses:**
- `404 Not Found`: Item doesn't exist
- `500 Internal Server Error`: Failed to delete item

**Note:** This operation:
- Removes the item from the index
- Deletes associated files from disk
- Cannot be undone

---

#### PUT /item/:selector

Increment the copy count for an item without copying to clipboard.

**Path Parameters:**
- `selector` (string): Hash or offset identifier

**Response:** `ClipboardJsonItem` object with updated copy count

**Example:**
```bash
curl -X PUT {{URL}}/item/0
```

**Use Cases:**
- Tracking usage statistics
- Manual counter updates
- Analytics without clipboard modification

---

#### POST /item/:selector/copy

Copy a clipboard item to the system clipboard and increment its copy count.

**Path Parameters:**
- `selector` (string): Hash or offset identifier

**Response:** 200 OK with updated `ClipboardJsonItem` object

**Example:**
```bash
curl -X POST {{URL}}/item/0/copy
```

**Response Example:**
```json
{
  "hash": "abc123...",
  "offset": 0,
  "timestamp": "2025-11-02T10:30:00Z",
  "lastSeen": "2025-11-02T10:30:00Z",
  "copyCount": 1,
  "kind": "text",
  "summary": "Sample clipboard content",
  "byteSize": 23,
  "sources": ["clipboard"],
  "plugins": ["text"],
  "metadata": {
    "text": {"length": 23, "lines": 1}
  }
}
```

**Behavior:**
- Sets system clipboard to the item's content
- Preserves all original formats (text, HTML, images, etc.)
- Increments the `copyCount` field
- Returns updated metadata

---

### Search

#### GET /search

Search clipboard history using full-text search.

**Query Parameters:**
- `query` (string, required): Search query text
- `offset` (number, optional): Skip N results (default: 0)
- `count` (number, optional): Maximum results to return (default: 50)
- `sort` (string, optional): Sort order (`date`, `copies`, `type`, `relevance`). Default: `relevance`

**Response:** Array of matching `ClipboardJsonItem` objects

**Examples:**

Basic search:
```bash
curl "{{URL}}/search?query=important"
```

Paginated search:
```bash
curl "{{URL}}/search?query=meeting&offset=0&count=10"
```

**Search Behavior:**
- Case-insensitive matching
- Searches in content summaries and metadata
- Returns items ranked by relevance (most recent first)
- Empty query returns error

**Error Responses:**
- `400 Bad Request`: Empty or missing query parameter

---

### Statistics

#### GET /stats

Retrieve library statistics including total items, size, and historical breakdown.

**Response:**
```json
{
  "totalItems": 150,
  "totalSize": 1048576,
  "typeCounts": {
    "text": 100,
    "image": 40,
    "file": 10
  },
  "history": {
    "2025-11-01": {
      "text": {
        "count": 15,
        "ids": ["hash1", "hash2", ...]
      },
      "image": {
        "count": 5,
        "ids": ["hash3", ...]
      }
    }
  }
}
```

**Example:**
```bash
curl {{URL}}/stats
```

---

### Configuration

#### GET /mtime

Get the timestamp of the most recently added clipboard item.

**Response:**
```json
{
  "lastModified": "2025-11-02T10:30:00Z",
  "id": "abc123..."
}
```

**Fields:**
- `lastModified` (string|null): ISO 8601 timestamp of most recent item
- `id` (string|null): Hash of most recent item

**Example:**
```bash
curl {{URL}}/mtime
```

**Use Cases:**
- Polling for changes
- Synchronization
- Cache invalidation

---

#### GET /dir

Get the current data directory path.

**Response:**
```json
{
  "path": "/Users/username/.local/share/clipboard_manager"
}
```

**Example:**
```bash
curl {{URL}}/dir
```

---

#### POST /dir

Update the data directory path.

**Request Body:**
```json
{
  "mode": "move",
  "path": "/new/path/to/data"
}
```

**Fields:**
- `mode` (string): Operation mode
  - `"move"`: Move existing data to new location
  - `"update"`: Change path without moving (data must already exist)
- `path` (string): Absolute path to new data directory

**Response:**
```json
{
  "path": "/new/path/to/data"
}
```

**Example (Move Data):**
```bash
curl -X POST {{URL}}/dir \
  -H "Content-Type: application/json" \
  -d '{"mode": "move", "path": "/Users/me/clipboard_backup"}'
```

**Example (Update Path):**
```bash
curl -X POST {{URL}}/dir \
  -H "Content-Type: application/json" \
  -d '{"mode": "update", "path": "/existing/data/path"}'
```

**Error Responses:**
- `400 Bad Request`: Invalid mode or path
- `500 Internal Server Error`: Failed to move/update directory

---

### Clipboard Operations

#### POST /copy

Copy provided JSON data directly to the system clipboard without saving to history.

**Request Body:** `ClipboardJsonFullItem` object

**Response:** 204 No Content (success)

**Example:**
```bash
curl -X POST {{URL}}/copy \
  -H "Content-Type: application/json" \
  -d '{
    "hash": "temporary",
    "offset": 0,
    "timestamp": "2025-11-02T10:30:00Z",
    "lastSeen": "2025-11-02T10:30:00Z",
    "copyCount": 0,
    "kind": "text",
    "summary": "Test content",
    "byteSize": 12,
    "sources": ["api"],
    "plugins": [
      {
        "id": "text",
        "data": "Test content"
      }
    ]
  }'
```

**Behavior:**
- Sets system clipboard immediately
- Does NOT save to history
- Useful for temporary clipboard operations
- Supports all plugin formats

**Use Cases:**
- Programmatic clipboard injection
- Testing
- Integration with other tools

---

#### POST /save

Save provided JSON data to clipboard history and optionally copy to system clipboard.

**Request Body:** `ClipboardJsonFullItem` object

**Response:** Updated `ClipboardJsonFullItem` with assigned hash and metadata

**Example:**
```bash
curl -X POST {{URL}}/save \
  -H "Content-Type: application/json" \
  -d '{
    "hash": "new-item",
    "offset": 0,
    "timestamp": "2025-11-02T10:30:00Z",
    "lastSeen": "2025-11-02T10:30:00Z",
    "copyCount": 0,
    "kind": "text",
    "summary": "Saved content",
    "byteSize": 13,
    "sources": ["api"],
    "plugins": [
      {
        "id": "text",
        "data": "Saved content"
      }
    ]
  }'
```

**Response Example:**
```json
{
  "hash": "def789...",
  "offset": 0,
  "timestamp": "2025-11-02T10:30:00Z",
  "lastSeen": "2025-11-02T10:30:00Z",
  "copyCount": 0,
  "kind": "text",
  "summary": "Saved content",
  "byteSize": 13,
  "sources": ["api"],
  "plugins": [
    {
      "id": "text",
      "data": "Saved content"
    }
  ]
}
```

**Behavior:**
- Computes content hash
- Saves files to data directory
- Updates search index
- Returns complete item with assigned hash
- Hash in request is ignored; computed from content

**Use Cases:**
- Importing clipboard items
- Bulk data loading
- Synchronization
- External integrations

---

## Error Handling

The API uses standard HTTP status codes and returns JSON error objects.

### Status Codes

- `200 OK`: Successful request
- `204 No Content`: Successful request with no response body
- `400 Bad Request`: Invalid request parameters
- `404 Not Found`: Resource doesn't exist
- `500 Internal Server Error`: Server-side error

### Error Response Format

```json
{
  "error": "Descriptive error message"
}
```

### Common Errors

**404 - Item Not Found:**
```json
{
  "error": "Unknown item abc123"
}
```

**404 - Invalid Offset:**
```json
{
  "error": "No item at offset 999"
}
```

**400 - Empty Query:**
```json
{
  "error": "query parameter cannot be empty"
}
```

**400 - Invalid Mode:**
```json
{
  "error": "Unsupported mode invalid"
}
```

**500 - Internal Error:**
```json
{
  "error": "Failed to access clipboard: permission denied"
}
```

---

## Examples

### Common Workflows

#### 1. Get Recent Clipboard History

```bash
# Get 20 most recent items
curl "{{URL}}/items?count=20"
```

#### 2. Search and Copy

```bash
# Search for items
curl "{{URL}}/search?query=meeting+notes"

# Copy first result to clipboard
curl -X POST {{URL}}/item/0/copy
```

#### 3. Export Item Data

```bash
# Get full data for an item
curl {{URL}}/item/0/data > clipboard_item.json
```

#### 4. Import Item

```bash
# Save external data to history
curl -X POST {{URL}}/save \
  -H "Content-Type: application/json" \
  -d @clipboard_item.json
```

#### 5. Clean Up Old Items

```bash
# Get items from offset 100 onwards
curl "{{URL}}/items?offset=100&count=50" | \
  jq -r '.[].hash' | \
  while read hash; do
    curl -X DELETE "http://127.0.0.1:3000/item/$hash"
  done
```

#### 6. Monitor for Changes

```bash
# Poll for new items
LAST_ID=""
while true; do
  CURRENT=$(curl -s http://127.0.0.1:3000/mtime | jq -r '.id')
  if [ "$CURRENT" != "$LAST_ID" ]; then
    echo "New clipboard item: $CURRENT"
    LAST_ID="$CURRENT"
  fi
  sleep 2
done
```

#### 7. Backup Clipboard History

```bash
# Export all items
curl "http://127.0.0.1:3000/items" | jq -r '.[].hash' | \
  while read hash; do
    curl "http://127.0.0.1:3000/item/$hash/data" > "backup/$hash.json"
  done
```

---

## Plugin System

The clipboard manager uses a plugin architecture to handle different data formats.

### Available Plugins

#### Text Plugin
- **ID**: `text`
- **Kind**: `text`
- **Data Type**: String
- **Supports**: Plain text content

**Data Structure:**
```json
{
  "id": "text",
  "data": "Plain text content here"
}
```

**Metadata:**
```json
{
  "length": 1024,
  "lines": 15
}
```

---

#### HTML Plugin
- **ID**: `html`
- **Kind**: `text`
- **Data Type**: String
- **Supports**: HTML formatted content

**Data Structure:**
```json
{
  "id": "html",
  "data": "<p>HTML content with <b>formatting</b></p>"
}
```

**Metadata:**
```json
{
  "length": 512,
  "stripped_length": 256
}
```

---

#### RTF Plugin
- **ID**: `rtf`
- **Kind**: `text`
- **Data Type**: String (Base64)
- **Supports**: Rich Text Format

**Data Structure:**
```json
{
  "id": "rtf",
  "data": "e1xydGYxXGFuc2l..."
}
```

**Metadata:**
```json
{
  "size": 2048
}
```

---

#### Image Plugin
- **ID**: `image`
- **Kind**: `image`
- **Data Type**: String (Base64) or Object
- **Supports**: PNG, JPEG, and other image formats

**Data Structure (Base64):**
```json
{
  "id": "image",
  "data": "iVBORw0KGgoAAAANSUhEUgAA..."
}
```

**Data Structure (File Reference):**
```json
{
  "id": "image",
  "data": {
    "path": "image__data.png",
    "width": 1920,
    "height": 1080,
    "format": "png"
  }
}
```

**Metadata:**
```json
{
  "width": 1920,
  "height": 1080,
  "format": "png",
  "size": 524288
}
```

---

#### Files Plugin
- **ID**: `files`
- **Kind**: `file`
- **Data Type**: Array of file paths or objects
- **Supports**: File system references

**Data Structure (Simple):**
```json
{
  "id": "files",
  "data": [
    "/path/to/file1.txt",
    "/path/to/file2.pdf"
  ]
}
```

**Data Structure (Detailed):**
```json
{
  "id": "files",
  "data": [
    {
      "name": "file1.txt",
      "source_path": "/path/to/file1.txt",
      "size": 1024,
      "mime": "text/plain",
      "extension": "txt"
    }
  ]
}
```

**Metadata:**
```json
{
  "entries": [
    {
      "name": "file1.txt",
      "source_path": "/path/to/file1.txt",
      "size": 1024,
      "mime": "text/plain",
      "extension": "txt"
    }
  ]
}
```

---

### Plugin Behavior

1. **Multiple Plugins**: Items can have multiple plugins active simultaneously
   - Example: Text copied from a web browser may have both `text` and `html` plugins

2. **Priority**: Plugins have priority values that determine processing order
   - Higher priority plugins are processed first
   - Affects summary generation and display

3. **Import/Export**: Each plugin handles its own serialization
   - JSON import validates and reconstructs content
   - Export generates appropriate JSON structures
   - Reversible: import(export(data)) == data

4. **Metadata**: Plugins store format-specific metadata
   - Stored in `EntryMetadata.extra.plugins[plugin_id]`
   - Used for reconstruction without reading files
   - Enables efficient queries

5. **Summary Generation**: Each plugin computes its own summary
   - Text: First 200 characters
   - HTML: Stripped text preview
   - Image: Dimensions and format
   - Files: File names and sizes
