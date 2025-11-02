use super::ServiceStatus;
use anyhow::{Result, bail};

fn unsupported() -> Result<()> {
    bail!("Service management is not supported on this platform")
}

pub fn install_agent() -> Result<()> {
    unsupported()
}

pub fn uninstall_agent() -> Result<()> {
    unsupported()
}

pub fn start_agent() -> Result<()> {
    unsupported()
}

pub fn stop_agent() -> Result<()> {
    unsupported()
}

pub fn service_status() -> Result<ServiceStatus> {
    bail!("Service management is not supported on this platform")
}

pub fn print_logs(_lines: usize, _follow: bool) -> Result<()> {
    unsupported()
}
