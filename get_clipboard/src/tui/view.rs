use crate::data::store::{
    human_size, narrowest_folder, preview_snippet, resolved_file_paths, saved_format_labels,
};
use crate::tui::state::AppState;
use crate::util::time::format_human;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};

pub fn draw_frame(frame: &mut Frame<'_>, state: &AppState) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(frame.size());

    let title = format!(
        "get_clipboard v{} — {} items",
        env!("CARGO_PKG_VERSION"),
        state.items.len()
    );
    let header = Paragraph::new(title)
        .block(Block::default().borders(Borders::BOTTOM))
        .style(Style::default().fg(Color::White));
    frame.render_widget(header, layout[0]);

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
    frame.render_widget(query, layout[1]);

    let show_preview = frame.size().width > 100;
    let main_areas: Vec<Rect> = if show_preview {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(layout[2])
            .to_vec()
    } else {
        vec![layout[2]]
    };

    render_list(frame, state, main_areas[0]);

    if show_preview {
        if let Some(area) = main_areas.get(1) {
            render_preview(frame, state, *area);
        }
    }

    let status_text = state.status.clone().unwrap_or_else(default_status);
    let footer = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::TOP))
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(footer, layout[3]);
}

fn list_state(selected: usize) -> ListState {
    let mut state = ListState::default();
    state.select(Some(selected));
    state
}

fn render_list(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let list_width = area.width as usize;
    let mut items = Vec::new();
    for item in &state.items {
        let offset_text = format!("#{:<4}", item.offset);
        let type_text = item.kind.clone();
        let base_width = offset_text.len() + type_text.len() + 4;
        let available = list_width.saturating_sub(base_width + 2);
        let summary = truncate_display(&item.summary, available);
        let spans = vec![
            Span::styled(offset_text, Style::default().fg(Color::DarkGray)),
            Span::raw(" "),
            Span::styled(summary, Style::default().fg(Color::White)),
            Span::raw(" "),
            Span::styled(type_text, Style::default().fg(Color::Blue)),
        ];
        items.push(ListItem::new(Line::from(spans)));
    }

    let mut list_state = list_state(state.selected);
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_preview(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Preview");
    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);
    frame.render_widget(Clear, inner);

    let Some(selected) = state.selected_item() else {
        let placeholder = Paragraph::new("Select an item to preview");
        frame.render_widget(placeholder, inner);
        return;
    };

    let Some(preview_state) = state.preview.as_ref() else {
        let placeholder = Paragraph::new("Loading preview…");
        frame.render_widget(placeholder, inner);
        return;
    };

    let text_content = match &preview_state.content.text {
        Some(text) if !text.is_empty() => text.clone(),
        _ => preview_snippet(&preview_state.content, &selected.metadata),
    };

    let preview_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(6)])
        .split(inner);

    let preview_style = Style::default()
        .fg(Color::White)
        .bg(Color::Rgb(24, 24, 24))
        .add_modifier(Modifier::BOLD);
    let text_widget = Paragraph::new(text_content)
        .wrap(Wrap { trim: false })
        .style(preview_style);
    frame.render_widget(text_widget, preview_layout[0]);

    let info_lines = build_info_lines(&selected.metadata, &preview_state.content);
    let metadata_style = Style::default().fg(Color::Gray).add_modifier(Modifier::DIM);
    let info_widget = Paragraph::new(info_lines).style(metadata_style);
    frame.render_widget(info_widget, preview_layout[1]);
}

fn build_info_lines(
    metadata: &crate::data::model::EntryMetadata,
    preview: &crate::data::store::ItemPreview,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if let Some(summary) = file_summary_line(metadata, preview) {
        lines.push(Line::from(summary));
    }

    let label_style = Style::default().fg(Color::White);
    let mut info_pairs = Vec::new();
    info_pairs.push(("Type", format!("{:?}", metadata.kind)));
    info_pairs.push(("Copies", metadata.copy_count.to_string()));
    info_pairs.push(("First", format_human(metadata.first_seen)));
    if metadata.first_seen != metadata.last_seen {
        info_pairs.push(("Last", format_human(metadata.last_seen)));
    }
    if let Some((w, h)) = preview.dimensions {
        info_pairs.push(("Dimensions", format!("{} x {}", w, h)));
    }
    let format_labels = saved_format_labels(metadata);
    if !format_labels.is_empty() {
        info_pairs.push(("Formats", format_labels.join(", ")));
    }
    info_pairs.push(("Version", metadata.version.clone()));

    for (label, value) in info_pairs {
        let spans = vec![
            Span::styled(format!("{} ", label), label_style),
            Span::raw(value),
        ];
        lines.push(Line::from(spans));
    }

    lines
}

fn file_summary_line(
    metadata: &crate::data::model::EntryMetadata,
    preview: &crate::data::store::ItemPreview,
) -> Option<String> {
    let resolved_paths = resolved_file_paths(metadata);
    let mut count = resolved_paths.len();
    if count == 0 {
        count = metadata.sources.len();
    }
    if count == 0 {
        count = preview.files.len();
    }
    if count <= 1 {
        return None;
    }
    let folder = if !resolved_paths.is_empty() {
        narrowest_folder(resolved_paths.as_slice()).unwrap_or_else(|| String::from("(unknown)"))
    } else if !metadata.sources.is_empty() {
        narrowest_folder(metadata.sources.as_slice()).unwrap_or_else(|| String::from("(unknown)"))
    } else {
        String::from("(unknown)")
    };
    Some(format!(
        "[{} {} in {} - total {}]",
        count,
        if count == 1 { "file" } else { "files" },
        folder,
        human_size(metadata.byte_size)
    ))
}

fn truncate_display(input: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }
    let mut text = input.replace('\n', " ").replace('\r', " ");
    if text.len() > max_len {
        if max_len > 3 {
            text.truncate(max_len - 3);
            text.push_str("...");
        } else {
            text.truncate(max_len);
        }
    }
    text
}

fn default_status() -> String {
    String::from("Enter copy • Shift+Enter copy+stay • Alt+Delete delete • Ctrl+C exit")
}
