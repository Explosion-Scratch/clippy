use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about = "Minimal yet powerful clipboard history for macOS", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Args, Debug, Clone, Default)]
pub struct FilterFlags {
    #[arg(long, help = "Filter to text items only")]
    pub text: bool,
    #[arg(long, help = "Filter to image items only")]
    pub image: bool,
    #[arg(long, help = "Filter to file items only")]
    pub file: bool,
    #[arg(long, help = "Filter to items containing HTML")]
    pub html: bool,
    #[arg(long, help = "Filter to items containing RTF")]
    pub rtf: bool,
}

impl FilterFlags {
    pub fn is_empty(&self) -> bool {
        !self.text && !self.image && !self.file && !self.html && !self.rtf
    }
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    #[command(about = "Interactive TUI for browsing and selecting clipboard items")]
    Interactive {
        #[arg(short, long, help = "Initial search query")]
        query: Option<String>,
    },
    #[command(about = "Copy a clipboard item back to the clipboard")]
    Copy {
        #[arg(default_value = "0", help = "Item selector (index, hash, or search term)")]
        selector: String,
        #[command(flatten)]
        filters: FilterFlags,
    },
    #[command(about = "Delete a clipboard item")]
    Delete {
        #[arg(default_value = "0", help = "Item selector (index, hash, or search term)")]
        selector: String,
        #[command(flatten)]
        filters: FilterFlags,
    },
    #[command(about = "Show details of a clipboard item")]
    Show {
        #[arg(default_value = "0", help = "Item selector (index, hash, or search term)")]
        selector: String,
        #[command(flatten)]
        filters: FilterFlags,
        #[arg(long, help = "Output in JSON format")]
        json: bool,
    },
    #[command(about = "Watch for new clipboard items")]
    Watch,
    #[command(about = "Manage the background service")]
    Service(ServiceArgs),
    #[command(about = "Manage the data directory")]
    Dir(DirArgs),
    #[command(about = "Search clipboard history")]
    Search(SearchArgs),
    #[command(about = "Start the HTTP API server")]
    Api(ApiArgs),
    #[command(about = "List clipboard history")]
    History(HistoryArgs),
    #[command(about = "Copy item to clipboard and paste it")]
    Paste {
        #[arg(default_value = "0", help = "Item selector (index, hash, or search term)")]
        selector: String,
        #[command(flatten)]
        filters: FilterFlags,
    },
    #[command(about = "Export clipboard history to a JSON file")]
    Export {
        #[arg(help = "Path to the export file")]
        path: PathBuf,
    },
    #[command(about = "Import clipboard history from a JSON file")]
    Import {
        #[arg(help = "Path to the import file")]
        path: PathBuf,
    },
    #[command(about = "Show clipboard statistics")]
    Stats {
        #[arg(long, help = "Output in JSON format")]
        json: bool,
    },
    #[command(about = "Manage accessibility permissions")]
    Permissions {
        #[command(subcommand)]
        subcommand: PermissionsCmd,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum PermissionsCmd {
    #[command(about = "Check if accessibility permissions are granted")]
    Check,
    #[command(about = "Request accessibility permissions")]
    Request,
}

#[derive(Parser, Debug, Clone)]
pub struct ServiceArgs {
    #[command(subcommand)]
    pub action: ServiceAction,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ServiceAction {
    #[command(about = "Install the background service")]
    Install,
    #[command(about = "Uninstall the background service")]
    Uninstall,
    #[command(about = "Start the background service")]
    Start,
    #[command(about = "Stop the background service")]
    Stop,
    #[command(about = "Check the status of the background service")]
    Status,
    #[command(about = "View service logs")]
    Logs {
        #[arg(short = 'n', long, default_value_t = 200, help = "Number of log lines to show")]
        lines: usize,
        #[arg(short, long, help = "Follow log output in real-time")]
        follow: bool,
    },
}

#[derive(Parser, Debug, Clone)]
pub struct DirArgs {
    #[command(subcommand)]
    pub command: DirCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum DirCommand {
    #[command(about = "Get the current data directory path")]
    Get,
    #[command(about = "Set the data directory path (does not move existing data)")]
    Set {
        #[arg(help = "New data directory path")]
        path: PathBuf,
    },
    #[command(about = "Move data to a new directory")]
    Move {
        #[arg(help = "New data directory path")]
        path: PathBuf,
    },
}

#[derive(Parser, Debug, Clone, Default)]
pub struct HistoryArgs {
    #[arg(short, long, help = "Maximum number of items to show")]
    pub limit: Option<usize>,
    #[arg(short, long, help = "Filter by search query")]
    pub query: Option<String>,
    #[arg(long, value_enum, help = "Filter by item type")]
    pub kind: Option<EntryKind>,
    #[arg(long, help = "Filter items from this date (YYYY-MM-DD or relative like '1d')")]
    pub from: Option<String>,
    #[arg(long, help = "Filter items until this date (YYYY-MM-DD or relative like '1d')")]
    pub to: Option<String>,
    #[arg(long, help = "Include full content in JSON output")]
    pub full: bool,
    #[arg(long, help = "Treat query as a regular expression")]
    pub regex: bool,
    #[arg(long, value_enum, help = "Sort order")]
    pub sort: Option<HistorySort>,
    #[command(flatten)]
    pub filters: FilterFlags,
    #[arg(long, help = "Output in JSON format")]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct SearchArgs {
    #[arg(help = "Search query (supports operators like type:image)")]
    pub query: String,
    #[arg(short, long, help = "Maximum number of results")]
    pub limit: Option<usize>,
    #[arg(long, help = "Include full content in JSON output")]
    pub full: bool,
    #[arg(long, help = "Treat query as a regular expression")]
    pub regex: bool,
    #[arg(long, value_enum, help = "Sort order")]
    pub sort: Option<SearchSort>,
    #[command(flatten)]
    pub filters: FilterFlags,
    #[arg(long, help = "Output in JSON format")]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ApiArgs {
    #[arg(long, default_value_t = 3016, help = "Port to listen on")]
    pub port: u16,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum EntryKind {
    Text,
    Image,
    File,
    Other,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum HistorySort {
    Date,
    Copies,
    Type,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum SearchSort {
    Date,
    Copies,
    Type,
    Relevance,
}
