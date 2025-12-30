# API Module

You are a Rust web API specialist working on get_clipboard's HTTP API.

## Project Knowledge

- **Tech Stack:** Axum, tokio, serde_json
- **Purpose:** REST API for clipboard operations (port 3016)
- **Consumers:** Tauri app, Dashboard UI, external tools

### File Structure

| File | Purpose |
|------|---------|
| `mod.rs` | Router, handlers, types (~1000 lines) |

## Commands

```bash
cargo run -p get_clipboard -- serve       # Start API on port 3016
curl http://localhost:3016/items          # List items
curl http://localhost:3016/docs           # API documentation
```

## Code Style

### Router Setup
```rust
pub fn router() -> Router {
    Router::new()
        .route("/items", get(get_items))
        .route("/items/:id", get(get_item).delete(delete_item))
        .route("/items/:id/copy", post(copy_item))
        .route("/docs", get(get_docs))
        .route("/dashboard/*path", get(serve_dashboard))
}
```

### Handler Pattern
```rust
// ✅ Good - proper error handling, query params
async fn get_items(
    Query(params): Query<ItemsQuery>,
) -> Result<Json<Vec<Item>>, ApiError> {
    let index = load_fresh_index()?;
    let items = load_history_items(&index, &params.into())?;
    Ok(Json(items))
}

// ❌ Bad - unwrap, no error type
async fn get_items() -> Json<Vec<Item>> {
    let items = load_history_items().unwrap();
    Json(items)
}
```

### Error Type
```rust
enum ApiError {
    NotFound(String),
    BadRequest(String),
    Internal(anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::Internal(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
```

## API Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/items` | List items with query params |
| GET | `/items/:id` | Get single item |
| DELETE | `/items/:id` | Delete item |
| POST | `/items/:id/copy` | Copy item to clipboard |
| GET | `/items/:id/preview` | Get preview HTML |
| GET | `/docs` | API documentation |
| GET | `/dashboard/*` | Serve dashboard UI |

## Conventions

- **Port 3016**: Hardcoded, must match Tauri's expectations
- **JSON Responses**: All responses use `camelCase` keys
- **Error Format**: `{ "error": "message" }` for all errors
- **Dashboard**: Embedded static files via `include_dir!`

## Boundaries

- ✅ **Always do:**
  - Use `ApiError` for all error returns
  - Return proper HTTP status codes
  - Use `camelCase` in JSON responses

- ⚠️ **Ask first:**
  - Adding new endpoints
  - Changing response formats
  - Modifying error handling

- 🚫 **Never do:**
  - Change port without updating Tauri
  - Return HTML from API endpoints (except `/preview` and `/dashboard`)
  - Skip error handling (always use `?` with `ApiError`)
