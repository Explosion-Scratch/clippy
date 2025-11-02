use super::ServiceStatus;
use crate::config::io::resolve_paths;
use crate::util::time;
use anyhow::{Context, Result, anyhow, bail};
use directories::BaseDirs;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const SERVICE_NAME: &str = "get_clipboard.service";
const UNIT_ID: &str = "get_clipboard";

pub fn install_agent() -> Result<()> {
    let unit_path = service_unit_path()?;
    let content = build_unit()?;
    if let Some(dir) = unit_path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&unit_path, content)?;
    run_systemctl(&["--user", "daemon-reload"])?;
    run_systemctl(&["--user", "enable", "--now", UNIT_ID])?;
    println!("Installed systemd unit at {}", unit_path.display());
    println!("Service logs: {}", log_file_path()?.display());
    Ok(())
}

pub fn uninstall_agent() -> Result<()> {
    let unit_path = service_unit_path()?;
    if unit_path.exists() {
        let _ = run_systemctl(&["--user", "disable", "--now", UNIT_ID]);
        fs::remove_file(&unit_path)?;
        run_systemctl(&["--user", "daemon-reload"])?;
        println!("Removed systemd unit {}", unit_path.display());
    } else {
        println!("Systemd unit not installed");
    }
    Ok(())
}

pub fn start_agent() -> Result<()> {
    println!("Starting systemd service {}", UNIT_ID);
    run_systemctl(&["--user", "start", UNIT_ID])
}

pub fn stop_agent() -> Result<()> {
    println!("Stopping systemd service {}", UNIT_ID);
    run_systemctl(&["--user", "stop", UNIT_ID])
}

pub fn service_status() -> Result<ServiceStatus> {
    let unit_path = service_unit_path()?;
    let installed = unit_path.exists();
    let running = if installed {
        Command::new("systemctl")
            .args(["--user", "is-active", UNIT_ID])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    } else {
        false
    };

    let mut status = ServiceStatus::new(installed, running)
        .with_detail("unit", unit_path.to_string_lossy())
        .with_detail("log", log_file_path()?.to_string_lossy());

    if installed {
        if let Ok(output) = Command::new("systemctl")
            .args(["--user", "status", UNIT_ID])
            .output()
        {
            if !output.stdout.is_empty() {
                let snippet = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .take(6)
                    .collect::<Vec<_>>()
                    .join(" | ");
                if !snippet.is_empty() {
                    status.details.push(("systemctl".into(), snippet));
                }
            }
        }
    }

    Ok(status)
}

pub fn print_logs(lines: usize, follow: bool) -> Result<()> {
    let log_path = log_file_path()?;
    if !log_path.exists() {
        bail!("Log file not found at {}", log_path.display());
    }
    println!("Streaming logs from {}", log_path.display());
    if follow {
        println!("Press Ctrl+C to stop following logs.");
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

fn service_unit_path() -> Result<PathBuf> {
    let dirs = BaseDirs::new().ok_or_else(|| anyhow!("Missing base directories"))?;
    Ok(dirs
        .home_dir()
        .join(".config/systemd/user")
        .join(SERVICE_NAME))
}

fn build_unit() -> Result<String> {
    let exe = std::env::current_exe()?;
    let paths = resolve_paths();
    fs::create_dir_all(&paths.config_dir)?;
    let log_path = paths.config_dir.join("service.log");
    let timestamp = time::format_human(time::now());
    Ok(format!(
        "[Unit]\nDescription=get_clipboard clipboard watcher\nAfter=default.target\n\n[Service]\nExecStart={} watch\nRestart=always\nEnvironment=GET_CLIPBOARD_STARTED={}\nStandardOutput=append:{}\nStandardError=append:{}\n\n[Install]\nWantedBy=default.target\n",
        exe.to_string_lossy(),
        timestamp,
        log_path.to_string_lossy(),
        log_path.to_string_lossy()
    ))
}

fn log_file_path() -> Result<PathBuf> {
    let paths = resolve_paths();
    Ok(paths.config_dir.join("service.log"))
}

fn run_systemctl(args: &[&str]) -> Result<()> {
    let status = Command::new("systemctl").args(args).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("systemctl exited with status {}", status))
    }
}
