use crate::tui::state::AppState;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

pub fn draw_frame(frame: &mut Frame<'_>, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(frame.size());

    let title = format!("get_clipboard — {} items", state.items.len());
    let header = Paragraph::new(title)
        .block(Block::default().borders(Borders::BOTTOM))
        .style(Style::default().fg(Color::White));
    frame.render_widget(header, chunks[0]);

    let query_display = if state.filter.is_empty() {
        String::from("Type to search clipboard history…")
    } else {
        format!("› {}", state.filter)
    };
    let query = Paragraph::new(query_display)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Gray))
                .title("Search"),
        )
        .style(Style::default().fg(Color::White));
    frame.render_widget(query, chunks[1]);

    let items: Vec<_> = state
        .items
        .iter()
        .map(|item| {
            let mut spans = Vec::new();
            let offset = item.offset;
            spans.push(Span::styled(
                format!("#{:<4}", offset),
                Style::default().fg(Color::DarkGray),
            ));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                item.summary.clone(),
                Style::default().fg(Color::White),
            ));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format!("{}", item.kind),
                Style::default().fg(Color::Blue),
            ));
            ListItem::new(Line::from(spans))
        })
        .collect();

    let mut list_state = list_state(state.selected);
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");
    frame.render_stateful_widget(list, chunks[2], &mut list_state);

    let status_text = state
        .status
        .clone()
        .unwrap_or_else(|| "Enter to copy, d delete, h help, q quit".into());
    let footer = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::TOP))
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(footer, chunks[3]);
}

fn list_state(selected: usize) -> ListState {
    let mut state = ListState::default();
    state.select(Some(selected));
    state
}
