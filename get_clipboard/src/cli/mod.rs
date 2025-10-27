pub mod args;
pub mod handlers;

use anyhow::Result;
use clap::Parser;

pub fn run() -> Result<()> {
    let cli = args::Cli::parse();
    handlers::dispatch(cli)
}
