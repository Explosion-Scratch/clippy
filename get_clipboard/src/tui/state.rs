use crate::data::store::{HistoryItem, ItemPreview};
use crossterm::event::KeyCode;

pub struct AppState {
    pub items: Vec<HistoryItem>,
    pub selected: usize,
    pub query: String,
    pub status: Option<String>,
    pub filter: String,
    pub sticky_query: Option<String>,
    pub preview: Option<PreviewState>,
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
    }

    pub fn backspace(&mut self) {
        if !self.filter.is_empty() {
            self.filter.pop();
            self.query = self.filter.clone();
            self.selected = 0;
            self.invalidate_preview();
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

    pub fn set_items(&mut self, items: Vec<HistoryItem>) {
        self.items = items;
        if self.selected >= self.items.len() {
            self.selected = self.items.len().saturating_sub(1);
        }
        self.invalidate_preview();
    }

    pub fn invalidate_preview(&mut self) {
        self.preview = None;
    }

    pub fn selected_item(&self) -> Option<&HistoryItem> {
        self.items.get(self.selected)
    }
}
