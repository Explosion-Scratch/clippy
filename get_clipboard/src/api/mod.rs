use axum::{
    Json, Router,
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete as axum_delete, get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;

use crate::clipboard::plugins;
use crate::config::io::{
    move_data_dir as config_move_data_dir, set_data_dir as config_set_data_dir,
};
use crate::config::{ensure_data_dir, load_config};
use crate::data::SearchIndex;
use crate::data::model::{EntryMetadata, SearchIndexRecord};
use crate::data::store::{
    copy_by_selector, delete_entry, increment_copy_count, load_history_items, load_index,
    load_metadata, refresh_index,
};
use crate::search::SearchOptions;
use crate::util::time::format_iso;

use tokio::net::TcpListener;

pub async fn serve(port: u16) -> Result<()> {
    refresh_index()?;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("API listening on http://{}", addr);
    let app = router();
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

fn router() -> Router {
    Router::new()
        .route("/items", get(get_items))
        .route(
            "/item/:selector",
            get(get_item).delete(axum_delete(delete_item)).put(put_item),
        )
        .route("/item/:selector/copy", post(copy_item))
        .route("/search", get(search_items))
        .route("/mtime", get(get_mtime))
        .route("/dir", get(get_dir).post(update_dir))
}

#[derive(Debug, Deserialize)]
struct ItemsQuery {
    offset: Option<usize>,
    count: Option<usize>,
    ids: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    query: String,
    offset: Option<usize>,
    count: Option<usize>,
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

    let (items, _) = load_history_items(&index, &options).map_err(ApiError::from)?;
    let mut response = Vec::new();
    for item in items {
        response.push(
            json_from_metadata(&item.metadata, item.offset, &data_dir).map_err(ApiError::from)?,
        );
    }
    Ok(Json(response))
}

async fn get_item(
    Path(selector): Path<String>,
) -> Result<Json<plugins::ClipboardJsonItem>, ApiError> {
    let index = load_fresh_index()?;
    let (ordered, offsets) = ordered_index(&index);
    let (hash, offset) = resolve_selector(&ordered, &offsets, &selector)?;
    let metadata = load_metadata(&hash).map_err(ApiError::from)?;
    let data_dir = data_dir_path().map_err(ApiError::from)?;
    let item = json_from_metadata(&metadata, offset, &data_dir).map_err(ApiError::from)?;
    Ok(Json(item))
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
    if params.query.trim().is_empty() {
        return Err(ApiError::bad_request("query parameter cannot be empty"));
    }
    let index = load_fresh_index()?;
    let data_dir = data_dir_path().map_err(ApiError::from)?;
    let mut options = SearchOptions::default();
    options.query = Some(params.query.clone());
    options.offset = params.offset.unwrap_or(0);
    options.limit = Some(params.count.unwrap_or(50));
    let (items, _) = load_history_items(&index, &options).map_err(ApiError::from)?;
    let mut response = Vec::new();
    for item in items {
        response.push(
            json_from_metadata(&item.metadata, item.offset, &data_dir).map_err(ApiError::from)?,
        );
    }
    Ok(Json(response))
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
    plugins::build_json_item_with_preference(metadata, &item_dir, offset, None)
}
