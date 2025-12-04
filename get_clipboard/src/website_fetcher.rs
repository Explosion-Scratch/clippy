use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct WebsiteData {
    pub title: String,
    pub description: String,
    pub favicon: String,
    pub og_image: String,
    pub color: Option<String>,
    pub image_alt: Option<String>,
}

#[derive(Debug, Default)]
struct RawMeta {
    title: Option<String>,
    description: Option<String>,
    theme_color: Option<String>,
    og: HashMap<String, String>,
    twitter: HashMap<String, String>,
    links: HashMap<String, String>,
    other: HashMap<String, String>,
}

fn extract_raw_meta(document: &Html, base_url: &Url) -> RawMeta {
    let mut meta = RawMeta::default();

    if let Ok(sel) = Selector::parse("title") {
        if let Some(el) = document.select(&sel).next() {
            meta.title = Some(el.text().collect::<String>().trim().to_string());
        }
    }

    if let Ok(sel) = Selector::parse("meta[property], meta[name]") {
        for el in document.select(&sel) {
            let prop = el.value().attr("property").or_else(|| el.value().attr("name"));
            let content = el.value().attr("content").or_else(|| el.value().attr("value"));

            if let (Some(prop), Some(content)) = (prop, content) {
                if let Some((prefix, suffix)) = prop.split_once(':') {
                    match prefix {
                        "og" => { meta.og.insert(suffix.to_string(), content.to_string()); }
                        "twitter" => { meta.twitter.insert(suffix.to_string(), content.to_string()); }
                        _ => { meta.other.insert(prop.to_string(), content.to_string()); }
                    }
                } else {
                    match prop {
                        "description" => meta.description = Some(content.to_string()),
                        "theme-color" => meta.theme_color = Some(content.to_string()),
                        _ => { meta.other.insert(prop.to_string(), content.to_string()); }
                    }
                }
            }
        }
    }

    if let Ok(sel) = Selector::parse("link[rel]") {
        for el in document.select(&sel) {
            if let (Some(rel), Some(href)) = (el.value().attr("rel"), el.value().attr("href")) {
                let resolved = base_url.join(href).map(|u| u.to_string()).unwrap_or_else(|_| href.to_string());
                meta.links.insert(rel.to_string(), resolved);
            }
        }
    }

    meta
}

fn parse_meta(meta: RawMeta) -> WebsiteData {
    let image = meta.og.get("image")
        .or_else(|| meta.twitter.get("image:src"))
        .or_else(|| meta.twitter.get("image"))
        .or_else(|| meta.other.get("image"))
        .cloned()
        .unwrap_or_default();

    let title = meta.title
        .or_else(|| meta.twitter.get("title").cloned())
        .or_else(|| meta.og.get("title").cloned())
        .or_else(|| meta.og.get("site_name").cloned())
        .unwrap_or_else(|| "Title not found".to_string());

    let description = meta.description
        .or_else(|| meta.og.get("description").cloned())
        .or_else(|| meta.twitter.get("description").cloned())
        .unwrap_or_else(|| "Description not found".to_string());

    let image_alt = meta.og.get("image:alt").cloned();

    let color = meta.theme_color;

    let favicon = meta.links.get("icon")
        .or_else(|| meta.links.get("favicon"))
        .or_else(|| meta.links.get("alternate icon"))
        .or_else(|| meta.links.get("shortcut icon"))
        .or_else(|| meta.links.get("apple-touch-icon"))
        .or_else(|| meta.links.get("fluid-icon"))
        .cloned()
        .unwrap_or_else(|| "Favicon not found".to_string());

    WebsiteData {
        title,
        description,
        favicon,
        og_image: image,
        color,
        image_alt,
    }
}

pub fn fetch_website_data(url: &Url) -> Result<WebsiteData, Box<dyn Error + Send + Sync>> {
    let response = ureq::get(url.as_str())
        .set("User-Agent", "clippy-clipboard-manager/0.1.0")
        .call()?;

    let body = response.into_string()?;
    let document = Html::parse_document(&body);
    let raw_meta = extract_raw_meta(&document, url);
    let website_data = parse_meta(raw_meta);

    Ok(website_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    #[test]
    fn test_fetch_website_data_successful() {
        let url = Url::parse("https://github.com").unwrap();
        let result = fetch_website_data(&url);
        assert!(result.is_ok());
        let website_data = result.unwrap();
        assert!(!website_data.title.is_empty());
    }
}
