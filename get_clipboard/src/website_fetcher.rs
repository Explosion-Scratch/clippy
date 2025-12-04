use reqwest::Client;
use scraper::{ Html, Selector };
use serde::{ Deserialize, Serialize };
use url::Url;
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct WebsiteData {
    pub title: String,
    pub description: String,
    pub favicon: String,
    pub og_image: String,
}

pub async fn fetch_website_data(
    http_client: &Client,
    url: &Url
) -> Result<WebsiteData, Box<dyn Error>> {
    let response = http_client.get(url.as_str()).send().await?;

    let body = response.text().await?;

    let document = Html::parse_document(&body);

    let title_selector = Selector::parse("title").unwrap();
    let description_selector = Selector::parse("meta[name='description']").unwrap();
    let favicon_selector = Selector::parse("link[rel='icon']").unwrap();
    let og_image_selector = Selector::parse("meta[property='og:image']").unwrap();

    let title = document
        .select(&title_selector)
        .next()
        .map(|title| title.text().collect::<String>())
        .unwrap_or_else(|| "Title not found".to_string());

    // Extract the website description
    let description = document
        .select(&description_selector)
        .next()
        .and_then(|desc| desc.value().attr("content"))
        .map(|desc| desc.to_string())
        .unwrap_or_else(|| "Description not found".to_string());

    // Extract the favicon URL
    let favicon = document
        .select(&favicon_selector)
        .next()
        .and_then(|icon| icon.value().attr("href"))
        .map(|icon| icon.to_string())
        .unwrap_or_else(|| "Favicon not found".to_string());

    let og_image = document
        .select(&og_image_selector)
        .next()
        .and_then(|og| og.value().attr("content"))
        .map(|og| og.to_string())
        .unwrap_or_else(|| "og:image not found".to_string());

    let website_data = WebsiteData {
        title,
        description,
        favicon,
        og_image,
    };

    Ok(website_data)
}

#[cfg(test)]
mod tests {
    // Import necessary items for testing
    use super::*;
    use reqwest::Url;

    // Define your test function
    #[tokio::test]
    async fn test_fetch_website_data_successful() {
        // Create a mock HTTP client for testing
        let http_client = reqwest::Client::new();

        // Replace this URL with a URL to a test website
        let url = Url::parse("https://github.com").unwrap();

        // Call the fetch_website_data function
        let result = fetch_website_data(&http_client, &url).await;

        // Check that the result is successful (Ok variant)
        assert!(result.is_ok());

        // Add more specific assertions based on the expected data from the test website
        let website_data = result.unwrap();
        assert!(!website_data.title.is_empty());
    }
}
