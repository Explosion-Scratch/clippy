use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use time::{Duration, OffsetDateTime};

const APP_NAME: &str = "get_clipboard";
const ORGANIZATION: &str = "clippith";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub max_items: Option<usize>,
    pub override_data_dir: Option<PathBuf>,
    pub pruning: Option<PrunePolicy>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum PrunePolicy {
    MaxAge { days: u64 },
    MaxCount { count: usize },
}

impl Default for PrunePolicy {
    fn default() -> Self {
        PrunePolicy::MaxCount { count: usize::MAX }
    }
}

impl AppConfig {
    pub fn data_dir(&self) -> PathBuf {
        if let Some(path) = &self.override_data_dir {
            return path.clone();
        }
        default_project_dirs().data_dir().to_path_buf()
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(self.data_dir())
    }

    pub fn should_prune(&self, total_items: usize) -> Option<PruneDirective> {
        match self.pruning.clone().unwrap_or_default() {
            PrunePolicy::MaxCount { count } if total_items > count => {
                Some(PruneDirective::ByCount(total_items - count))
            }
            PrunePolicy::MaxAge { days } => {
                let cutoff = OffsetDateTime::now_utc() - Duration::days(days as i64);
                Some(PruneDirective::ByDate(cutoff))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PruneDirective {
    ByCount(usize),
    ByDate(OffsetDateTime),
}

pub fn default_project_dirs() -> ProjectDirs {
    ProjectDirs::from("com", ORGANIZATION, APP_NAME)
        .expect("Project directories should resolve on macOS")
}

pub fn normalize_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }
}
