use crate::config::io::resolve_paths;
use crate::util::time;
use anyhow::{Result, anyhow, bail};
use directories::BaseDirs;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const LABEL: &str = "com.clippith.get_clipboard";

pub fn install_agent() -> Result<()> {
    let plist_path = agent_plist_path()?;
    let content = build_plist()?;
    if let Some(dir) = plist_path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&plist_path, content)?;
    run_launchctl(["load", "-w", plist_path.to_string_lossy().as_ref()])
}

pub fn uninstall_agent() -> Result<()> {
    let plist_path = agent_plist_path()?;
    if plist_path.exists() {
        let _ = run_launchctl(["unload", "-w", plist_path.to_string_lossy().as_ref()]);
        fs::remove_file(&plist_path)?;
    }
    Ok(())
}

pub fn start_agent() -> Result<()> {
    run_launchctl(["start", LABEL])
}

pub fn stop_agent() -> Result<()> {
    run_launchctl(["stop", LABEL])
}

pub fn print_logs(lines: usize, follow: bool) -> Result<()> {
    let log_path = log_file_path()?;
    if !log_path.exists() {
        bail!("Log file not found at {}", log_path.display());
    }
    let mut command = Command::new("tail");
    command.arg("-n").arg(lines.to_string());
    if follow {
        command.arg("-f");
    }
    command.arg(log_path.to_string_lossy().as_ref());
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("tail exited with status {}", status))
    }
}

fn agent_plist_path() -> Result<PathBuf> {
    let dirs = BaseDirs::new().ok_or_else(|| anyhow!("Missing base directories"))?;
    Ok(dirs
        .home_dir()
        .join("Library/LaunchAgents")
        .join("com.clippith.get_clipboard.plist"))
}

fn build_plist() -> Result<String> {
    let exe = std::env::current_exe()?;
    let paths = resolve_paths();
    fs::create_dir_all(&paths.config_dir)?;
    let log_path = paths.config_dir.join("service.log");
    let now = time::format_human(time::now());
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n    <key>Label</key>\n    <string>{LABEL}</string>\n    <key>ProgramArguments</key>\n    <array>\n        <string>{}</string>\n        <string>watch</string>\n    </array>\n    <key>RunAtLoad</key>\n    <true/>\n    <key>KeepAlive</key>\n    <true/>\n    <key>StandardErrorPath</key>\n    <string>{}</string>\n    <key>StandardOutPath</key>\n    <string>{}</string>\n    <key>EnvironmentVariables</key>\n    <dict>\n        <key>GET_CLIPBOARD_STARTED</key>\n        <string>{}</string>\n    </dict>\n</dict>\n</plist>\n",
        exe.to_string_lossy(),
        log_path.to_string_lossy(),
        log_path.to_string_lossy(),
        now
    ))
}

fn log_file_path() -> Result<PathBuf> {
    let paths = resolve_paths();
    Ok(paths.config_dir.join("service.log"))
}

fn run_launchctl<const N: usize>(args: [&str; N]) -> Result<()> {
    let status = Command::new("launchctl").args(args).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("launchctl exited with status {}", status))
    }
}
