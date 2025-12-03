use crate::data::model::{EntryKind, SearchIndex, SearchIndexRecord};
use crate::util::time::OffsetDateTime;

#[derive(Debug, Clone, Default)]
pub struct SelectionFilter {
    pub include_text: bool,
    pub include_image: bool,
    pub include_file: bool,
    pub include_other: bool,
    pub include_html: bool,
    pub include_formats: Vec<String>,
}

impl SelectionFilter {
    pub fn matches(&self, record: &SearchIndexRecord) -> bool {
        let kind_filter_active = self.include_text
            || self.include_image
            || self.include_file
            || self.include_other
            || self.include_html;
        let format_filter_active = !self.include_formats.is_empty();

        if !kind_filter_active && !format_filter_active {
            return true;
        }

        let matches_kind = if kind_filter_active {
            (self.include_text && record.kind == EntryKind::Text)
                || (self.include_image && record.kind == EntryKind::Image)
                || (self.include_file && record.kind == EntryKind::File)
                || (self.include_other && record.kind == EntryKind::Other)
                || (self.include_html && contains_format(&record.detected_formats, "html"))
        } else {
            false
        };

        let matches_format = if format_filter_active {
            self.include_formats
                .iter()
                .any(|f| contains_format(&record.detected_formats, f))
        } else {
            false
        };

        matches_kind || matches_format
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    #[default]
    Date,
    Copies,
    Type,
    Relevance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDirection {
    #[default]
    Desc,
    Asc,
}

#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub query: Option<String>,
    pub limit: Option<usize>,
    pub offset: usize,
    pub filter: SelectionFilter,
    pub from: Option<OffsetDateTime>,
    pub to: Option<OffsetDateTime>,
    pub sort: SortOrder,
    pub order: SortDirection,
    pub regex: bool,
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub hash: String,
    pub summary: Option<String>,
    pub kind: EntryKind,
    pub byte_size: u64,
    pub offset: usize,
    pub global_offset: usize,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub hits: Vec<SearchHit>,
    pub has_more: bool,
    pub total: usize,
}

pub fn search(index: &SearchIndex, options: &SearchOptions) -> SearchResult {
    let normalized_query = options.query.as_ref().and_then(|query| {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_lowercase())
        }
    });

    let from = options.from.as_ref();
    let to = options.to.as_ref();

    let mut all_records: Vec<_> = index.values().collect();
    // Sort by Date first to establish "stable indices"
    all_records.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));

    let mut indexed_records: Vec<(usize, &SearchIndexRecord)> =
        all_records.into_iter().enumerate().collect();

    match options.sort {
        SortOrder::Date => { /* already sorted in desc order */ }
        SortOrder::Copies => {
            indexed_records.sort_by(|(_, a), (_, b)| b.copy_count.cmp(&a.copy_count))
        }
        SortOrder::Type => indexed_records.sort_by(|(_, a), (_, b)| {
            let kind_a = format!("{:?}", a.kind);
            let kind_b = format!("{:?}", b.kind);
            kind_a.cmp(&kind_b)
        }),
        SortOrder::Relevance => {
            if let Some(query) = &normalized_query {
                indexed_records.sort_by(|(_, a), (_, b)| {
                    let score_a = calculate_relevance(a, query);
                    let score_b = calculate_relevance(b, query);
                    score_b.cmp(&score_a)
                });
            }
        }
    }

    // Apply sort direction (reverse if ascending)
    if options.order == SortDirection::Asc {
        indexed_records.reverse();
    }

    let records: Vec<_> = indexed_records
        .iter()
        .filter(|(_, record)| in_range(record, from, to))
        .filter(|(_, record)| options.filter.matches(record))
        .collect();

    let limit = options.limit.unwrap_or(usize::MAX);
    let mut hits = Vec::new();
    let mut total_matches = 0;
    let mut collected = 0;
    let mut has_more = false;

    for (global_position, record) in records {
        let record = *record;
        if let Some(query) = normalized_query.as_ref() {
            if !query_matches(record, query, options.regex) {
                continue;
            }
        }

        total_matches += 1;
        if total_matches <= options.offset {
            continue;
        }

        if collected >= limit {
            has_more = true;
            break;
        }

        collected += 1;
        hits.push(SearchHit {
            hash: record.hash.clone(),
            summary: record.summary.clone(),
            kind: record.kind.clone(),
            byte_size: record.byte_size,
            offset: total_matches - 1,
            global_offset: *global_position,
        });
    }

    SearchResult {
        hits,
        has_more,
        total: total_matches,
    }
}

fn in_range(
    record: &SearchIndexRecord,
    from: Option<&OffsetDateTime>,
    to: Option<&OffsetDateTime>,
) -> bool {
    match (from, to) {
        (Some(start), Some(end)) => record.last_seen >= *start && record.last_seen <= *end,
        (Some(start), None) => record.last_seen >= *start,
        (None, Some(end)) => record.last_seen <= *end,
        (None, None) => true,
    }
}

fn query_matches(record: &SearchIndexRecord, query: &str, is_regex: bool) -> bool {
    if is_regex {
        if let Ok(re) = regex::RegexBuilder::new(query)
            .case_insensitive(true)
            .build()
        {
            if re.is_match(&record.hash) {
                return true;
            }
            if record
                .summary
                .as_ref()
                .map(|summary| re.is_match(summary))
                .unwrap_or(false)
            {
                return true;
            }
            return record
                .search_text
                .as_ref()
                .map(|text| re.is_match(text))
                .unwrap_or(false);
        }
    }

    if record.hash.to_lowercase().contains(query) {
        return true;
    }

    if record
        .summary
        .as_ref()
        .map(|summary| summary.to_lowercase().contains(query))
        .unwrap_or(false)
    {
        return true;
    }

    record
        .search_text
        .as_ref()
        .map(|text| text.to_lowercase().contains(query))
        .unwrap_or(false)
}

fn contains_format(formats: &[String], needle: &str) -> bool {
    formats
        .iter()
        .any(|format| format.to_ascii_lowercase().contains(needle))
}

fn calculate_relevance(record: &SearchIndexRecord, query: &str) -> u32 {
    // Note: Regex relevance scoring is simplified to boolean match for now
    // as calculating "how much" it matches is complex and potentially slow.
    // We fall back to standard string matching for relevance if not regex,
    // or if regex we could try to see if it matches.
    // For now, let's keep the existing logic which assumes 'query' is a string literal.
    // If the user passed a regex, this might give low scores if the regex syntax
    // doesn't literally appear in the text, but that's acceptable for a first pass.

    let hash = record.hash.to_lowercase();
    let mut score = if hash == query {
        100
    } else if hash.contains(query) {
        80
    } else if let Some(summary) = &record.summary {
        let summary = summary.to_lowercase();
        if summary == query {
            90
        } else if summary.starts_with(query) {
            70
        } else if summary.contains(query) {
            60
        } else if let Some(text) = &record.search_text {
            if text.to_lowercase().contains(query) {
                40
            } else {
                0
            }
        } else {
            0
        }
    } else if let Some(text) = &record.search_text {
        if text.to_lowercase().contains(query) {
            40
        } else {
            0
        }
    } else {
        0
    };

    if score > 0 {
        let content_len = record
            .summary
            .as_ref()
            .map(|s| s.len())
            .or_else(|| record.search_text.as_ref().map(|t| t.len()))
            .unwrap_or(0) as f64;

        let length_boost = if content_len > 0.0 {
            (1000.0 / (content_len + 100.0)).max(0.5)
        } else {
            1.0
        };

        score = (score as f64 * length_boost).round() as u32;
    }

    score
}

pub fn parse_search_query(query: &str, force_regex: bool) -> (String, bool, SelectionFilter) {
    let mut filter = SelectionFilter::default();
    let mut final_query = query.to_string();
    let mut is_regex = force_regex;

    if query.starts_with("@") {
        match query {
            "@link" => {
                final_query = r"^https?://[^\s]+$".to_string();
                is_regex = true;
            }
            "@email" => {
                final_query = r"^[\w\-\.]+@([\w-]+\.)+[\w-]{2,4}$".to_string();
                is_regex = true;
            }
            "@image" => {
                filter.include_image = true;
                final_query = "".to_string();
            }
            "@file" => {
                filter.include_file = true;
                final_query = "".to_string();
            }
            "@html" => {
                filter.include_html = true;
                final_query = "".to_string();
            }
            "@color" => {
                // Matches hex, rgb, rgba, hsl, hsla
                final_query = r"(#[0-9a-fA-F]{3,6}|rgba?\([^)]+\)|hsla?\([^)]+\))".to_string();
                is_regex = true;
            }
            "@path" => {
                // Matches file paths (Unix/Windows)
                final_query = r"(/[^/ ]*)+/?|[a-zA-Z]:\\[^\\]*".to_string();
                is_regex = true;
                filter.include_file = true; // Also include file types
                filter.include_text = true; // And text that looks like paths
            }
            _ => {
                // Check for @[thing] syntax
                if query.starts_with("@[") && query.ends_with("]") {
                    // Treat as regex
                    final_query = query[2..query.len() - 1].to_string();
                    is_regex = true;
                }
            }
        }
    }

    (final_query, is_regex, filter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::model::{EntryKind, SearchIndexRecord};
    use time::OffsetDateTime;

    fn create_record(hash: &str, kind: EntryKind, formats: Vec<String>, summary: Option<String>) -> SearchIndexRecord {
        SearchIndexRecord {
            hash: hash.to_string(),
            last_seen: OffsetDateTime::now_utc(),
            kind,
            copy_count: 1,
            summary,
            search_text: None,
            detected_formats: formats,
            byte_size: 100,
            relative_path: "".to_string(),
        }
    }

    #[test]
    fn test_parse_search_query_link() {
        let (query, is_regex, _) = parse_search_query("@link", false);
        assert!(is_regex);
        let re = regex::RegexBuilder::new(&query).case_insensitive(true).build().unwrap();
        
        assert!(re.is_match("https://google.com"));
        assert!(re.is_match("http://example.com/foo"));
        assert!(!re.is_match("foo https://google.com bar"));
    }

    #[test]
    fn test_parse_search_query_email() {
        let (query, is_regex, _) = parse_search_query("@email", false);
        assert!(is_regex);
        let re = regex::RegexBuilder::new(&query).case_insensitive(true).build().unwrap();

        assert!(re.is_match("test@example.com"));
        assert!(re.is_match("foo.bar@baz.co.uk"));
        assert!(!re.is_match("not an email"));
        assert!(!re.is_match("contact: email@inside.text"));
    }

    #[test]
    fn test_search_html_filter() {
        let (_, _, filter) = parse_search_query("@html", false);
        assert!(filter.include_html);

        let record_html = create_record("1", EntryKind::Text, vec!["public.html".to_string()], None);
        let record_text = create_record("2", EntryKind::Text, vec!["public.utf8-plain-text".to_string()], None);

        assert!(filter.matches(&record_html));
        assert!(!filter.matches(&record_text));
    }
}
