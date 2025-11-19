use super::ServiceStatus;
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
    
    // Try to unload first to ensure we can update
    if plist_path.exists() {
        let _ = run_launchctl(["unload", plist_path.to_string_lossy().as_ref()]);
    }

    fs::write(&plist_path, content)?;
    run_launchctl(["load", "-w", plist_path.to_string_lossy().as_ref()])?;
    println!(
        "Installed launch agent {} at {}",
        LABEL,
        plist_path.display()
    );
    println!("Service logs: {}", log_file_path()?.display());
    Ok(())
}

pub fn uninstall_agent() -> Result<()> {
    let plist_path = agent_plist_path()?;
    if plist_path.exists() {
        let _ = run_launchctl(["unload", "-w", plist_path.to_string_lossy().as_ref()]);
        fs::remove_file(&plist_path)?;
        println!("Removed launch agent {}", plist_path.display());
    } else {
        println!("Launch agent not installed");
    }
    Ok(())
}

pub fn start_agent() -> Result<()> {
    let plist_path = agent_plist_path()?;
    if !plist_path.exists() {
        bail!("Service is not installed. Run `get_clipboard service install` first.");
    }
    
    let is_running = Command::new("launchctl")
        .args(["list", LABEL])
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    
    if is_running {
        println!("Launch agent {} is already running", LABEL);
        return Ok(());
    }
    
    println!("Starting launch agent {}", LABEL);
    run_launchctl(["load", "-w", plist_path.to_string_lossy().as_ref()])
}

pub fn stop_agent() -> Result<()> {
    let plist_path = agent_plist_path()?;
    if !plist_path.exists() {
        bail!("Service is not installed");
    }
    println!("Stopping launch agent {}", LABEL);
    let _ = run_launchctl(["stop", LABEL]);
    run_launchctl(["unload", plist_path.to_string_lossy().as_ref()])
}

pub fn service_status() -> Result<ServiceStatus> {
    let plist_path = agent_plist_path()?;
    let installed = plist_path.exists();
    let running = if installed {
        Command::new("launchctl")
            .args(["list", LABEL])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    } else {
        false
    };

    let mut status = ServiceStatus::new(installed, running)
        .with_detail("label", LABEL)
        .with_detail("plist", plist_path.to_string_lossy())
        .with_detail("log", log_file_path()?.to_string_lossy());

    if installed {
        if let Ok(output) = Command::new("launchctl").args(["list", LABEL]).output() {
            if !output.stdout.is_empty() {
                let snippet = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .take(5)
                    .collect::<Vec<_>>()
                    .join(" | ");
                if !snippet.is_empty() {
                    status.details.push(("launchctl".into(), snippet));
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
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n    <key>Label</key>\n    <string>{LABEL}</string>\n    <key>ProgramArguments</key>\n    <array>\n        <string>{}</string>\n        <string>api</string>\n        <string>--port</string>\n        <string>3016</string>\n    </array>\n    <key>RunAtLoad</key>\n    <true/>\n    <key>KeepAlive</key>\n    <true/>\n    <key>StandardErrorPath</key>\n    <string>{}</string>\n    <key>StandardOutPath</key>\n    <string>{}</string>\n    <key>EnvironmentVariables</key>\n    <dict>\n        <key>GET_CLIPBOARD_STARTED</key>\n        <string>{}</string>\n    </dict>\n</dict>\n</plist>\n",
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
