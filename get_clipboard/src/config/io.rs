use crate::config::model::{AppConfig, default_project_dirs, normalize_path};
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ConfigPaths {
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub data_dir: PathBuf,
}

pub fn resolve_paths() -> ConfigPaths {
    let dirs = default_project_dirs();
    let config_dir = dirs.config_dir().to_path_buf();
    ConfigPaths {
        data_dir: dirs.data_dir().to_path_buf(),
        config_file: config_dir.join("config.json"),
        config_dir,
    }
}

pub fn config_file_path() -> PathBuf {
    resolve_paths().config_file
}

pub fn load_config() -> Result<AppConfig> {
    let paths = resolve_paths();
    if !paths.config_file.exists() {
        fs::create_dir_all(&paths.config_dir)?;
        let config = AppConfig::default();
        save_config_internal(&config, &paths.config_file)?;
        return Ok(config);
    }
    let bytes = fs::read(&paths.config_file)
        .with_context(|| format!("Failed to read config at {}", paths.config_file.display()))?;
    let config: AppConfig = serde_json::from_slice(&bytes)
        .with_context(|| format!("Failed to parse config at {}", paths.config_file.display()))?;
    Ok(config)
}

pub fn save_config(config: &AppConfig) -> Result<()> {
    let paths = resolve_paths();
    fs::create_dir_all(&paths.config_dir)?;
    save_config_internal(config, &paths.config_file)
}

pub fn set_data_dir(path: PathBuf) -> Result<()> {
    let mut config = load_config().unwrap_or_default();
    config.override_data_dir = Some(normalize_path(&path));
    save_config(&config)
}

pub fn move_data_dir(target: PathBuf) -> Result<()> {
    let mut config = load_config().unwrap_or_default();
    let current = config.data_dir();
    let normalized = normalize_path(&target);
    if current == normalized {
        bail!("Directory already set to {}", current.display());
    }
    fs::create_dir_all(&normalized)
        .with_context(|| format!("Failed to create target directory {}", normalized.display()))?;
    for entry in fs::read_dir(&current)? {
        let entry = entry?;
        let source = entry.path();
        let dest = normalized.join(entry.file_name());
        fs::rename(source, dest)?;
    }
    config.override_data_dir = Some(normalized);
    save_config(&config)?;
    Ok(())
}

fn save_config_internal(config: &AppConfig, path: &Path) -> Result<()> {
    let mut value = serde_json::to_value(config)?;
    if let Value::Object(map) = &mut value {
        map.insert(
            "version".into(),
            Value::String(env!("CARGO_PKG_VERSION").to_string()),
        );
    }
    let json = serde_json::to_vec_pretty(&value)?;
    fs::write(path, json).with_context(|| format!("Failed to write config at {}", path.display()))
}

pub fn ensure_data_dir(config: &AppConfig) -> Result<PathBuf> {
    let path = config.data_dir();
    fs::create_dir_all(&path)?;
    Ok(path)
}
