use crate::paste::simulate_system_paste_internal;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::ShellExt;

const API_BASE: &str = "http://localhost:3016";

#[derive(Serialize, Deserialize)]
struct DirResponse {
    path: String,
}

#[derive(Serialize, Deserialize)]
struct MtimeResponse {
    #[serde(rename = "lastModified")]
    last_modified: Option<String>,
    id: Option<String>,
}

#[tauri::command]
pub async fn init_service(app: AppHandle) -> Result<String, String> {
    println!("Installing clipboard service...");
    let install_output = app
        .shell()
        .sidecar("get_clipboard")
        .map_err(|e| e.to_string())?
        .args(["service", "install"])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let install_stdout = String::from_utf8_lossy(&install_output.stdout);
    let install_stderr = String::from_utf8_lossy(&install_output.stderr);
    println!("Install stdout: {}", install_stdout);

    if !install_output.status.success() {
        eprintln!("Service install warning/error: {}", install_stderr);
    }

    println!("Starting clipboard service (API server)...");
    let start_output = app
        .shell()
        .sidecar("get_clipboard")
        .map_err(|e| e.to_string())?
        .args(["service", "start"])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let start_stdout = String::from_utf8_lossy(&start_output.stdout);
    let start_stderr = String::from_utf8_lossy(&start_output.stderr);

    if !start_output.status.success() {
        if start_stderr.contains("already") {
            println!("Service start notice: {}", start_stderr);
        } else {
            eprintln!("Service start error: {}", start_stderr);
            return Err(format!("Failed to start service: {}", start_stderr));
        }
    } else {
        println!("Service started: {}", start_stdout);
    }

    // Wait a moment for the API to come up
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    Ok("Service initialized successfully".to_string())
}

#[tauri::command]
pub async fn stop_service(app: AppHandle) -> Result<String, String> {
    let sidecar = app
        .shell()
        .sidecar("get_clipboard")
        .map_err(|e| e.to_string())?;
    let output = sidecar
        .args(["service", "stop"])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub async fn get_service_status(app: AppHandle) -> Result<String, String> {
    let sidecar = app
        .shell()
        .sidecar("get_clipboard")
        .map_err(|e| e.to_string())?;
    let output = sidecar
        .args(["service", "status"])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub async fn get_history(
    _app: AppHandle,
    limit: Option<usize>,
    offset: Option<usize>,
    query: Option<String>,
    sort: Option<String>,
    order: Option<String>,
) -> Result<String, String> {
    let client = reqwest::Client::new();

    // Determine base URL based on whether a query is present
    let (base_url, is_search) = if let Some(ref q) = query {
        if !q.trim().is_empty() {
            (format!("{}/search", API_BASE), true)
        } else {
            (format!("{}/items", API_BASE), false)
        }
    } else {
        (format!("{}/items", API_BASE), false)
    };

    let mut params = vec![];
    if let Some(count) = limit {
        params.push(format!("count={}", count));
    }
    if let Some(off) = offset {
        params.push(format!("offset={}", off));
    }

    if is_search {
        if let Some(q) = query {
            params.push(format!("query={}", urlencoding::encode(&q)));
        }
    }

    if let Some(s) = sort {
        params.push(format!("sort={}", s));
    }
    if let Some(o) = order {
        params.push(format!("order={}", o));
    }

    let mut url = base_url;
    if !params.is_empty() {
        url.push('?');
        url.push_str(&params.join("&"));
    }

    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;

    if response.status().is_success() {
        Ok(response.text().await.map_err(|e| e.to_string())?)
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

#[tauri::command]
pub async fn get_mtime(_app: AppHandle) -> Result<String, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/mtime", API_BASE);
    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;

    if response.status().is_success() {
        // Just pass the JSON string through to frontend
        Ok(response.text().await.map_err(|e| e.to_string())?)
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

#[tauri::command]
pub async fn copy_item(_app: AppHandle, selector: String) -> Result<(), String> {
    let client = reqwest::Client::new();
    let url = format!("{}/item/{}/copy", API_BASE, selector);

    let response = client.post(&url).send().await.map_err(|e| e.to_string())?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

#[tauri::command]
pub async fn paste_item(app: AppHandle, selector: String) -> Result<(), String> {
    // 1. Copy to system clipboard via API (incrementing copy count)
    let client = reqwest::Client::new();
    let url = format!("{}/item/{}/copy", API_BASE, selector);
    let response = client.post(&url).send().await.map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("API error during copy: {}", response.status()));
    }

    // 2. Simulate system paste using the main app process which has permissions
    let app_clone = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        // Small delay to allow window hiding/focus switching to complete
        std::thread::sleep(std::time::Duration::from_millis(100));
        if let Err(e) = simulate_system_paste_internal(&app_clone) {
            eprintln!("Failed to simulate paste: {}", e);
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn delete_item(_app: AppHandle, selector: String) -> Result<(), String> {
    let client = reqwest::Client::new();
    let url = format!("{}/item/{}", API_BASE, selector);

    let response = client
        .delete(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

#[tauri::command]
pub async fn configure_data_dir(app: AppHandle) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let path_str = app_data_dir.to_string_lossy().to_string();

    let sidecar = app
        .shell()
        .sidecar("get_clipboard")
        .map_err(|e| e.to_string())?;
    let output = sidecar
        .args(["dir", "set", &path_str])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        println!("Configured data dir to: {}", path_str);
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub async fn db_get_count(_app: AppHandle) -> Result<usize, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/stats", API_BASE);

    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        Ok(json["totalItems"].as_u64().unwrap_or(0) as usize)
    } else {
        Ok(0)
    }
}

#[tauri::command]
pub async fn db_get_size(_app: AppHandle) -> Result<u64, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/stats", API_BASE);

    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        Ok(json["totalSize"].as_u64().unwrap_or(0))
    } else {
        Ok(0)
    }
}

#[tauri::command]
pub async fn db_export_all(_app: AppHandle) -> Result<String, String> {
    let client = reqwest::Client::new();
    // Get all items (summary) to get IDs
    let items_url = format!("{}/items?count=1000000", API_BASE);
    let resp = client
        .get(&items_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Failed to list items: {}", resp.status()));
    }
    let items: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;

    // Fetch full data for each item to ensure complete export
    let mut full_items = Vec::new();
    for item in items {
        if let Some(id) = item["hash"].as_str().or(item["id"].as_str()) {
            let data_url = format!("{}/item/{}/data", API_BASE, id);
            if let Ok(resp) = client.get(&data_url).send().await {
                if resp.status().is_success() {
                    if let Ok(full_item) = resp.json::<serde_json::Value>().await {
                        full_items.push(full_item);
                    }
                }
            }
        }
    }

    serde_json::to_string_pretty(&full_items).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_import_all(_app: AppHandle, json_data: String) -> Result<String, String> {
    let client = reqwest::Client::new();
    let items: Vec<serde_json::Value> =
        serde_json::from_str(&json_data).map_err(|e| e.to_string())?;
    let mut success = 0;
    let mut failed = 0;

    let save_url = format!("{}/save", API_BASE);

    for item in items {
        let resp = client.post(&save_url).json(&item).send().await;
        match resp {
            Ok(r) if r.status().is_success() => success += 1,
            Ok(r) => {
                println!("Failed to import item: {}", r.status());
                failed += 1;
            }
            Err(e) => {
                println!("Request failed: {}", e);
                failed += 1;
            }
        }
    }

    Ok(format!("Imported {} items. Failed: {}", success, failed))
}

#[tauri::command]
pub async fn db_delete_all(_app: AppHandle) -> Result<String, String> {
    let client = reqwest::Client::new();
    let items_url = format!("{}/items?count=1000000", API_BASE);
    let resp = client
        .get(&items_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let items: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;

    let mut count = 0;
    for item in items {
        if let Some(id) = item["hash"].as_str().or(item["id"].as_str()) {
            let del_url = format!("{}/item/{}", API_BASE, id);
            if let Ok(resp) = client.delete(&del_url).send().await {
                if resp.status().is_success() {
                    count += 1;
                }
            }
        }
    }
    Ok(format!("Deleted {} items", count))
}

#[tauri::command]
pub async fn get_sidecar_dir(_app: AppHandle) -> Result<String, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/dir", API_BASE);
    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if response.status().is_success() {
        let json: DirResponse = response.json().await.map_err(|e| e.to_string())?;
        Ok(json.path)
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

#[tauri::command]
pub async fn set_sidecar_dir(
    _app: AppHandle,
    mode: String,
    path: String,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/dir", API_BASE);
    let body = serde_json::json!({
        "mode": mode,
        "path": path
    });

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let json: DirResponse = response.json().await.map_err(|e| e.to_string())?;
        Ok(json.path)
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

#[tauri::command]
pub async fn get_app_data_dir(app: AppHandle) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(app_data_dir.to_string_lossy().to_string())
}
