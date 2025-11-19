use crate::data::model::{EntryKind, SearchIndex, SearchIndexRecord};
use crate::util::time::OffsetDateTime;

#[derive(Debug, Clone, Default)]
pub struct SelectionFilter {
    pub include_text: bool,
    pub include_image: bool,
    pub include_file: bool,
    pub include_other: bool,
    pub include_formats: Vec<String>,
}

impl SelectionFilter {
    pub fn matches(&self, record: &SearchIndexRecord) -> bool {
        let kind_filter_active = self.include_text
            || self.include_image
            || self.include_file
            || self.include_other;
        let format_filter_active = !self.include_formats.is_empty();

        if !kind_filter_active && !format_filter_active {
            return true;
        }

        let matches_kind = if kind_filter_active {
            (self.include_text && record.kind == EntryKind::Text)
                || (self.include_image && record.kind == EntryKind::Image)
                || (self.include_file && record.kind == EntryKind::File)
                || (self.include_other && record.kind == EntryKind::Other)
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

#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub query: Option<String>,
    pub limit: Option<usize>,
    pub offset: usize,
    pub filter: SelectionFilter,
    pub from: Option<OffsetDateTime>,
    pub to: Option<OffsetDateTime>,
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
    all_records.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));

    let mut records: Vec<_> = all_records
        .iter()
        .enumerate()
        .filter(|(_, record)| in_range(record, from, to))
        .filter(|(_, record)| options.filter.matches(record))
        .collect();

    let limit = options.limit.unwrap_or(usize::MAX);
    let mut hits = Vec::new();
    let mut total_matches = 0;
    let mut collected = 0;
    let mut has_more = false;

    for (global_position, record) in records {
        if let Some(query) = normalized_query.as_ref() {
            if !query_matches(record, query) {
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
            global_offset: global_position,
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

fn query_matches(record: &SearchIndexRecord, query: &str) -> bool {
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
