use crate::data::model::SearchIndex;
use anyhow::Result;
use regex::Regex;

pub fn query_index(index: &SearchIndex, query: &str) -> Result<Vec<String>> {
    let pattern = Regex::new(&query.to_lowercase())?;
    let mut results: Vec<_> = index
        .values()
        .filter(|record| match &record.summary {
            Some(summary) => pattern.is_match(&summary.to_lowercase()),
            None => false,
        })
        .map(|record| record.hash.clone())
        .collect();
    results.sort();
    Ok(results)
}
