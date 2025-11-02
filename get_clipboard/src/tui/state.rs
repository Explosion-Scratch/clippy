use crate::data::store::{HistoryItem, ItemPreview};
use crossterm::event::KeyCode;
use std::time::Instant;

pub struct AppState {
    pub items: Vec<HistoryItem>,
    pub selected: usize,
    pub query: String,
    pub status: Option<String>,
    pub filter: String,
    pub sticky_query: Option<String>,
    pub preview: Option<PreviewState>,
    pub has_more: bool,
    pub loading: bool,
    pub pending_reload: bool,
    pub last_filter_change: Option<Instant>,
}

pub struct PreviewState {
    pub hash: String,
    pub content: ItemPreview,
}

impl AppState {
    pub fn new(items: Vec<HistoryItem>) -> Self {
        AppState {
            items,
            selected: 0,
            query: String::new(),
            status: None,
            filter: String::new(),
            sticky_query: None,
            preview: None,
            has_more: false,
            loading: false,
            pending_reload: false,
            last_filter_change: None,
        }
    }

    pub fn handle_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Down => self.next(),
            KeyCode::Up => self.previous(),
            _ => {}
        }
    }

    pub fn handle_char(&mut self, ch: char) {
        self.filter.push(ch);
        self.query = self.filter.clone();
        self.selected = 0;
        self.invalidate_preview();
        self.mark_filter_dirty();
    }

    pub fn backspace(&mut self) {
        if !self.filter.is_empty() {
            self.filter.pop();
            self.query = self.filter.clone();
            self.selected = 0;
            self.invalidate_preview();
            self.mark_filter_dirty();
        }
    }

    pub fn next(&mut self) {
        if self.selected + 1 < self.items.len() {
            self.selected += 1;
            self.invalidate_preview();
        }
    }

    pub fn previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.invalidate_preview();
        }
    }

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status = Some(message.into());
    }

    pub fn set_items(&mut self, items: Vec<HistoryItem>, has_more: bool) {
        self.items = items;
        if self.selected >= self.items.len() {
            self.selected = self.items.len().saturating_sub(1);
        }
        self.invalidate_preview();
        self.has_more = has_more;
        self.loading = false;
        self.pending_reload = false;
        self.last_filter_change = None;
        self.query = self.filter.clone();
    }

    pub fn append_items(&mut self, items: Vec<HistoryItem>, has_more: bool) {
        let previous_len = self.items.len();
        for item in items {
            let exists = self
                .items
                .iter()
                .any(|existing| existing.metadata.hash == item.metadata.hash);
            if !exists {
                self.items.push(item);
            }
        }
        if self.items.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.items.len() {
            self.selected = self.items.len().saturating_sub(1);
            self.invalidate_preview();
        } else if previous_len == 0 {
            self.selected = 0;
            self.invalidate_preview();
        }
        self.has_more = has_more;
        self.loading = false;
    }

    pub fn invalidate_preview(&mut self) {
        self.preview = None;
    }

    pub fn selected_item(&self) -> Option<&HistoryItem> {
        self.items.get(self.selected)
    }

    pub fn mark_filter_dirty(&mut self) {
        self.pending_reload = true;
        self.last_filter_change = Some(Instant::now());
    }

    pub fn should_reload(&self, debounce: std::time::Duration) -> bool {
        if !self.pending_reload {
            return false;
        }
        match self.last_filter_change {
            Some(timestamp) => timestamp.elapsed() >= debounce,
            None => true,
        }
    }
}
