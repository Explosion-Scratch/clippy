use crate::cli::args::{Cli, Command, DirCommand, HistoryArgs, ServiceAction};
use crate::config::{AppConfig, load_config, save_config};
use crate::data::store::{
    copy_by_selector, history_stream, load_index, refresh_index, resolve_selector,
};
use crate::service::{launchd, watch};
use crate::tui;
use crate::util::time::parse_date;
use anyhow::{Context, Result, bail};
use image::GenericImageView;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn dispatch(cli: Cli) -> Result<()> {
    let command = cli
        .command
        .unwrap_or(Command::History(HistoryArgs::default()));
    match command {
        Command::Interactive { query } => run_interactive(query),
        Command::Copy { selector } => copy_entry(&selector),
        Command::Watch => watch::run_watch(None),
        Command::Service(args) => run_service(args.action),
        Command::Dir(args) => run_dir(args.command),
        Command::History(args) => print_history(args),
    }
}

fn run_interactive(query: Option<String>) -> Result<()> {
    refresh_index()?;
    let index = load_index()?;
    tui::start(index, query)
}

fn copy_entry(selector: &str) -> Result<()> {
    refresh_index()?;
    let index = load_index()?;
    let target = resolve_selector(&index, selector)
        .with_context(|| format!("No clipboard item found for selector {selector}"))?;
    copy_by_selector(&index, &target)
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

fn get_image_info(image_path: &Path, byte_size: u64) -> String {
    let size_kb = byte_size as f64 / 1024.0;

    if let Ok(img) = image::open(image_path) {
        let (width, height) = img.dimensions();
        format!("[Image {}x{} {:.1}KB]", width, height, size_kb)
    } else {
        format!("[Image {:.1}KB]", size_kb)
    }
}

fn print_history(args: HistoryArgs) -> Result<()> {
    refresh_index()?;
    let index = load_index()?;
    let from = args.from.map(|d| parse_date(&d)).transpose()?;
    let to = args.to.map(|d| parse_date(&d)).transpose()?;
    let items = history_stream(&index, args.limit, args.query, args.kind, from, to)?;

    println!(
        "{:<6} {:<12} {:<19} {:<8} {}",
        "OFFSET", "ID", "DATE", "TYPE", "CONTENT"
    );
    println!("{}", "-".repeat(100));

    for (idx, item) in items.enumerate() {
        let short_hash = item.metadata.hash.chars().take(12).collect::<String>();
        let date_str = crate::util::time::format_human(item.metadata.last_seen);
        let type_str = format!("{:?}", item.metadata.kind);

        let content = match item.metadata.kind {
            crate::data::model::EntryKind::Image => {
                let config = load_config()?;
                let data_dir = config.data_dir();
                let image_path = data_dir
                    .join(&item.metadata.relative_path)
                    .join(&item.metadata.content_filename);
                get_image_info(&image_path, item.metadata.byte_size)
            }
            crate::data::model::EntryKind::File => {
                item.summary.chars().take(60).collect::<String>()
            }
            _ => item
                .summary
                .chars()
                .take(60)
                .collect::<String>()
                .replace('\n', " ")
                .replace('\r', ""),
        };

        println!(
            "{:<6} {:<12} {:<19} {:<8} {}",
            idx, short_hash, date_str, type_str, content
        );
    }
    Ok(())
}

fn normalize(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        fs::canonicalize(&path).unwrap_or(path)
    }
}
