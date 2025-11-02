use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about = "Minimal yet powerful clipboard history for macOS", long_about = None)]
pub struct Cli {
    #[command(flatten)]
    pub filters: FilterFlags,
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Args, Debug, Clone, Default)]
pub struct FilterFlags {
    #[arg(long, global = true)]
    pub text: bool,
    #[arg(long, global = true)]
    pub image: bool,
    #[arg(long, global = true)]
    pub file: bool,
    #[arg(long, global = true)]
    pub html: bool,
    #[arg(long, global = true)]
    pub rtf: bool,
}

impl FilterFlags {
    pub fn is_empty(&self) -> bool {
        !self.text && !self.image && !self.file && !self.html && !self.rtf
    }
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    Interactive {
        #[arg(short, long)]
        query: Option<String>,
    },
    Copy {
        #[arg(default_value = "0")]
        selector: String,
    },
    Delete {
        #[arg(default_value = "0")]
        selector: String,
    },
    Show {
        #[arg(default_value = "0")]
        selector: String,
    },
    Watch,
    Service(ServiceArgs),
    Dir(DirArgs),
    Search(SearchArgs),
    Api(ApiArgs),
    History(HistoryArgs),
}

#[derive(Parser, Debug, Clone)]
pub struct ServiceArgs {
    #[command(subcommand)]
    pub action: ServiceAction,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ServiceAction {
    Install,
    Uninstall,
    Start,
    Stop,
    Status,
    Logs {
        #[arg(short = 'n', long, default_value_t = 200)]
        lines: usize,
        #[arg(short, long)]
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
    Get,
    Set { path: PathBuf },
    Move { path: PathBuf },
}

#[derive(Parser, Debug, Clone, Default)]
pub struct HistoryArgs {
    #[arg(short, long)]
    pub limit: Option<usize>,
    #[arg(short, long)]
    pub query: Option<String>,
    #[arg(long, value_enum)]
    pub kind: Option<EntryKind>,
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub to: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct SearchArgs {
    pub query: String,
    #[arg(short, long)]
    pub limit: Option<usize>,
}

#[derive(Args, Debug, Clone)]
pub struct ApiArgs {
    #[arg(long, default_value_t = 3016)]
    pub port: u16,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum EntryKind {
    Text,
    Image,
    File,
    Other,
}
