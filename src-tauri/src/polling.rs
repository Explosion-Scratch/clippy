use crate::api;
use tauri::Emitter;

pub fn start_mtime_polling(app_handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let client = reqwest::Client::new();
        let mtime_url = api::mtime_url();
        let mut last_known_id: Option<String> = None;

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            match client.get(&mtime_url).send().await {
                Ok(response) => {
                    if let Ok(json) = response.json::<serde_json::Value>().await {
                        if let Some(id) = json["id"].as_str() {
                            let id_str = id.to_string();
                            let should_emit = match &last_known_id {
                                Some(known) => known != &id_str,
                                None => true,
                            };

                            if should_emit {
                                last_known_id = Some(id_str.clone());
                                println!("Clipboard changed: {}", id_str);
                                if let Err(e) = app_handle.emit("clipboard-changed", id_str) {
                                    eprintln!("Failed to emit clipboard-changed event: {}", e);
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    // API not available, silently continue
                }
            }
        }
    });
}
