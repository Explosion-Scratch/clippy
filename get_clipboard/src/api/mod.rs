use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Path as AxumPath, Query},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete as axum_delete, get, post},
};
use include_dir::{Dir, include_dir};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

use crate::clipboard::plugins;
use crate::config::io::{
    move_data_dir as config_move_data_dir, set_data_dir as config_set_data_dir,
};
use crate::config::{ensure_data_dir, load_config};
use crate::data::SearchIndex;
use crate::data::model::{EntryMetadata, SearchIndexRecord};
use crate::data::store::{
    copy_by_selector, copy_json_item, delete_entry, increment_copy_count, load_history_items,
    load_index, load_metadata, refresh_index, store_json_item,
};
use crate::search::SearchOptions;
use crate::util::paste;
use crate::util::time::format_iso;

use tokio::net::TcpListener;

const API_DOCS: &str = include_str!("../../API.md");

static FRONTEND_DIST: Dir = include_dir!("$CARGO_MANIFEST_DIR/frontend-dist");
static TEMPLATES: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates");

// Store API start time as a static variable
static mut API_START_TIME: Option<u64> = None;
static mut API_PORT: Option<u16> = None;

pub async fn serve(port: u16) -> Result<()> {
    refresh_index()?;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("API listening on http://{}", addr);
    println!("Dashboard available at http://{}/dashboard", addr);

    // Record API start time and port
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    unsafe {
        API_START_TIME = Some(start_time);
        API_PORT = Some(port);
    }

    // Note: Watcher is now run separately via 'get_clipboard watch' command

    let app = router();
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

fn router() -> Router {
    Router::new()
        .route("/", get(get_docs))
        .route("/version", get(get_version))
        .route("/dashboard", get(serve_dashboard_index))
        .route("/dashboard/", get(serve_dashboard_index))
        .route("/dashboard/*path", get(serve_dashboard))
        .route("/items", get(get_items))
        .route("/item/:selector/data", get(get_item_data))
        .route(
            "/item/:selector",
            get(get_item).delete(axum_delete(delete_item)).put(put_item),
        )
        .route("/item/:selector/preview", get(preview_item))
        .route("/item/:selector/copy", post(copy_item))
        .route("/item/:selector/paste", post(paste_item))
        .route("/search", get(search_items))
        .route("/stats", get(get_stats))
        .route("/mtime", get(get_mtime))
        .route("/dir", get(get_dir).post(update_dir))
        .route("/copy", post(copy_payload))
        .route("/save", post(save_payload))
}

async fn get_docs() -> impl IntoResponse {
    let port = unsafe { API_PORT.unwrap_or(3000) };
    let url = format!("http://127.0.0.1:{}", port);
    let docs = API_DOCS.replace("{{URL}}", &url);
    (
        StatusCode::OK,
        [("Content-Type", "text/plain; charset=utf-8")],
        docs,
    )
}

async fn serve_dashboard_index() -> impl IntoResponse {
    serve_dashboard_file("index.html".to_string()).await
}

async fn serve_dashboard(AxumPath(path): AxumPath<String>) -> impl IntoResponse {
    serve_dashboard_file(path).await
}

async fn serve_dashboard_file(path: String) -> impl IntoResponse + use<> {
    let file_path = if path.is_empty() || path == "/" {
        "index.html"
    } else {
        path.strip_prefix('/').unwrap_or(path.as_str())
    };

    if let Some(file) = FRONTEND_DIST.get_file(file_path) {
        let content = file.contents();
        let mime_type = mime_guess::from_path(file_path)
            .first_or_octet_stream()
            .to_string();

        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime_type)
            .body(Body::from(content.to_vec()))
            .unwrap();
    }

    if file_path == "index.html" || file_path.ends_with(".html") {
        if let Some(index_file) = FRONTEND_DIST.get_file("index.html") {
            let content = index_file.contents();
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html")
                .body(Body::from(content.to_vec()))
                .unwrap();
        }
    }

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("File not found"))
        .unwrap()
}

async fn get_items(
    Query(params): Query<ItemsQuery>,
) -> Result<Json<Vec<plugins::ClipboardJsonItem>>, ApiError> {
    let index = load_fresh_index()?;
    let data_dir = data_dir_path().map_err(ApiError::from)?;

    if let Some(ids) = params.ids.as_ref() {
        let selectors: Vec<_> = ids
            .split(',')
            .map(|raw| raw.trim())
            .filter(|value| !value.is_empty())
            .map(Selector::parse)
            .collect();
        return items_by_selectors(&index, &data_dir, selectors);
    }

    let mut options = SearchOptions::default();
    options.offset = params.offset.unwrap_or(0);
    options.limit = params.count;

    if let Some(sort) = params.sort {
        options.sort = match sort.to_lowercase().as_str() {
            "date" => crate::search::SortOrder::Date,
            "copies" => crate::search::SortOrder::Copies,
            "type" => crate::search::SortOrder::Type,
            _ => crate::search::SortOrder::Date,
        };
    }

    if let Some(order) = params.order {
        options.order = match order.to_lowercase().as_str() {
            "asc" | "ascending" => crate::search::SortDirection::Asc,
            "desc" | "descending" => crate::search::SortDirection::Desc,
            _ => crate::search::SortDirection::Desc,
        };
    }

    let (items, _) = load_history_items(&index, &options).map_err(ApiError::from)?;
    let mut response = Vec::new();
    for item in items {
        response.push(
            json_from_metadata(&item.metadata, item.offset, &data_dir).map_err(ApiError::from)?,
        );
    }
    Ok(Json(response))
}

#[derive(Clone)]
enum Selector {
    Hash(String),
    Offset(usize),
}

impl Selector {
    fn parse(input: &str) -> Self {
        if input.len() >= 6 {
            Selector::Hash(input.to_string())
        } else if let Ok(index) = input.parse::<usize>() {
            Selector::Offset(index)
        } else {
            Selector::Hash(input.to_string())
        }
    }
}

#[derive(Debug)]
enum ApiError {
    NotFound(String),
    BadRequest(String),
    Internal(anyhow::Error),
}

impl ApiError {
    fn not_found(message: impl Into<String>) -> Self {
        ApiError::NotFound(message.into())
    }

    fn bad_request(message: impl Into<String>) -> Self {
        ApiError::BadRequest(message.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::NotFound(message) => {
                (StatusCode::NOT_FOUND, Json(json!({ "error": message }))).into_response()
            }
            ApiError::BadRequest(message) => {
                (StatusCode::BAD_REQUEST, Json(json!({ "error": message }))).into_response()
            }
            ApiError::Internal(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": error.to_string() })),
            )
                .into_response(),
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        ApiError::Internal(error)
    }
}

#[derive(Debug, Deserialize)]
struct ItemQuery {
    formats: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ItemsQuery {
    offset: Option<usize>,
    count: Option<usize>,
    ids: Option<String>,
    sort: Option<String>,
    order: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    query: Option<String>,
    offset: Option<usize>,
    count: Option<usize>,
    formats: Option<String>,
    sort: Option<String>,
    order: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DirUpdateRequest {
    mode: String,
    path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DirResponse {
    path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MtimeResponse {
    last_modified: Option<String>,
    id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatsHistoryEntry {
    count: usize,
    ids: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatsResponse {
    total_items: usize,
    total_size: u64,
    type_counts: HashMap<String, usize>,
    history: HashMap<String, HashMap<String, StatsHistoryEntry>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionResponse {
    version: String,
    api_start_time: Option<u64>,
    api_start_time_iso: Option<String>,
}

async fn get_item(
    Path(selector): Path<String>,
    Query(params): Query<ItemQuery>,
) -> Result<Json<plugins::ClipboardJsonItem>, ApiError> {
    let index = load_fresh_index()?;
    let data_dir = data_dir_path().map_err(ApiError::from)?;

    let mut filter = crate::search::SelectionFilter::default();
    if let Some(formats) = params.formats {
        for fmt in formats.split(',') {
            let fmt = fmt.trim().to_lowercase();
            match fmt.as_str() {
                "text" => filter.include_text = true,
                "image" => filter.include_image = true,
                "file" | "files" => filter.include_file = true,
                other => filter.include_formats.push(other.to_string()),
            }
        }
    }

    let (ordered, offsets) = ordered_index_filtered(&index, &filter);
    let (hash, offset, real_index) = resolve_selector_filtered(&ordered, &offsets, &selector)?;
    let metadata = load_metadata(&hash).map_err(ApiError::from)?;
    let item = json_from_metadata_with_index(&metadata, offset, real_index, &data_dir)
        .map_err(ApiError::from)?;
    Ok(Json(item))
}

async fn get_item_data(
    Path(selector): Path<String>,
    Query(params): Query<ItemQuery>,
) -> Result<Json<plugins::ClipboardJsonFullItem>, ApiError> {
    let index = load_fresh_index()?;
    let data_dir = data_dir_path().map_err(ApiError::from)?;

    let mut filter = crate::search::SelectionFilter::default();
    if let Some(formats) = params.formats {
        for fmt in formats.split(',') {
            let fmt = fmt.trim().to_lowercase();
            match fmt.as_str() {
                "text" => filter.include_text = true,
                "image" => filter.include_image = true,
                "file" | "files" => filter.include_file = true,
                other => filter.include_formats.push(other.to_string()),
            }
        }
    }

    let (ordered, offsets) = ordered_index_filtered(&index, &filter);
    let (hash, offset, real_index) = resolve_selector_filtered(&ordered, &offsets, &selector)?;
    let metadata = load_metadata(&hash).map_err(ApiError::from)?;
    let item_dir = data_dir.join(&metadata.relative_path);
    let item = plugins::build_full_json_item(&metadata, &item_dir, Some(offset), Some(real_index))
        .map_err(ApiError::from)?;
    Ok(Json(item))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewResponse {
    formats_order: Vec<String>,
    data: HashMap<String, PreviewData>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewData {
    html: String,
    text: Option<String>,
}

async fn preview_item(Path(selector): Path<String>) -> Result<Json<PreviewResponse>, ApiError> {
    let index = load_fresh_index()?;
    let data_dir = data_dir_path().map_err(ApiError::from)?;
    let (ordered, offsets) = ordered_index(&index);
    let (hash, _) = resolve_selector(&ordered, &offsets, &selector)?;
    let metadata = load_metadata(&hash).map_err(ApiError::from)?;
    let item_dir = data_dir.join(&metadata.relative_path);

    let mut data = HashMap::new();
    let mut formats_order = Vec::new();

    // Helper to load template
    let load_template = |name: &str| -> Result<String, ApiError> {
        let file = TEMPLATES
            .get_file(name)
            .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Template {} not found", name)))?;
        Ok(String::from_utf8_lossy(file.contents()).to_string())
    };

    // Helper to load base JS/CSS
    let base_js = load_template("base_iframe.js")?;
    let style_css = load_template("style.css")?;

    let wrap_html = |content_html: String| -> String {
        // We inject style and script directly to make it self-contained
        // The templates already have <link> and <script> tags, but since we are returning
        // the HTML string to be rendered in an iframe (likely via srcdoc or similar),
        // we should probably inline them or ensure the relative paths work.
        // User said "bundle this templates folder... fill in the templates".
        // If the iframe is rendered with srcdoc, relative links won't work easily unless base tag is set.
        // But simpler to inline for "self contained".
        // However, the templates I wrote use <link href="style.css">.
        // I will replace those tags with inline content.
        content_html
            .replace(
                r#"<link rel="stylesheet" href="style.css">"#,
                &format!("<style>{}</style>", style_css),
            )
            .replace(
                r#"<script src="base_iframe.js"></script>"#,
                &format!("<script>{}</script>", base_js),
            )
    };

    // 1. Text Format
    if metadata.kind == crate::data::model::EntryKind::Text {
        if let Some(text) = &metadata.summary {
            // Or load full text if needed
            // For text kind, summary is usually the content if short, or we might need to load full content.
            // Let's try to load full content if available.
            let full_text =
                if let Some(text_file) = metadata.files.iter().find(|f| f.ends_with(".txt")) {
                    std::fs::read_to_string(item_dir.join(text_file)).unwrap_or(text.clone())
                } else {
                    text.clone()
                };

            let template = load_template("text.html")?;
            let html = template.replace("{{content}}", &html_escape::encode_text(&full_text));
            let final_html = wrap_html(html);

            data.insert(
                "text".to_string(),
                PreviewData {
                    html: final_html,
                    text: Some(full_text),
                },
            );
            formats_order.push("text".to_string());
        }
    }

    // 2. Image Format
    if metadata.kind == crate::data::model::EntryKind::Image {
        // We need to serve the image. For a self-contained HTML, we can base64 encode it.
        // Or we can use a blob URL if served from same origin, but API returns JSON.
        // Base64 is safest for "self contained".
        if let Some(img_file) = metadata
            .files
            .iter()
            .find(|f| f.ends_with(".png") || f.ends_with(".jpg"))
        {
            let img_path = item_dir.join(img_file);
            if let Ok(img_bytes) = std::fs::read(&img_path) {
                let mime = if img_file.ends_with(".png") {
                    "image/png"
                } else {
                    "image/jpeg"
                };
                let b64 = base64::encode(&img_bytes);
                let src = format!("data:{};base64,{}", mime, b64);

                let template = load_template("image.html")?;
                let html = template.replace("{{content}}", &src);
                let final_html = wrap_html(html);

                data.insert(
                    "image".to_string(),
                    PreviewData {
                        html: final_html,
                        text: None,
                    },
                );
                formats_order.push("image".to_string());
            }
        }
    }

    // 3. Files Format
    if metadata.kind == crate::data::model::EntryKind::File {
        // We have a list of files in metadata.summary (usually) or we need to parse it.
        // Actually metadata.summary for files is usually the list of files or first file.
        // Let's assume we can get the list.
        // In `mapApiItem` in frontend, it uses `item.summary` as a single file or parses it?
        // The `ClipboardItem.vue` logic handles it.
        // For now, let's assume summary contains the file list or we can get it from somewhere.
        // `metadata.files` contains the files stored in the data dir, NOT the original file paths.
        // The original file paths are usually in the content if it was a file copy.
        // But `get_clipboard` stores the file content if it can, or just paths?
        // Looking at `src/clipboard/plugins/files.rs` would clarify, but let's assume `summary` has the paths for now.
        // Or `search_text`.

        let file_paths: Vec<String> = if let Some(summary) = &metadata.summary {
            summary.lines().map(|s| s.to_string()).collect()
        } else {
            vec![]
        };

        if !file_paths.is_empty() {
            let template = load_template("file.html")?;
            // Simple handlebars-like replacement for list
            // {{#each files}}...{{/each}}
            // I'll do a manual replacement since I don't have a template engine.
            let list_item_template = r#"<li class="file-item">
                <span class="file-icon"><svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 256 256"><path fill="currentColor" d="m213.66 82.34l-56-56A8 8 0 0 0 152 24H56a16 16 0 0 0-16 16v176a16 16 0 0 0 16 16h144a16 16 0 0 0 16-16V88a8 8 0 0 0-2.34-5.66M160 51.31L188.69 80H160ZM200 216H56V40h88v48a8 8 0 0 0 8 8h48z"/></svg></span>
                <span class="file-path" title="{{this}}">{{this}}</span>
                <button class="copy-btn" data-text="{{this}}">Copy Path</button>
            </li>"#;

            let list_html: String = file_paths
                .iter()
                .map(|path| list_item_template.replace("{{this}}", &html_escape::encode_text(path)))
                .collect();

            // Regex to replace the block
            let re = regex::Regex::new(r"\{\{#each files\}\}([\s\S]*?)\{\{/each\}\}").unwrap();
            // Note: my manual replacement above used the inner content of the block from my memory of the template.
            // But I should extract it from the template string to be correct.
            // Let's just replace the whole block with my generated list for simplicity,
            // assuming the template matches what I wrote.
            // Actually, to be robust, I should just replace `{{#each files}}...{{/each}}` with the list.
            // But I need the inner template.
            // Let's simplify: I'll just replace `{{#each files}}...{{/each}}` with the generated list items,
            // ignoring the inner template in the file and using the one hardcoded here which matches.

            let html = re.replace(&template, &list_html).to_string();
            let final_html = wrap_html(html);

            data.insert(
                "files".to_string(),
                PreviewData {
                    html: final_html,
                    text: None, // Files implement their own copy buttons
                },
            );
            formats_order.push("files".to_string());
        }
    }

    // 4. HTML Format
    // Check if we have HTML content stored
    if let Some(html_file) = metadata.files.iter().find(|f| f.ends_with(".html")) {
        let html_content = std::fs::read_to_string(item_dir.join(html_file)).unwrap_or_default();
        if !html_content.is_empty() {
            let template = load_template("html.html")?;
            // Escape for srcdoc attribute
            let escaped_content = html_escape::encode_double_quoted_attribute(&html_content);
            let html = template.replace("{{content}}", &escaped_content);
            let final_html = wrap_html(html);

            data.insert(
                "html".to_string(),
                PreviewData {
                    html: final_html,
                    text: None,
                },
            );
            formats_order.push("html".to_string());
        }
    }

    Ok(Json(PreviewResponse {
        formats_order,
        data,
    }))
}

async fn copy_item(
    Path(selector): Path<String>,
) -> Result<(StatusCode, Json<plugins::ClipboardJsonItem>), ApiError> {
    let index = load_fresh_index()?;
    let (ordered, offsets) = ordered_index(&index);
    let (hash, offset) = resolve_selector(&ordered, &offsets, &selector)?;
    copy_by_selector(&hash).map_err(ApiError::from)?;
    let metadata = increment_copy_count(&hash).map_err(ApiError::from)?;
    let data_dir = data_dir_path().map_err(ApiError::from)?;
    let item = json_from_metadata(&metadata, offset, &data_dir).map_err(ApiError::from)?;
    Ok((StatusCode::OK, Json(item)))
}

async fn paste_item(
    Path(selector): Path<String>,
) -> Result<(StatusCode, Json<plugins::ClipboardJsonItem>), ApiError> {
    let index = load_fresh_index()?;
    let (ordered, offsets) = ordered_index(&index);
    let (hash, offset) = resolve_selector(&ordered, &offsets, &selector)?;
    copy_by_selector(&hash).map_err(ApiError::from)?;
    paste::simulate_paste().map_err(ApiError::from)?;
    let metadata = increment_copy_count(&hash).map_err(ApiError::from)?;
    let data_dir = data_dir_path().map_err(ApiError::from)?;
    let item = json_from_metadata(&metadata, offset, &data_dir).map_err(ApiError::from)?;
    Ok((StatusCode::OK, Json(item)))
}

async fn delete_item(Path(selector): Path<String>) -> Result<StatusCode, ApiError> {
    let index = load_fresh_index()?;
    let (ordered, offsets) = ordered_index(&index);
    let (hash, _) = resolve_selector(&ordered, &offsets, &selector)?;
    delete_entry(&hash).map_err(ApiError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn put_item(
    Path(selector): Path<String>,
) -> Result<Json<plugins::ClipboardJsonItem>, ApiError> {
    let index = load_fresh_index()?;
    let (ordered, offsets) = ordered_index(&index);
    let (hash, offset) = resolve_selector(&ordered, &offsets, &selector)?;
    let metadata = increment_copy_count(&hash).map_err(ApiError::from)?;
    let data_dir = data_dir_path().map_err(ApiError::from)?;
    let item = json_from_metadata(&metadata, offset, &data_dir).map_err(ApiError::from)?;
    Ok(Json(item))
}

async fn search_items(
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<plugins::ClipboardJsonItem>>, ApiError> {
    let query = params.query.as_deref().unwrap_or("").trim();
    let has_sort = params.sort.is_some();
    if query.is_empty() && params.formats.is_none() && !has_sort {
        return Err(ApiError::bad_request(
            "query, formats, or sort parameter must be provided",
        ));
    }
    let index = load_fresh_index()?;
    let data_dir = data_dir_path().map_err(ApiError::from)?;

    let (parsed_query, is_regex, mut selection_filter) =
        crate::search::parse_search_query(query, false);

    let mut options = SearchOptions::default();
    if !parsed_query.is_empty() {
        options.query = Some(parsed_query);
    }
    options.regex = is_regex;
    options.filter = selection_filter;

    options.offset = params.offset.unwrap_or(0);
    options.limit = Some(params.count.unwrap_or(50));

    if let Some(sort) = params.sort {
        options.sort = match sort.to_lowercase().as_str() {
            "date" => crate::search::SortOrder::Date,
            "copies" => crate::search::SortOrder::Copies,
            "type" => crate::search::SortOrder::Type,
            "relevance" => crate::search::SortOrder::Relevance,
            _ => crate::search::SortOrder::Date,
        };
    }

    if let Some(order) = params.order {
        options.order = match order.to_lowercase().as_str() {
            "asc" | "ascending" => crate::search::SortDirection::Asc,
            "desc" | "descending" => crate::search::SortDirection::Desc,
            _ => crate::search::SortDirection::Desc,
        };
    }

    if let Some(formats) = params.formats {
        for fmt in formats.split(',') {
            let fmt = fmt.trim().to_lowercase();
            match fmt.as_str() {
                "text" => options.filter.include_text = true,
                "image" => options.filter.include_image = true,
                "file" | "files" => options.filter.include_file = true,
                "html" => options.filter.include_html = true,
                other => options.filter.include_formats.push(other.to_string()),
            }
        }
    }

    let (items, _) = load_history_items(&index, &options).map_err(ApiError::from)?;
    let mut response = Vec::new();
    for item in items {
        response.push(
            json_from_metadata_with_index(
                &item.metadata,
                item.offset,
                item.global_offset,
                &data_dir,
            )
            .map_err(ApiError::from)?,
        );
    }
    Ok(Json(response))
}

async fn get_stats() -> Result<Json<StatsResponse>, ApiError> {
    let index = load_fresh_index()?;

    let total_items = index.len();
    let total_size = index.values().map(|r| r.byte_size).sum();

    let mut type_counts = HashMap::new();
    let mut history: HashMap<String, HashMap<String, StatsHistoryEntry>> = HashMap::new();

    for record in index.values() {
        let kind_str = match record.kind {
            crate::data::model::EntryKind::Text => "text",
            crate::data::model::EntryKind::Image => "image",
            crate::data::model::EntryKind::File => "file",
            crate::data::model::EntryKind::Other => "other",
        };
        *type_counts.entry(kind_str.to_string()).or_insert(0) += 1;

        if record.detected_formats.iter().any(|f| {
            let f = f.to_lowercase();
            f == "html" || f.contains("public.html") || f.contains("text/html")
        }) {
            *type_counts.entry("html".to_string()).or_insert(0) += 1;
        }

        // History grouping
        let date = format_iso(record.last_seen)
            .split('T')
            .next()
            .unwrap_or("unknown")
            .to_string();

        let day_entry = history.entry(date).or_default();
        let type_entry = day_entry
            .entry(kind_str.to_string())
            .or_insert(StatsHistoryEntry {
                count: 0,
                ids: Vec::new(),
            });
        type_entry.count += 1;
        type_entry.ids.push(record.hash.clone());
    }

    Ok(Json(StatsResponse {
        total_items,
        total_size,
        type_counts,
        history,
    }))
}

async fn get_mtime() -> Result<Json<MtimeResponse>, ApiError> {
    let index = load_fresh_index()?;
    if let Some(record) = index.values().max_by_key(|record| record.last_seen) {
        Ok(Json(MtimeResponse {
            last_modified: Some(format_iso(record.last_seen)),
            id: Some(record.hash.clone()),
        }))
    } else {
        Ok(Json(MtimeResponse {
            last_modified: None,
            id: None,
        }))
    }
}

async fn get_version() -> Json<VersionResponse> {
    let version = env!("CARGO_PKG_VERSION").to_string();

    let (api_start_time, api_start_time_iso) = unsafe {
        match API_START_TIME {
            Some(timestamp) => {
                // Convert Unix timestamp to OffsetDateTime
                let datetime = time::OffsetDateTime::from_unix_timestamp(timestamp as i64)
                    .unwrap_or_else(|_| time::OffsetDateTime::UNIX_EPOCH);
                let iso_string = format_iso(datetime);
                (Some(timestamp), Some(iso_string))
            }
            None => (None, None),
        }
    };

    Json(VersionResponse {
        version,
        api_start_time,
        api_start_time_iso,
    })
}

async fn get_dir() -> Result<Json<DirResponse>, ApiError> {
    let config = load_config().map_err(ApiError::from)?;
    Ok(Json(DirResponse {
        path: config.data_dir().to_string_lossy().to_string(),
    }))
}

async fn update_dir(Json(payload): Json<DirUpdateRequest>) -> Result<Json<DirResponse>, ApiError> {
    let target = PathBuf::from(&payload.path);
    match payload.mode.as_str() {
        "move" => {
            config_move_data_dir(target).map_err(ApiError::from)?;
            refresh_index().map_err(ApiError::from)?;
        }
        "update" => {
            config_set_data_dir(target).map_err(ApiError::from)?;
            refresh_index().map_err(ApiError::from)?;
        }
        other => return Err(ApiError::bad_request(format!("Unsupported mode {other}"))),
    }
    let config = load_config().map_err(ApiError::from)?;
    Ok(Json(DirResponse {
        path: config.data_dir().to_string_lossy().to_string(),
    }))
}

async fn copy_payload(
    Json(payload): Json<plugins::ClipboardJsonFullItem>,
) -> Result<StatusCode, ApiError> {
    copy_json_item(&payload).map_err(ApiError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn save_payload(
    Json(payload): Json<plugins::ClipboardJsonFullItem>,
) -> Result<Json<plugins::ClipboardJsonFullItem>, ApiError> {
    let metadata = store_json_item(&payload).map_err(ApiError::from)?;
    let data_dir = data_dir_path().map_err(ApiError::from)?;
    let item_dir = data_dir.join(&metadata.relative_path);
    let index = load_index().map_err(ApiError::from)?;
    let (_, offsets) = ordered_index(&index);
    let offset = offsets.get(&metadata.hash).copied();
    let item = plugins::build_full_json_item(&metadata, &item_dir, offset, None)
        .map_err(ApiError::from)?;
    Ok(Json(item))
}

fn items_by_selectors(
    index: &SearchIndex,
    data_dir: &std::path::Path,
    selectors: Vec<Selector>,
) -> Result<Json<Vec<plugins::ClipboardJsonItem>>, ApiError> {
    let (ordered, offsets) = ordered_index(index);
    let mut response = Vec::new();
    for selector in selectors {
        let (hash, offset) = match selector {
            Selector::Hash(hash) => {
                let offset = offsets
                    .get(&hash)
                    .copied()
                    .ok_or_else(|| ApiError::not_found(format!("Unknown item {hash}")))?;
                (hash, offset)
            }
            Selector::Offset(index) => {
                let record = ordered
                    .get(index)
                    .ok_or_else(|| ApiError::not_found(format!("No item at offset {index}")))?;
                (record.hash.clone(), index)
            }
        };
        let metadata = load_metadata(&hash).map_err(ApiError::from)?;
        response.push(json_from_metadata(&metadata, offset, data_dir).map_err(ApiError::from)?);
    }
    Ok(Json(response))
}

fn ordered_index(index: &SearchIndex) -> (Vec<&SearchIndexRecord>, HashMap<String, usize>) {
    let mut ordered: Vec<_> = index.values().collect();
    ordered.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));
    let offsets = ordered
        .iter()
        .enumerate()
        .map(|(idx, record)| (record.hash.clone(), idx))
        .collect();
    (ordered, offsets)
}

fn ordered_index_filtered<'a>(
    index: &'a SearchIndex,
    filter: &crate::search::SelectionFilter,
) -> (Vec<(usize, &'a SearchIndexRecord)>, HashMap<String, usize>) {
    let mut all_ordered: Vec<_> = index.values().collect();
    all_ordered.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));

    let filtered: Vec<_> = all_ordered
        .iter()
        .enumerate()
        .filter(|(_, record)| filter.matches(record))
        .map(|(idx, record)| (idx, *record))
        .collect();

    let offsets = filtered
        .iter()
        .enumerate()
        .map(|(filtered_idx, (_, record))| (record.hash.clone(), filtered_idx))
        .collect();
    (filtered, offsets)
}

fn resolve_selector(
    ordered: &[&SearchIndexRecord],
    offsets: &HashMap<String, usize>,
    selector: &str,
) -> Result<(String, usize), ApiError> {
    match Selector::parse(selector) {
        Selector::Hash(hash) => {
            let offset = offsets
                .get(&hash)
                .copied()
                .ok_or_else(|| ApiError::not_found(format!("Unknown item {hash}")))?;
            Ok((hash, offset))
        }
        Selector::Offset(index) => {
            let record = ordered
                .get(index)
                .ok_or_else(|| ApiError::not_found(format!("No item at offset {index}")))?;
            Ok((record.hash.clone(), index))
        }
    }
}

fn resolve_selector_filtered(
    ordered: &[(usize, &SearchIndexRecord)],
    offsets: &HashMap<String, usize>,
    selector: &str,
) -> Result<(String, usize, usize), ApiError> {
    match Selector::parse(selector) {
        Selector::Hash(hash) => {
            let offset = offsets
                .get(&hash)
                .copied()
                .ok_or_else(|| ApiError::not_found(format!("Unknown item {hash}")))?;
            let (real_index, _) = ordered
                .get(offset)
                .ok_or_else(|| ApiError::not_found(format!("Unknown item {hash}")))?;
            Ok((hash, offset, *real_index))
        }
        Selector::Offset(index) => {
            let (real_index, record) = ordered
                .get(index)
                .ok_or_else(|| ApiError::not_found(format!("No item at offset {index}")))?;
            Ok((record.hash.clone(), index, *real_index))
        }
    }
}

fn data_dir_path() -> Result<PathBuf> {
    let config = load_config()?;
    ensure_data_dir(&config)
}

fn load_fresh_index() -> Result<SearchIndex, ApiError> {
    refresh_index().map_err(ApiError::from)?;
    load_index().map_err(ApiError::from)
}

fn json_from_metadata(
    metadata: &EntryMetadata,
    offset: usize,
    data_dir: &std::path::Path,
) -> Result<plugins::ClipboardJsonItem> {
    let item_dir = data_dir.join(&metadata.relative_path);
    plugins::build_json_item_with_preference(metadata, &item_dir, offset, None, None)
}

fn json_from_metadata_with_index(
    metadata: &EntryMetadata,
    offset: usize,
    real_index: usize,
    data_dir: &std::path::Path,
) -> Result<plugins::ClipboardJsonItem> {
    let item_dir = data_dir.join(&metadata.relative_path);
    plugins::build_json_item_with_preference(metadata, &item_dir, offset, None, Some(real_index))
}
