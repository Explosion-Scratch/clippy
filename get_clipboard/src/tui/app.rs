use crate::data::SearchIndex;
use crate::data::store::{
    SelectionFilter, copy_by_selector, delete_entry, history_stream, load_index, load_item_preview,
    preview_snippet,
};
use crate::tui::state::{AppState, PreviewState};
use crate::tui::view::draw_frame;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{ExecutableCommand, execute};
use std::io::{Stdout, stdout};
use std::time::Duration;

pub fn start(mut index: SearchIndex, query: Option<String>) -> Result<()> {
    let mut stdout = stdout();
    let mut terminal = setup_terminal(&mut stdout)?;
    let mut state = AppState::new(Vec::new());
    if let Some(q) = query {
        state.filter = q.clone();
        state.sticky_query = Some(q);
    }
    rebuild_items(&mut state, &mut index)?;
    ensure_preview(&mut state)?;
    terminal.draw(|frame| draw_frame(frame, &state))?;
    event_loop(&mut terminal, &mut state, &mut index)?;
    drop(terminal);
    teardown_terminal(&mut stdout)
}

fn setup_terminal(
    stdout: &mut Stdout,
) -> Result<ratatui::Terminal<ratatui::backend::CrosstermBackend<&mut Stdout>>> {
    execute!(stdout, EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;
    Ok(ratatui::Terminal::new(
        ratatui::backend::CrosstermBackend::new(stdout),
    )?)
}

fn teardown_terminal(stdout: &mut Stdout) -> Result<()> {
    terminal::disable_raw_mode()?;
    stdout.execute(LeaveAlternateScreen)?;
    Ok(())
}

fn rebuild_items(state: &mut AppState, index: &mut SearchIndex) -> Result<()> {
    *index = load_index()?;
    let items: Vec<_> = history_stream(
        index,
        Some(200),
        if state.filter.is_empty() {
            None
        } else {
            Some(state.filter.clone())
        },
        &SelectionFilter::default(),
        None,
        None,
    )?
    .collect();
    state.set_items(items);
    Ok(())
}

fn ensure_preview(state: &mut AppState) -> Result<()> {
    if let Some(item) = state.selected_item() {
        let needs_refresh = match state.preview.as_ref() {
            Some(existing) => existing.hash != item.metadata.hash,
            None => true,
        };
        if needs_refresh {
            let preview = load_item_preview(&item.metadata)?;
            state.preview = Some(PreviewState {
                hash: item.metadata.hash.clone(),
                content: preview,
            });
        }
    } else {
        state.preview = None;
    }
    Ok(())
}

fn preview_text_for_state(
    state: &AppState,
    metadata: &crate::data::model::EntryMetadata,
) -> String {
    if let Some(preview) = state.preview.as_ref() {
        preview_snippet(&preview.content, metadata)
    } else {
        metadata
            .summary
            .clone()
            .unwrap_or_else(|| format!("{:?}", metadata.kind))
    }
}

fn copy_status(snippet: &str) -> String {
    let clean = snippet.trim().replace('\n', " ").replace('\r', " ");
    let mut status = format!("Copied {}", clean);
    if status.len() > 70 {
        status.truncate(67);
        status.push_str("...");
    }
    status
}

fn event_loop(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<&mut Stdout>>,
    state: &mut AppState,
    index: &mut SearchIndex,
) -> Result<()> {
    loop {
        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(KeyEvent {
                    code, modifiers, ..
                }) => match code {
                    KeyCode::Enter => {
                        ensure_preview(state)?;
                        if let Some(item) = state.selected_item().or_else(|| state.items.first()) {
                            copy_by_selector(&item.metadata.hash)?;
                            let snippet = preview_text_for_state(state, &item.metadata);
                            let clean_snippet = snippet.replace('\n', " ").replace('\r', " ");
                            eprintln!("Copied: {}", clean_snippet);
                            state.set_status(copy_status(&clean_snippet));
                            if !modifiers.contains(KeyModifiers::SHIFT) {
                                break;
                            }
                        }
                    }
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        break;
                    }
                    KeyCode::Backspace if modifiers.contains(KeyModifiers::ALT) => {
                        if let Some(item) = state.selected_item() {
                            delete_entry(&item.metadata.hash)?;
                            rebuild_items(state, index)?;
                            state.set_status("Deleted item");
                        }
                    }
                    KeyCode::Delete if modifiers.contains(KeyModifiers::ALT) => {
                        if let Some(item) = state.selected_item() {
                            delete_entry(&item.metadata.hash)?;
                            rebuild_items(state, index)?;
                            state.set_status("Deleted item");
                        }
                    }
                    KeyCode::Char(ch) => {
                        if !modifiers.contains(KeyModifiers::CONTROL) {
                            state.handle_char(ch);
                            rebuild_items(state, index)?;
                        }
                    }
                    KeyCode::Backspace => {
                        state.backspace();
                        rebuild_items(state, index)?;
                    }
                    KeyCode::Esc => {
                        if let Some(original) = &state.sticky_query {
                            state.filter = original.clone();
                        } else {
                            state.filter.clear();
                        }
                        rebuild_items(state, index)?;
                    }
                    other => {
                        state.handle_key(other);
                    }
                },
                _ => {}
            }
        }
        ensure_preview(state)?;
        terminal.draw(|frame| draw_frame(frame, state))?;
    }
    Ok(())
}
