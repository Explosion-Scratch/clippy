use super::ServiceStatus;
use crate::config::io::resolve_paths;
use anyhow::{Result, anyhow, bail};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const TASK_NAME: &str = "ClippyGetClipboard";

pub fn install_agent() -> Result<()> {
    let exe = std::env::current_exe()?;
    let paths = resolve_paths();
    fs::create_dir_all(&paths.config_dir)?;
    let log_path = paths.config_dir.join("service.log");
    // Ensure task runs 'watch' instead of just the bare executable
    let command = format!(
        "cmd /C \"\\\"{}\\\" watch >> \\\"{}\\\" 2>&1\"",
        exe.to_string_lossy(),
        log_path.to_string_lossy()
    );
    let status = Command::new("schtasks")
        .args([
            "/Create", "/SC", "ONLOGON", "/RL", "LIMITED", "/TN", TASK_NAME, "/F", "/TR", &command,
        ])
        .status()?;
    if !status.success() {
        return Err(anyhow!("schtasks exited with status {}", status));
    }
    println!("Installed scheduled task {}", TASK_NAME);
    println!("Service logs: {}", log_path.display());
    Ok(())
}

pub fn uninstall_agent() -> Result<()> {
    let status = Command::new("schtasks")
        .args(["/Delete", "/TN", TASK_NAME, "/F"])
        .status()?;
    if status.success() {
        println!("Removed scheduled task {}", TASK_NAME);
    } else {
        println!("Scheduled task {} not found", TASK_NAME);
    }
    Ok(())
}

pub fn start_agent() -> Result<()> {
    println!("Starting scheduled task {}", TASK_NAME);
    let status = Command::new("schtasks")
        .args(["/Run", "/TN", TASK_NAME])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("schtasks exited with status {}", status))
    }
}

pub fn stop_agent() -> Result<()> {
    println!("Stopping scheduled task {}", TASK_NAME);
    let status = Command::new("schtasks")
        .args(["/End", "/TN", TASK_NAME])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("schtasks exited with status {}", status))
    }
}

pub fn service_status() -> Result<ServiceStatus> {
    let query_status = Command::new("schtasks")
        .args(["/Query", "/TN", TASK_NAME])
        .status();
    let installed = query_status.map(|status| status.success()).unwrap_or(false);
    let mut running = false;
    let mut details = Vec::new();

    if installed {
        if let Ok(output) = Command::new("schtasks")
            .args(["/Query", "/TN", TASK_NAME, "/FO", "LIST", "/V"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.trim_start().starts_with("Status:") {
                    if line.contains("Running") {
                        running = true;
                    }
                    details.push(("status".into(), line.trim().into()));
                }
                if line.trim_start().starts_with("Last Run Time:") {
                    details.push(("last_run".into(), line.trim().into()));
                }
            }
        }
    }

    let mut status = ServiceStatus {
        installed,
        running,
        details,
    };
    status.details.push(("task".into(), TASK_NAME.into()));
    status.details.push((
        "log".into(),
        log_file_path()?.to_string_lossy().into_owned(),
    ));
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
    let escaped = escape_powershell_path(&log_path);
    let mut script = format!("Get-Content -Path '{}' -Tail {}", escaped, lines);
    if follow {
        script.push_str(" -Wait");
    }
    let status = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("powershell exited with status {}", status))
    }
}

fn log_file_path() -> Result<PathBuf> {
    let paths = resolve_paths();
    Ok(paths.config_dir.join("service.log"))
}

fn escape_powershell_path(path: &PathBuf) -> String {
    path.to_string_lossy().replace('\'', "''")
}
