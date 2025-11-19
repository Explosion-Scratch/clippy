use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::ShellExt;

const API_BASE: &str = "http://localhost:3016";

#[derive(Serialize, Deserialize)]
struct DirResponse {
    path: String,
}

#[tauri::command]
pub async fn init_service(app: AppHandle) -> Result<String, String> {
    // Removed configure_data_dir to allow manual configuration on startup
    // configure_data_dir(app.clone()).await?;

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
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let mut url = format!("{}/items", API_BASE);

    let mut params = vec![];
    if let Some(count) = limit {
        params.push(format!("count={}", count));
    }
    if let Some(off) = offset {
        params.push(format!("offset={}", off));
    }
    if let Some(q) = query {
        params.push(format!("query={}", urlencoding::encode(&q)));
    }

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
pub async fn paste_item(_app: AppHandle, selector: String) -> Result<(), String> {
    let client = reqwest::Client::new();
    let url = format!("{}/item/{}/paste", API_BASE, selector);

    let response = client.post(&url).send().await.map_err(|e| e.to_string())?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("API error: {}", response.status()))
    }
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
pub async fn db_export_all(app: AppHandle) -> Result<String, String> {
    get_history(app, Some(999999), None, None).await
}

#[tauri::command]
pub async fn db_import_all(_app: AppHandle, _json_data: String) -> Result<String, String> {
    Err("Import not supported yet".to_string())
}

#[tauri::command]
pub async fn db_delete_all(_app: AppHandle) -> Result<String, String> {
    Err("Delete all not supported yet".to_string())
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
