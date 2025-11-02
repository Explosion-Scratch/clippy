mod api;
mod cli;
mod clipboard;
mod config;
mod data;
mod fs;
mod search;
mod service;
mod tui;
mod util;

fn main() {
    let _ = color_eyre::install();
    if let Err(err) = cli::run() {
        eprintln!("{err:?}");
        std::process::exit(1);
    }
}
