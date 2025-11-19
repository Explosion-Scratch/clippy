pub mod watch;
pub mod permissions;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
mod unsupported;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
use unsupported as platform;
#[cfg(target_os = "windows")]
use windows as platform;

use anyhow::{Result, bail};

#[derive(Debug, Clone)]
pub struct ServiceStatus {
    pub installed: bool,
    pub running: bool,
    pub details: Vec<(String, String)>,
}

impl ServiceStatus {
    pub fn new(installed: bool, running: bool) -> Self {
        ServiceStatus {
            installed,
            running,
            details: Vec::new(),
        }
    }

    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.push((key.into(), value.into()));
        self
    }
}

pub fn install_agent() -> Result<()> {
    platform::install_agent()
}

pub fn uninstall_agent() -> Result<()> {
    platform::uninstall_agent()
}

pub fn start_agent() -> Result<()> {
    let status = service_status()?;
    if !status.installed {
        bail!("Service is not installed. Run `get_clipboard service install` first.");
    }
    platform::start_agent()
}

pub fn stop_agent() -> Result<()> {
    let status = service_status()?;
    if !status.installed {
        bail!("Service is not installed. Run `get_clipboard service install` first.");
    }
    platform::stop_agent()
}

pub fn service_status() -> Result<ServiceStatus> {
    platform::service_status()
}

pub fn print_logs(lines: usize, follow: bool) -> Result<()> {
    platform::print_logs(lines, follow)
}
