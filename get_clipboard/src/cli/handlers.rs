use crate::cli::args::{
    Cli, Command, DirCommand, EntryKind as CliEntryKind, FilterFlags, HistoryArgs, ServiceAction,
};
use crate::config::{AppConfig, load_config, save_config};
use crate::data::model::{EntryKind, EntryMetadata};
use crate::data::store::{
    ItemPreview, SelectionFilter, copy_by_selector, delete_entry, history_stream, human_size,
    load_index, load_item_preview, load_metadata, preview_snippet, refresh_index, resolve_selector,
};
use crate::service::{launchd, watch};
use crate::tui;
use crate::util::time::{format_human, parse_date};
use anyhow::{Context, Result, bail};
use std::{env, fs, path::PathBuf};
use viuer::Config as ViuerConfig;

pub fn dispatch(cli: Cli) -> Result<()> {
    let filters = cli.filters.clone();
    let command = cli
        .command
        .unwrap_or(Command::History(HistoryArgs::default()));
    match command {
        Command::Interactive { query } => run_interactive(query),
        Command::Copy { selector } => copy_entry(&selector, &filters),
        Command::Delete { selector } => delete_item(&selector, &filters),
        Command::Show { selector } => show_item(&selector, &filters),
        Command::Watch => watch::run_watch(None),
        Command::Service(args) => run_service(args.action),
        Command::Dir(args) => run_dir(args.command),
        Command::History(args) => print_history(args, &filters),
    }
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
    let preview = load_item_preview(&metadata)?;
    log_copy(&metadata, &preview);
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

fn show_item(selector: &str, filters: &FilterFlags) -> Result<()> {
    refresh_index()?;
    let index = load_index()?;
    let selection_filter = build_selection_filter(filters, None);
    let target = resolve_selector(&index, selector, &selection_filter)
        .with_context(|| format!("No clipboard item found for selector {selector}"))?;
    let metadata = load_metadata(&target)?;
    let preview = load_item_preview(&metadata)?;
    display_item(&metadata, &preview)
}

fn run_service(action: ServiceAction) -> Result<()> {
    match action {
        ServiceAction::Install => launchd::install_agent(),
        ServiceAction::Uninstall => launchd::uninstall_agent(),
        ServiceAction::Start => launchd::start_agent(),
        ServiceAction::Stop => launchd::stop_agent(),
        ServiceAction::Logs { lines, follow } => launchd::print_logs(lines, follow),
    }
}

fn run_dir(command: DirCommand) -> Result<()> {
    let mut config = load_config().unwrap_or_default();
    match command {
        DirCommand::Get => {
            println!("{}", config.data_dir().display());
            Ok(())
        }
        DirCommand::Set { path } => {
            config.override_data_dir = Some(normalize(path));
            save_config(&config)
        }
        DirCommand::Move { path } => move_dir(&mut config, normalize(path)),
    }
}

fn move_dir(config: &mut AppConfig, target: PathBuf) -> Result<()> {
    let current = config.data_dir();
    if current == target {
        bail!("Directory already set to {}", current.display());
    }
    fs::create_dir_all(&target)
        .with_context(|| format!("Failed to create target directory {}", target.display()))?;
    for entry in fs::read_dir(&current)? {
        let entry = entry?;
        let source = entry.path();
        let dest = target.join(entry.file_name());
        fs::rename(source, dest)?;
    }
    config.override_data_dir = Some(target.clone());
    save_config(config)?;
    println!("Moved data directory to {}", target.display());
    Ok(())
}

fn print_history(args: HistoryArgs, filters: &FilterFlags) -> Result<()> {
    refresh_index()?;
    let index = load_index()?;
    let from = args.from.map(|d| parse_date(&d)).transpose()?;
    let to = args.to.map(|d| parse_date(&d)).transpose()?;
    let selection_filter = build_selection_filter(filters, args.kind.clone());
    let items = history_stream(&index, args.limit, args.query, &selection_filter, from, to)?;

    for item in items {
        let short_hash = item.metadata.hash.chars().take(12).collect::<String>();
        let date_str = format_human(item.metadata.last_seen);
        let type_str = format!("{:?}", item.metadata.kind);
        let summary = item.summary.replace('\n', " ").replace('\r', " ");
        let size_str = human_size(item.metadata.byte_size);
        println!(
            "{}	{}	{}	{}	{}	{}",
            item.offset, short_hash, date_str, type_str, size_str, summary
        );
    }
    Ok(())
}

fn display_item(metadata: &EntryMetadata, preview: &ItemPreview) -> Result<()> {
    match metadata.kind {
        EntryKind::Text => {
            if let Some(text) = &preview.text {
                println!("{}", text);
            } else if let Some(path) = &preview.content_path {
                println!("{}", path.display());
            } else if let Some(summary) = &metadata.summary {
                println!("{}", summary);
            }
            Ok(())
        }
        EntryKind::Image => show_image(preview, metadata),
        EntryKind::File => {
            print_file_paths(metadata, preview);
            Ok(())
        }
        EntryKind::Other => {
            if let Some(path) = &preview.content_path {
                println!("{}", path.display());
            }
            Ok(())
        }
    }
}

fn show_image(preview: &ItemPreview, metadata: &EntryMetadata) -> Result<()> {
    if let Some(path) = &preview.content_path {
        if terminal_supports_images() {
            let mut config = ViuerConfig::default();
            config.restore_cursor = false;
            if viuer::print_from_file(path, &config).is_ok() {
                return Ok(());
            }
        }
    }
    println!("{}", preview_snippet(preview, metadata));
    Ok(())
}

fn print_file_paths(metadata: &EntryMetadata, preview: &ItemPreview) {
    if metadata.sources.is_empty() {
        for file in &preview.files {
            println!("{}", file.path.display());
        }
    } else {
        for source in &metadata.sources {
            println!("{}", source);
        }
    }
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

fn log_copy(metadata: &EntryMetadata, preview: &ItemPreview) {
    let snippet = preview_snippet(preview, metadata).replace('\n', " ");
    eprintln!("Copied: {}", snippet);
}

fn normalize(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        fs::canonicalize(&path).unwrap_or(path)
    }
}
