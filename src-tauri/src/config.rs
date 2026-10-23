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
    /// HTTP/SOCKS proxy for the search sidecar. Empty falls back to the system env vars.
    #[serde(default)]
    pub proxy: String,
}

fn default_download_dir() -> String {
    directories::UserDirs::new()
        .and_then(|u| u.download_dir().map(|p| p.join("kovcheg")))
        .unwrap_or_else(|| PathBuf::from("kovcheg"))
        .to_string_lossy()
        .into_owned()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            download_dir: default_download_dir(),
            trackers: vec![],
            install_dir: String::new(),
            proxy: String::new(),
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

/// One-time hop from the old torlink identity so an upgrade does not lose the library,
/// queue or settings. Copies, never moves: a stale torlink folder is harmless.
fn migrate_legacy_state(new_data: &Path, new_config: &Path) {
    if std::env::var("KOVCHEG_STATE_DIR").is_ok() || std::env::var("TORLINK_STATE_DIR").is_ok() {
        return;
    }
    if new_data.join("library.json").exists() || new_data.join("queue.json").exists() {
        return;
    }
    let Some(old) = ProjectDirs::from("", "", "torlink") else {
        return;
    };
    let old_data = old.data_dir();
    if !old_data.is_dir() {
        return;
    }
    let _ = std::fs::create_dir_all(new_data);
    let _ = std::fs::create_dir_all(new_config);
    for name in ["library.json", "queue.json", "history.json", "seeds.json"] {
        let from = old_data.join(name);
        let to = new_data.join(name);
        if from.is_file() && !to.exists() {
            let _ = std::fs::copy(&from, &to);
        }
    }
    for name in ["covers", "torrents"] {
        let from = old_data.join(name);
        if from.is_dir() {
            let _ = copy_dir(&from, &new_data.join(name));
        }
    }
    let old_config = old.config_dir().join("config.json");
    let new_config_file = new_config.join("config.json");
    if old_config.is_file() && !new_config_file.exists() {
        let _ = std::fs::copy(&old_config, &new_config_file);
    }
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)?.flatten() {
        let path = entry.path();
        let dest = to.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &dest)?;
        } else if !dest.exists() {
            std::fs::copy(&path, &dest)?;
        }
    }
    Ok(())
}

pub fn resolve_paths() -> Paths {
    if let Ok(override_dir) =
        std::env::var("KOVCHEG_STATE_DIR").or_else(|_| std::env::var("TORLINK_STATE_DIR"))
    {
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

    let proj = ProjectDirs::from("", "", "kovcheg").expect("project dirs");
    let data = proj.data_dir().to_path_buf();
    let config = proj.config_dir().to_path_buf();
    migrate_legacy_state(&data, &config);
    Paths {
        config_file: config.join("config.json"),
        queue_file: data.join("queue.json"),
        history_file: data.join("history.json"),
        seeds_file: data.join("seeds.json"),
        torrents_dir: data.join("torrents"),
        data_dir: data,
    }
}

/// Accepts what people actually paste into the field: a bare `127.0.0.1:10809` gets an
/// `http://` prefix. SOCKS is refused instead of being saved and silently failing — the
/// sidecar's ProxyAgent only speaks HTTP proxies.
pub fn normalize_proxy(raw: &str) -> anyhow::Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let lower = trimmed.to_lowercase();
    anyhow::ensure!(
        !lower.starts_with("socks"),
        "only http/https proxies are supported — a socks:// address will not work"
    );
    let with_scheme = if lower.starts_with("http://") || lower.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    };
    let url =
        reqwest::Url::parse(&with_scheme).map_err(|e| anyhow::anyhow!("invalid proxy URL: {e}"))?;
    anyhow::ensure!(url.host_str().is_some(), "invalid proxy URL: no host");
    Ok(with_scheme)
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
    fn normalize_proxy_adds_scheme_and_keeps_http() {
        assert_eq!(
            normalize_proxy(" 127.0.0.1:10809 ").unwrap(),
            "http://127.0.0.1:10809"
        );
        assert_eq!(
            normalize_proxy("http://127.0.0.1:10809").unwrap(),
            "http://127.0.0.1:10809"
        );
        assert_eq!(
            normalize_proxy("https://proxy.local:8443").unwrap(),
            "https://proxy.local:8443"
        );
        assert_eq!(normalize_proxy("   ").unwrap(), "");
    }

    #[test]
    fn normalize_proxy_refuses_socks_and_junk() {
        assert!(normalize_proxy("socks5://127.0.0.1:2080").is_err());
        assert!(normalize_proxy("SOCKS://127.0.0.1:1080").is_err());
        assert!(normalize_proxy("http://").is_err());
    }

    #[test]
    fn load_config_returns_default_for_missing_file() {
        let dir = std::env::temp_dir().join(format!("kovcheg-cfg-{}", std::process::id()));
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
