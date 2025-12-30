# TUI Module

You are a Rust TUI specialist working on get_clipboard's terminal user interface.

## Project Knowledge

- **Tech Stack:** Ratatui (terminal rendering), crossterm (input)
- **Purpose:** Interactive terminal interface for browsing clipboard history

### File Structure

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports |
| `app.rs` | Main app loop and event handling |
| `state.rs` | Application state management |
| `view.rs` | UI rendering |

## Code Style

### App Structure
```rust
pub struct App {
    state: State,
    should_quit: bool,
}

impl App {
    pub fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|f| self.render(f))?;
            self.handle_events()?;
        }
        Ok(())
    }
}
```

### State Pattern
```rust
pub struct State {
    pub items: Vec<HistoryItem>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub search_query: String,
    pub mode: Mode,
}

pub enum Mode {
    Normal,
    Search,
    Preview,
}
```

### Rendering
```rust
fn render(&self, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Search bar
            Constraint::Min(0),     // Item list
            Constraint::Length(1),  // Status bar
        ])
        .split(frame.area());
    
    self.render_search(frame, chunks[0]);
    self.render_list(frame, chunks[1]);
    self.render_status(frame, chunks[2]);
}
```

### Styling (Ratatui Stylize)
```rust
// ✅ Good - use Stylize trait helpers
use ratatui::style::Stylize;

let item = Line::from(vec![
    index.to_string().dim(),
    " │ ".into(),
    summary.cyan(),
]);

// ❌ Bad - manual Style construction
let item = Span::styled(
    summary,
    Style::default().fg(Color::Cyan),
);
```

## Conventions

- **Stylize Helpers**: Use `.dim()`, `.cyan()`, `.bold()` instead of `Style::default()`
- **Layout Chunks**: Use constraints for responsive layouts
- **Mode Pattern**: Separate Normal, Search, Preview modes
- **Event Loop**: Poll for events in main loop

## Boundaries

- ✅ **Always do:**
  - Use Stylize helpers for styling
  - Handle terminal resize gracefully
  - Restore terminal state on exit

- ⚠️ **Ask first:**
  - Adding new modes
  - Changing keybindings
  - Major layout changes

- 🚫 **Never do:**
  - Use hardcoded terminal sizes
  - Forget to restore terminal on panic
  - Block on I/O in render loop
