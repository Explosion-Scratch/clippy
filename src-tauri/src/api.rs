pub const API_PORT: u16 = 3016;
pub const API_BASE: &str = "http://localhost:3016";

pub fn stats_url() -> String {
    format!("{}/stats", API_BASE)
}

pub fn items_url(count: usize) -> String {
    format!("{}/items?count={}", API_BASE, count)
}

pub fn item_preview_url(id: &str, interactive: bool) -> String {
    format!("{}/item/{}/preview?interactive={}", API_BASE, id, interactive)
}

pub fn item_data_url(id: &str) -> String {
    format!("{}/item/{}/data", API_BASE, id)
}

pub fn item_copy_url(id: &str) -> String {
    format!("{}/item/{}/copy", API_BASE, id)
}

pub fn item_delete_url(id: &str) -> String {
    format!("{}/item/{}", API_BASE, id)
}

pub fn dashboard_url() -> String {
    format!("{}/dashboard", API_BASE)
}

pub fn dashboard_item_url(id: &str) -> String {
    format!("{}/dashboard?item={}", API_BASE, id)
}

pub fn search_url() -> String {
    format!("{}/search", API_BASE)
}

pub fn mtime_url() -> String {
    format!("{}/mtime", API_BASE)
}

pub fn dir_url() -> String {
    format!("{}/dir", API_BASE)
}

pub fn save_url() -> String {
    format!("{}/save", API_BASE)
}

pub fn item_edit_url(id: &str) -> String {
    format!("{}/item/{}", API_BASE, id)
}
