use crate::data::SearchIndex;
use crate::data::store::{copy_by_selector, delete_entry, history_stream, load_index};
use crate::tui::state::AppState;
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
        None,
        None,
        None,
    )?
    .collect();
    state.items = items;
    if state.selected >= state.items.len() {
        state.selected = state.items.len().saturating_sub(1);
    }
    Ok(())
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
                    KeyCode::Char('q') => break,
                    KeyCode::Char('d') => {
                        if let Some(item) = state.items.get(state.selected) {
                            delete_entry(&item.metadata.hash)?;
                            rebuild_items(state, index)?;
                            state.set_status("Deleted item");
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(item) = state
                            .items
                            .get(state.selected)
                            .or_else(|| state.items.first())
                        {
                            copy_by_selector(index, &item.metadata.hash)?;
                            if modifiers.contains(KeyModifiers::SHIFT) {
                                state.set_status("Copied item");
                            } else {
                                break;
                            }
                        }
                    }
                    KeyCode::Char('h') => {
                        state.set_status(
                            "Enter: copy & exit, Shift+Enter copy & stay, d delete, q quit",
                        );
                    }
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        break;
                    }
                    KeyCode::Char(ch) => {
                        state.handle_char(ch);
                        rebuild_items(state, index)?;
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
        terminal.draw(|frame| draw_frame(frame, state))?;
    }
    Ok(())
}
