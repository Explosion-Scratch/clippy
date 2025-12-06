use crate::api;
use crate::cli::args::{
    ApiArgs, Cli, Command, DirCommand, EntryKind as CliEntryKind, FilterFlags, HistoryArgs,
    PermissionsCmd, SearchArgs, ServiceAction,
};
use crate::clipboard::plugins::{self, DisplayContent, ImageDisplay};
use crate::config::{self, ensure_data_dir, load_config};
use crate::data::model::EntryMetadata;
use crate::data::store::{
    HistoryItem, SelectionFilter, copy_by_selector, delete_entry, human_size, load_history_items,
    load_index, load_metadata, refresh_index, resolve_selector, stream_history_items,
};
use crate::search::SearchOptions;
use crate::service::{self, ServiceStatus, permissions, watch};
use crate::tui;
use crate::util::paste;
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
    let command = cli
        .command
        .unwrap_or(Command::History(HistoryArgs::default()));
    match command {
        Command::Interactive { query } => run_interactive(query),
        Command::Copy { selector, filters } => copy_entry(&selector, &filters),
        Command::Delete { selector, filters } => delete_item(&selector, &filters),
        Command::Show {
            selector,
            filters,
            json,
        } => {
            let mode = if json {
                OutputMode::JsonFull
            } else {
                OutputMode::Text
            };
            show_item(&selector, &filters, mode)
        }
        Command::Watch => watch::run_watch(None),
        Command::Service(args) => run_service(args.action),
        Command::Dir(args) => run_dir(args.command),
        Command::Search(args) => {
            let mode = if args.json {
                if args.full {
                    OutputMode::JsonFull
                } else {
                    OutputMode::JsonSimple
                }
            } else {
                OutputMode::Text
            };
            run_search(args, mode)
        }
        Command::Api(args) => run_api(args),
        Command::History(args) => {
            let mode = if args.json {
                if args.full {
                    OutputMode::JsonFull
                } else {
                    OutputMode::JsonSimple
                }
            } else {
                OutputMode::Text
            };
            print_history(args, mode)
        }
        Command::Paste { selector, filters } => {
            copy_entry(&selector, &filters)?;
            paste::simulate_paste()?;
            Ok(())
        }
        Command::Export { path } => export_command(&path),
        Command::Import { path } => import_command(&path),
        Command::Stats { json } => run_stats(&json),
        Command::Permissions { subcommand } => match subcommand {
            PermissionsCmd::Check => {
                if permissions::check_accessibility() {
                    println!("Accessibility permissions granted");
                    Ok(())
                } else {
                    bail!("Accessibility permissions NOT granted");
                }
            }
            PermissionsCmd::Request => {
                permissions::request_accessibility();
                println!("Opened System Settings to request permissions");
                Ok(())
            }
        },
    }
}

fn run_interactive(query: Option<String>) -> Result<()> {
    tui::start(query)
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
            let json_item =
                plugins::build_full_json_item(&metadata, &item_dir, selector_index, None)?;
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
                None,
            )?;
            let output = to_string_pretty(&json_item)?;
            if !write_line(&output)? {
                return Ok(());
            }
        }
        OutputMode::Text => {
            let content = plugins::build_display_content_with_preference(
                &metadata,
                &item_dir,
                preferred_plugin,
            )?;
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

fn export_command(path: &Path) -> Result<()> {
    use crate::data::store::store_json_item;
    use serde::{Deserialize, Serialize};
    use std::fs::File;
    use std::io::Write;

    #[derive(Serialize)]
    struct ExportData {
        version: String,
        items: Vec<plugins::ClipboardJsonFullItem>,
    }

    refresh_index()?;
    let index = load_index()?;
    let config = load_config()?;
    let data_dir = ensure_data_dir(&config)?;

    let mut options = SearchOptions::default();
    options.limit = None;

    let (items, _) = load_history_items(&index, &options)?;
    let mut export_items = Vec::new();

    println!("Exporting {} items...", items.len());

    for (i, item) in items.iter().enumerate() {
        let item_dir = data_dir.join(&item.metadata.relative_path);
        match plugins::build_full_json_item(&item.metadata, &item_dir, Some(item.offset), None) {
            Ok(full_item) => {
                export_items.push(full_item);
                if (i + 1) % 100 == 0 {
                    println!("  Processed {}/{} items", i + 1, items.len());
                }
            }
            Err(e) => {
                eprintln!("  Warning: Failed to export item {}: {}", item.metadata.hash, e);
            }
        }
    }

    let export_data = ExportData {
        version: env!("CARGO_PKG_VERSION").to_string(),
        items: export_items,
    };

    let json = serde_json::to_string_pretty(&export_data)?;
    let mut file = File::create(path)?;
    file.write_all(json.as_bytes())?;

    println!("Exported {} items to {}", export_data.items.len(), path.display());
    Ok(())
}

fn import_command(path: &Path) -> Result<()> {
    use crate::data::store::store_json_item;
    use serde::Deserialize;
    use std::fs;

    #[derive(Deserialize)]
    struct ImportData {
        version: String,
        items: Vec<plugins::ClipboardJsonFullItem>,
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let import_data: ImportData = serde_json::from_str(&content)
        .with_context(|| "Failed to parse import file")?;

    println!("Importing from version {} ({} items)...", import_data.version, import_data.items.len());

    let mut success_count = 0;
    let mut skip_count = 0;
    let mut error_count = 0;

    for (i, item) in import_data.items.iter().enumerate() {
        let summary = item.summary.as_deref().unwrap_or("(no summary)");
        let truncated = if summary.len() > 50 {
            format!("{}...", &summary[..47])
        } else {
            summary.to_string()
        }.replace('\n', " ");

        match store_json_item(item) {
            Ok(_metadata) => {
                success_count += 1;
                println!("  [{}/{}] Imported: {}", i + 1, import_data.items.len(), truncated);
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("already exists") || err_str.contains("duplicate") {
                    skip_count += 1;
                    println!("  [{}/{}] Skipped (exists): {}", i + 1, import_data.items.len(), truncated);
                } else {
                    error_count += 1;
                    eprintln!("  [{}/{}] Failed: {} - {}", i + 1, import_data.items.len(), truncated, e);
                }
            }
        }
    }

    println!("\nImport complete: {} imported, {} skipped, {} errors", success_count, skip_count, error_count);
    Ok(())
}

fn run_stats(json: &bool) -> Result<()> {
    use std::collections::HashMap;
    use serde::Serialize;
    use std::fs;

    #[derive(Serialize)]
    struct StatsOutput {
        total_items: usize,
        total_size: u64,
        actual_storage_size: u64,
        type_counts: HashMap<String, usize>,
        largest_items: Vec<LargeItem>,
    }

    #[derive(Serialize, Clone)]
    struct LargeItem {
        hash: String,
        kind: String,
        storage_size: u64,
        summary: Option<String>,
    }

    refresh_index()?;
    let index = load_index()?;
    let config = load_config()?;
    let data_dir = ensure_data_dir(&config)?;

    let total_items = index.len();
    let total_size: u64 = index.values().map(|r| r.byte_size).sum();

    let mut type_counts: HashMap<String, usize> = HashMap::new();
    let mut items_with_storage: Vec<(String, String, u64, Option<String>, usize)> = Vec::new();
    let mut actual_storage_size: u64 = 0;

    // Build ordered index to get offsets
    let mut ordered: Vec<_> = index.values().collect();
    ordered.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));
    let offsets: HashMap<String, usize> = ordered
        .iter()
        .enumerate()
        .map(|(idx, record)| (record.hash.clone(), idx))
        .collect();

    for record in index.values() {
        let kind_str = match record.kind {
            crate::data::model::EntryKind::Text => "text",
            crate::data::model::EntryKind::Image => "image",
            crate::data::model::EntryKind::File => "file",
            crate::data::model::EntryKind::Other => "other",
        };
        *type_counts.entry(kind_str.to_string()).or_insert(0) += 1;

        let item_dir = data_dir.join(&record.relative_path);
        let storage_bytes = compute_dir_storage(&item_dir);
        actual_storage_size += storage_bytes;

        let summary = record.summary.clone();
        let offset = offsets.get(&record.hash).copied().unwrap_or(0);
        items_with_storage.push((
            record.hash.clone(),
            kind_str.to_string(),
            storage_bytes,
            summary,
            offset,
        ));
    }

    items_with_storage.sort_by(|a, b| b.2.cmp(&a.2));
    let largest: Vec<(LargeItem, usize)> = items_with_storage
        .into_iter()
        .take(20)
        .map(|(hash, kind, storage_size, summary, offset)| {
            (LargeItem {
                hash,
                kind,
                storage_size,
                summary,
            }, offset)
        })
        .collect();

    if *json {
        let output = StatsOutput {
            total_items,
            total_size,
            actual_storage_size,
            type_counts,
            largest_items: largest.iter().map(|(item, _)| item.clone()).collect(),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Clipboard Statistics");
        println!("====================");
        println!("Total items:    {}", total_items);
        println!("Reported size:  {}", human_size(total_size));
        println!("Storage size:   {}", human_size(actual_storage_size));
        println!();
        println!("By type:");
        for (type_name, count) in &type_counts {
            println!("  {:10} {}", type_name, count);
        }
        println!();
        println!("Top 20 Largest Items (by storage):");
        println!("{:<8} {:<10} {:<12} {}", "Index", "Type", "Size", "Summary");
        println!("{}", "-".repeat(70));
        for (item, offset) in largest.iter() {
            let summary = item.summary.as_deref().unwrap_or("(no summary)");
            let truncated = if summary.len() > 40 {
                format!("{}...", &summary[..37])
            } else {
                summary.to_string()
            }.replace('\n', " ");
            println!(
                "{:<8} {:<10} {:<12} {}",
                offset,
                item.kind,
                human_size(item.storage_size),
                truncated
            );
        }
    }

    Ok(())
}

fn compute_dir_storage(path: &Path) -> u64 {
    use std::fs;

    if !path.exists() {
        return 0;
    }
    
    if path.is_file() {
        return fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    }
    
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_file() {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            } else if entry_path.is_dir() {
                total += compute_dir_storage(&entry_path);
            }
        }
    }
    total
}

fn print_history(args: HistoryArgs, mode: OutputMode) -> Result<()> {
    refresh_index()?;
    let index = load_index()?;
    let HistoryArgs {
        limit,
        query,
        kind,
        from: from_str,
        to: to_str,
        sort,
        filters,
        ..
    } = args;

    let from = from_str.map(|value| parse_date(&value)).transpose()?;
    let to = to_str.map(|value| parse_date(&value)).transpose()?;
    let selection_filter = build_selection_filter(&filters, kind.clone());

    let mut options = SearchOptions::default();
    let is_interactive = io::stdout().is_terminal();
    options.limit = limit.or_else(|| if is_interactive { Some(100) } else { None });
    options.query = query;
    options.filter = selection_filter;
    options.from = from;
    options.to = to;
    options.sort = match sort {
        Some(crate::cli::args::HistorySort::Date) => crate::search::SortOrder::Date,
        Some(crate::cli::args::HistorySort::Copies) => crate::search::SortOrder::Copies,
        Some(crate::cli::args::HistorySort::Type) => crate::search::SortOrder::Type,
        None => crate::search::SortOrder::Date,
    };

    match mode {
        OutputMode::Text => {
            stream_history_items(&index, &options, |item| output_single_item(item, mode))
        }
        _ => {
            let (items, _) = load_history_items(&index, &options)?;
            output_history(&items, mode)
        }
    }
}

fn run_search(args: SearchArgs, mode: OutputMode) -> Result<()> {
    refresh_index()?;
    let index = load_index()?;
    let SearchArgs {
        query,
        limit,
        sort,
        regex,
        filters,
        ..
    } = args;

    let (query, is_regex, mut selection_filter) = crate::search::parse_search_query(&query, regex);
    let extra_filter = build_selection_filter(&filters, None);

    if extra_filter.include_text {
        selection_filter.include_text = true;
    }
    if extra_filter.include_image {
        selection_filter.include_image = true;
    }
    if extra_filter.include_file {
        selection_filter.include_file = true;
    }
    if extra_filter.include_other {
        selection_filter.include_other = true;
    }
    if extra_filter.include_html {
        selection_filter.include_html = true;
    }
    selection_filter
        .include_formats
        .extend(extra_filter.include_formats);

    let mut options = SearchOptions::default();
    options.limit = limit;
    options.query = Some(query);
    options.filter = selection_filter;
    options.regex = is_regex;
    options.sort = match sort {
        Some(crate::cli::args::SearchSort::Date) => crate::search::SortOrder::Date,
        Some(crate::cli::args::SearchSort::Copies) => crate::search::SortOrder::Copies,
        Some(crate::cli::args::SearchSort::Type) => crate::search::SortOrder::Type,
        Some(crate::cli::args::SearchSort::Relevance) => crate::search::SortOrder::Relevance,
        None => crate::search::SortOrder::Date,
    };

    match mode {
        OutputMode::Text => {
            stream_history_items(&index, &options, |item| output_single_item(item, mode))
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
            
            let config = load_config()?;
            let data_dir = ensure_data_dir(&config)?;
            let item_dir = data_dir.join(&item.metadata.relative_path);
            
            let raw_summary = plugins::build_summary(&item.metadata, &item_dir, is_interactive)
                .unwrap_or_else(|| item.summary.clone());
            
            let summary = if is_interactive {
                clip_summary_to_width(
                    &raw_summary,
                    terminal_width,
                    item.offset,
                    &timestamp,
                    copies,
                    item.global_offset,
                )
            } else {
                clean_summary(&raw_summary)
            };
            let line = format!(
                "{} ({}) [{} x{}]   {}",
                item.offset, item.global_offset, timestamp, copies, summary
            );
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
                    None,
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

            let config = load_config()?;
            let data_dir = ensure_data_dir(&config)?;

            for item in items {
                let item_dir = data_dir.join(&item.metadata.relative_path);
                let timestamp = format_history_timestamp(item.metadata.last_seen);
                let copies = item.metadata.copy_count;
                
                let raw_summary = plugins::build_summary(&item.metadata, &item_dir, is_interactive)
                    .unwrap_or_else(|| item.summary.clone());
                
                let summary = if is_interactive {
                    clip_summary_to_width(
                        &raw_summary,
                        terminal_width,
                        item.offset,
                        &timestamp,
                        copies,
                        item.global_offset,
                    )
                } else {
                    clean_summary(&raw_summary)
                };
                let line = format!(
                    "{} ({}) [{} x{}]   {}",
                    item.offset, item.global_offset, timestamp, copies, summary
                );
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
    if filters.html {
        selection.include_formats.push("html".to_string());
    }
    if filters.rtf {
        selection.include_formats.push("rtf".to_string());
    }
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
    global_index: usize,
) -> String {
    let clean = clean_summary(input);
    let prefix = format!(
        "{} ({}) [{} x{}]   ",
        index, global_index, timestamp, copies
    );
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
