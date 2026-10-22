use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::fs::replace_file;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default = "default_download_dir")]
    pub download_dir: String,
    #[serde(default)]
    pub trackers: Vec<String>,
    /// Empty means "install next to the downloads".
    #[serde(default)]
    pub install_dir: String,
}

fn default_download_dir() -> String {
    directories::UserDirs::new()
        .and_then(|u| u.download_dir().map(|p| p.join("torlink")))
        .unwrap_or_else(|| PathBuf::from("torlink"))
        .to_string_lossy()
        .into_owned()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            download_dir: default_download_dir(),
            trackers: vec![],
            install_dir: String::new(),
        }
    }
}

#[derive(Clone)]
pub struct Paths {
    pub config_file: PathBuf,
    pub queue_file: PathBuf,
    pub history_file: PathBuf,
    pub seeds_file: PathBuf,
    pub torrents_dir: PathBuf,
    pub data_dir: PathBuf,
}

pub fn resolve_paths() -> Paths {
    if let Ok(override_dir) = std::env::var("TORLINK_STATE_DIR") {
        let base = PathBuf::from(override_dir);
        let data = base.join("data");
        let config = base.join("config");
        return Paths {
            config_file: config.join("config.json"),
            queue_file: data.join("queue.json"),
            history_file: data.join("history.json"),
            seeds_file: data.join("seeds.json"),
            torrents_dir: data.join("torrents"),
            data_dir: data,
        };
    }

    let proj = ProjectDirs::from("", "", "torlink").expect("project dirs");
    let data = proj.data_dir().to_path_buf();
    let config = proj.config_dir().to_path_buf();
    Paths {
        config_file: config.join("config.json"),
        queue_file: data.join("queue.json"),
        history_file: data.join("history.json"),
        seeds_file: data.join("seeds.json"),
        torrents_dir: data.join("torrents"),
        data_dir: data,
    }
}

pub fn load_config(paths: &Paths) -> AppConfig {
    let raw = match std::fs::read_to_string(&paths.config_file) {
        Ok(s) => s,
        Err(_) => return AppConfig::default(),
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save_config(paths: &Paths, config: &AppConfig) -> anyhow::Result<()> {
    if let Some(parent) = paths.config_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = paths.config_file.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(config)?)?;
    replace_file(&tmp, &paths.config_file)?;
    Ok(())
}

pub fn ensure_dirs(paths: &Paths) -> anyhow::Result<()> {
    std::fs::create_dir_all(&paths.data_dir)?;
    std::fs::create_dir_all(paths.config_file.parent().unwrap_or(Path::new(".")))?;
    std::fs::create_dir_all(&paths.torrents_dir)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_download_dir() {
        let cfg = AppConfig::default();
        assert!(!cfg.download_dir.is_empty());
        assert!(cfg.trackers.is_empty());
    }

    #[test]
    fn load_config_returns_default_for_missing_file() {
        let dir = std::env::temp_dir().join(format!("torlink-cfg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let paths = Paths {
            config_file: dir.join("config.json"),
            queue_file: dir.join("queue.json"),
            history_file: dir.join("history.json"),
            seeds_file: dir.join("seeds.json"),
            torrents_dir: dir.join("torrents"),
            data_dir: dir.clone(),
        };
        let cfg = load_config(&paths);
        assert!(!cfg.download_dir.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
