use crate::api;
use crate::cli::args::{
    ApiArgs, Cli, Command, DirCommand, EntryKind as CliEntryKind, FilterFlags, HistoryArgs,
    SearchArgs, ServiceAction,
};
use crate::clipboard::plugins::{self, DisplayContent, ImageDisplay};
use crate::config::{self, ensure_data_dir, load_config};
use crate::data::model::EntryMetadata;
use crate::data::store::{
    HistoryItem, SelectionFilter, copy_by_selector, delete_entry, human_size, load_history_items,
    load_index, load_metadata, refresh_index, resolve_selector, stream_history_items,
};
use crate::search::SearchOptions;
use crate::service::{self, ServiceStatus, watch};
use crate::tui;
use crate::util::time::{OffsetDateTime, format_iso, parse_date};
use anyhow::{Context, Result, bail};
use serde_json::to_string_pretty;
use std::{
    env,
    io::{self, ErrorKind, IsTerminal, Write},
    path::Path,
};
use viuer::Config as ViuerConfig;

#[derive(Debug, Clone, Copy)]
enum OutputMode {
    Text,
    JsonSimple,
    JsonFull,
}

pub fn dispatch(cli: Cli) -> Result<()> {
    let filters = cli.filters.clone();
    let json = cli.json;
    let command = cli
        .command
        .unwrap_or(Command::History(HistoryArgs::default()));
    match command {
        Command::Interactive { query } => {
            ensure_filters_unsupported(&cli.filters, cli.json, "interactive")?;
            run_interactive(query)
        }
        Command::Copy { selector } => copy_entry(&selector, &filters),
        Command::Delete { selector } => delete_item(&selector, &filters),
        Command::Show { selector } => {
            let mode = if json {
                OutputMode::JsonFull
            } else {
                OutputMode::Text
            };
            show_item(&selector, &filters, mode)
        }
        Command::Watch => watch::run_watch(None),
        Command::Service(args) => run_service(args.action),
        Command::Dir(args) => {
            ensure_filters_unsupported(&cli.filters, cli.json, "dir")?;
            run_dir(args.command)
        }
        Command::Search(args) => {
            let mode = if json {
                if args.full {
                    OutputMode::JsonFull
                } else {
                    OutputMode::JsonSimple
                }
            } else {
                OutputMode::Text
            };
            run_search(args, &filters, mode)
        }
        Command::Api(args) => run_api(args),
        Command::History(args) => {
            let mode = if json {
                if args.full {
                    OutputMode::JsonFull
                } else {
                    OutputMode::JsonSimple
                }
            } else {
                OutputMode::Text
            };
            print_history(args, &filters, mode)
        }
    }
}

fn ensure_filters_unsupported(filters: &FilterFlags, json: bool, command: &str) -> Result<()> {
    if json {
        bail!("--json is not supported for {command} command");
    }
    if !filters.is_empty() {
        bail!("Format filters are not supported for {command} command");
    }
    Ok(())
}

fn run_interactive(query: Option<String>) -> Result<()> {
    refresh_index()?;
    let index = load_index()?;
    tui::start(index, query)
}

fn copy_entry(selector: &str, filters: &FilterFlags) -> Result<()> {
    refresh_index()?;
    let index = load_index()?;
    let selection_filter = build_selection_filter(filters, None);
    let target = resolve_selector(&index, selector, &selection_filter)
        .with_context(|| format!("No clipboard item found for selector {selector}"))?;
    let metadata = copy_by_selector(&target)?;
    log_copy(&metadata);
    Ok(())
}

fn delete_item(selector: &str, filters: &FilterFlags) -> Result<()> {
    refresh_index()?;
    let index = load_index()?;
    let selection_filter = build_selection_filter(filters, None);
    let target = resolve_selector(&index, selector, &selection_filter)
        .with_context(|| format!("No clipboard item found for selector {selector}"))?;
    let metadata = load_metadata(&target)?;
    delete_entry(&target)?;
    let summary = metadata
        .summary
        .clone()
        .unwrap_or_else(|| target.chars().take(12).collect());
    println!("Deleted {}", summary);
    Ok(())
}

fn show_item(selector: &str, filters: &FilterFlags, mode: OutputMode) -> Result<()> {
    refresh_index()?;
    let index = load_index()?;
    let selection_filter = build_selection_filter(filters, None);
    let target = resolve_selector(&index, selector, &selection_filter)
        .with_context(|| format!("No clipboard item found for selector {selector}"))?;
    let metadata = load_metadata(&target)?;
    let selector_index = selector.parse::<usize>().ok();
    let config = load_config()?;
    let data_dir = ensure_data_dir(&config)?;
    let item_dir = data_dir.join(&metadata.relative_path);
    let preferred_plugin = preferred_display_plugin(filters);

    match mode {
        OutputMode::JsonFull => {
            let json_item = plugins::build_full_json_item(
                &metadata,
                &item_dir,
                selector_index,
            )?;
            let output = to_string_pretty(&json_item)?;
            if !write_line(&output)? {
                return Ok(());
            }
        }
        OutputMode::JsonSimple => {
            let json_item = plugins::build_json_item_with_preference(
                &metadata,
                &item_dir,
                selector_index.unwrap_or(0),
                preferred_plugin,
            )?;
            let output = to_string_pretty(&json_item)?;
            if !write_line(&output)? {
                return Ok(());
            }
        }
        OutputMode::Text => {
            let content =
                plugins::build_display_content_with_preference(&metadata, &item_dir, preferred_plugin)?;
            render_display(content)?;
            log_item_details(&metadata, &item_dir)?;
        }
    }

    Ok(())
}

fn run_service(action: ServiceAction) -> Result<()> {
    match action {
        ServiceAction::Install => {
            service::install_agent()?;
            if let Ok(status) = service::service_status() {
                print_service_status(&status);
            }
            Ok(())
        }
        ServiceAction::Uninstall => {
            service::uninstall_agent()?;
            if let Ok(status) = service::service_status() {
                print_service_status(&status);
            }
            Ok(())
        }
        ServiceAction::Start => service::start_agent(),
        ServiceAction::Stop => service::stop_agent(),
        ServiceAction::Status => {
            let status = service::service_status()?;
            print_service_status(&status);
            Ok(())
        }
        ServiceAction::Logs { lines, follow } => service::print_logs(lines, follow),
    }
}

fn run_api(args: ApiArgs) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Failed to initialize async runtime")?;
    runtime.block_on(api::serve(args.port))?;
    Ok(())
}

fn run_dir(command: DirCommand) -> Result<()> {
    let mut config = load_config().unwrap_or_default();
    match command {
        DirCommand::Get => {
            println!("{}", config.data_dir().display());
            Ok(())
        }
        DirCommand::Set { path } => {
            config::io::set_data_dir(path)?;
            Ok(())
        }
        DirCommand::Move { path } => {
            config::io::move_data_dir(path)?;
            config = load_config().unwrap_or_default();
            println!("Moved data directory to {}", config.data_dir().display());
            Ok(())
        }
    }
}

fn print_history(args: HistoryArgs, filters: &FilterFlags, mode: OutputMode) -> Result<()> {
    refresh_index()?;
    let index = load_index()?;
    let HistoryArgs {
        limit,
        query,
        kind,
        from: from_str,
        to: to_str,
        ..
    } = args;

    let from = from_str.map(|value| parse_date(&value)).transpose()?;
    let to = to_str.map(|value| parse_date(&value)).transpose()?;
    let selection_filter = build_selection_filter(filters, kind.clone());

    let mut options = SearchOptions::default();
    options.limit = Some(limit);
    options.query = query;
    options.filter = selection_filter;
    options.from = from;
    options.to = to;

    match mode {
        OutputMode::Text => {
            stream_history_items(&index, &options, |item| {
                output_single_item(item, mode)
            })
        }
        _ => {
            let (items, _) = load_history_items(&index, &options)?;
            output_history(&items, mode)
        }
    }
}

fn run_search(args: SearchArgs, filters: &FilterFlags, mode: OutputMode) -> Result<()> {
    refresh_index()?;
    let index = load_index()?;
    let SearchArgs { query, limit, .. } = args;
    let selection_filter = build_selection_filter(filters, None);

    let mut options = SearchOptions::default();
    options.limit = limit;
    options.query = Some(query);
    options.filter = selection_filter;

    match mode {
        OutputMode::Text => {
            stream_history_items(&index, &options, |item| {
                output_single_item(item, mode)
            })
        }
        _ => {
            let (items, _) = load_history_items(&index, &options)?;
            output_history(&items, mode)
        }
    }
}

fn output_single_item(item: &HistoryItem, mode: OutputMode) -> Result<bool> {
    match mode {
        OutputMode::Text => {
            let is_interactive = io::stdout().is_terminal();
            let terminal_width = if is_interactive {
                crossterm::terminal::size()
                    .map(|(width, _)| width as usize)
                    .unwrap_or(80)
            } else {
                usize::MAX
            };

            let timestamp = format_history_timestamp(item.metadata.last_seen);
            let copies = item.metadata.copy_count;
            let summary = if is_interactive {
                clip_summary_to_width(
                    &item.summary,
                    terminal_width,
                    item.offset,
                    &timestamp,
                    copies,
                )
            } else {
                clean_summary(&item.summary)
            };
            let line = format!("{} [{} x{}]   {}", item.offset, timestamp, copies, summary);
            write_line(&line)
        }
        _ => Ok(true),
    }
}

fn output_history(items: &[HistoryItem], mode: OutputMode) -> Result<()> {
    match mode {
        OutputMode::JsonFull => {
            let config = load_config()?;
            let data_dir = ensure_data_dir(&config)?;
            let mut json_items = Vec::new();
            for item in items {
                let item_dir = data_dir.join(&item.metadata.relative_path);
                json_items.push(plugins::build_full_json_item(
                    &item.metadata,
                    &item_dir,
                    Some(item.offset),
                )?);
            }
            let output = to_string_pretty(&json_items)?;
            if !write_line(&output)? {
                return Ok(());
            }
            return Ok(());
        }
        OutputMode::JsonSimple => {
            let config = load_config()?;
            let data_dir = ensure_data_dir(&config)?;
            let mut json_items = Vec::new();
            for item in items {
                let item_dir = data_dir.join(&item.metadata.relative_path);
                json_items.push(plugins::build_json_item(
                    &item.metadata,
                    &item_dir,
                    item.offset,
                )?);
            }
            let output = to_string_pretty(&json_items)?;
            if !write_line(&output)? {
                return Ok(());
            }
            return Ok(());
        }
        OutputMode::Text => {
            let is_interactive = io::stdout().is_terminal();
            let terminal_width = if is_interactive {
                crossterm::terminal::size()
                    .map(|(width, _)| width as usize)
                    .unwrap_or(80)
            } else {
                usize::MAX
            };

            for item in items {
                let timestamp = format_history_timestamp(item.metadata.last_seen);
                let copies = item.metadata.copy_count;
                let summary = if is_interactive {
                    clip_summary_to_width(
                        &item.summary,
                        terminal_width,
                        item.offset,
                        &timestamp,
                        copies,
                    )
                } else {
                    clean_summary(&item.summary)
                };
                let line = format!("{} [{} x{}]   {}", item.offset, timestamp, copies, summary);
                if !write_line(&line)? {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn render_display(content: DisplayContent) -> Result<()> {
    match content {
        DisplayContent::Text(text) => {
            if !write_text_block(&text)? {
                return Ok(());
            }
            Ok(())
        }
        DisplayContent::Lines(lines) => {
            for line in lines {
                if !write_line(&line)? {
                    break;
                }
            }
            Ok(())
        }
        DisplayContent::Image(image) => render_image(&image),
        DisplayContent::Empty => Ok(()),
    }
}

fn render_image(image: &ImageDisplay) -> Result<()> {
    if terminal_supports_images() {
        let mut config = ViuerConfig::default();
        config.restore_cursor = false;
        if viuer::print_from_file(&image.path, &config).is_ok() {
            return Ok(());
        }
    }
    if let Some(fallback) = &image.fallback {
        if !write_line(fallback)? {
            return Ok(());
        }
    } else {
        let path_text = format!("{}", image.path.display());
        if !write_line(&path_text)? {
            return Ok(());
        }
    }
    Ok(())
}

fn terminal_supports_images() -> bool {
    env::var("ITERM_SESSION_ID").is_ok()
        || env::var("TERM_PROGRAM")
            .map(|value| {
                value.eq_ignore_ascii_case("WezTerm") || value.eq_ignore_ascii_case("iTerm.app")
            })
            .unwrap_or(false)
        || env::var("TERM")
            .map(|term| term.contains("kitty") || term.contains("wezterm"))
            .unwrap_or(false)
        || env::var("WEZTERM_PANE").is_ok()
}

fn build_selection_filter(filters: &FilterFlags, kind: Option<CliEntryKind>) -> SelectionFilter {
    let mut selection = SelectionFilter::default();
    if filters.text || matches!(kind, Some(CliEntryKind::Text)) {
        selection.include_text = true;
    }
    if filters.image || matches!(kind, Some(CliEntryKind::Image)) {
        selection.include_image = true;
    }
    if filters.file || matches!(kind, Some(CliEntryKind::File)) {
        selection.include_file = true;
    }
    if matches!(kind, Some(CliEntryKind::Other)) {
        selection.include_other = true;
    }
    selection.require_html = filters.html;
    selection.require_rtf = filters.rtf;
    selection
}

fn preferred_display_plugin(filters: &FilterFlags) -> Option<&'static str> {
    if filters.file {
        return Some("files");
    }
    if filters.html {
        return Some("html");
    }
    if filters.rtf {
        return Some("rtf");
    }
    if filters.text {
        return Some("text");
    }
    if filters.image {
        return Some("image");
    }
    None
}

fn print_service_status(status: &ServiceStatus) {
    println!("Service installed: {}", bool_word(status.installed));
    println!("Service running: {}", bool_word(status.running));
    if !status.details.is_empty() {
        for (key, value) in &status.details {
            println!("{}: {}", key, value);
        }
    }
}

fn bool_word(flag: bool) -> &'static str {
    if flag { "yes" } else { "no" }
}

fn log_copy(metadata: &EntryMetadata) {
    let summary = metadata
        .summary
        .clone()
        .unwrap_or_else(|| metadata.hash.chars().take(12).collect());
    let snippet = clean_summary(&summary);
    eprintln!("Copied: {}", snippet);
}

fn log_item_details(metadata: &EntryMetadata, item_dir: &Path) -> Result<()> {
    let mut details = Vec::new();
    details.push(("date".to_string(), format_iso(metadata.last_seen)));
    details.push(("copies".to_string(), metadata.copy_count.to_string()));
    details.push(("hash".to_string(), metadata.hash.clone()));
    details.push(("size".to_string(), human_size(metadata.byte_size)));
    details.push(("path".to_string(), item_dir.to_string_lossy().to_string()));
    details.push(("kind".to_string(), format!("{:?}", metadata.kind)));
    if let Some(summary) = &metadata.summary {
        details.push(("summary".to_string(), clean_summary(summary)));
    }
    for (key, value) in plugins::build_detail_log(metadata, item_dir)? {
        details.push((key, value));
    }
    let message = details
        .into_iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join(" ");
    eprintln!("\u{001b}[2m[details] {}\u{001b}[0m", message);
    Ok(())
}

fn format_history_timestamp(dt: OffsetDateTime) -> String {
    use time::macros::format_description;
    let format = format_description!("[month]/[day]@[hour]:[minute]");
    dt.format(&format).unwrap_or_else(|_| dt.to_string())
}

fn clean_summary(input: &str) -> String {
    let clean = input.replace('\n', " ").replace('\r', " ");
    let trimmed = clean.trim();
    if trimmed.is_empty() {
        "(empty)".into()
    } else {
        trimmed.to_string()
    }
}

fn clip_summary_to_width(
    input: &str,
    terminal_width: usize,
    index: usize,
    timestamp: &str,
    copies: u64,
) -> String {
    let clean = clean_summary(input);
    let prefix = format!("{} [{} x{}]   ", index, timestamp, copies);
    let prefix_len = prefix.chars().count();

    if terminal_width <= prefix_len {
        return String::new();
    }

    let available_width = terminal_width - prefix_len;
    if available_width == 0 {
        return String::new();
    }

    let char_count = clean.chars().count();
    if char_count <= available_width {
        return clean;
    }

    if available_width <= 3 {
        return clean.chars().take(available_width).collect();
    }

    let truncated: String = clean.chars().take(available_width - 3).collect();
    format!("{}...", truncated)
}

fn write_line(line: &str) -> Result<bool> {
    let mut stdout = io::stdout();
    match writeln!(stdout, "{}", line) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == ErrorKind::BrokenPipe => Ok(false),
        Err(err) => Err(err.into()),
    }
}

fn write_text_block(text: &str) -> Result<bool> {
    let mut stdout = io::stdout();
    match writeln!(stdout, "{}", text) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == ErrorKind::BrokenPipe => Ok(false),
        Err(err) => Err(err.into()),
    }
}
