use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::Paths;
use crate::fs::replace_file;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Downloading,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SeedStatus {
    Seeding,
    Paused,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PieceMap {
    pub total: u32,
    pub states: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub magnet: String,
    pub dir: String,
    pub status: DownloadStatus,
    pub progress: u32,
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "downloadedBytes")]
    pub downloaded_bytes: u64,
    pub speed: u64,
    pub peers: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(rename = "pieceMap", skip_serializing_if = "Option::is_none")]
    pub piece_map: Option<PieceMap>,
    #[serde(rename = "addedAt")]
    pub added_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedItem {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub magnet: String,
    pub dir: String,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: u64,
    pub status: SeedStatus,
    #[serde(rename = "uploadSpeed")]
    pub upload_speed: u64,
    pub uploaded: u64,
    pub peers: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedRecord {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub magnet: String,
    pub dir: String,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: u64,
    #[serde(rename = "completedAt")]
    pub completed_at: u64,
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Vec<T> {
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(err) => {
            // A half-written file must not look like "the user has nothing": keep it aside,
            // otherwise the next save silently replaces real data with an empty list.
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let backup = path.with_extension(format!("corrupt-{stamp}"));
            tracing::error!(
                "{} is unreadable ({err:#}); moved to {}",
                path.display(),
                backup.display()
            );
            let _ = std::fs::rename(path, &backup);
            vec![]
        }
    }
}

fn write_json_atomic<T: Serialize + ?Sized>(path: &Path, value: &T) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(value)?)?;
    replace_file(&tmp, path)?;
    Ok(())
}

pub fn load_queue(paths: &Paths) -> Vec<QueueItem> {
    read_json(&paths.queue_file)
}

pub fn save_queue(paths: &Paths, items: &[QueueItem]) -> anyhow::Result<()> {
    write_json_atomic(&paths.queue_file, items)
}

pub fn load_seeds(paths: &Paths) -> Vec<SeedRecord> {
    read_json(&paths.seeds_file)
}

pub fn save_seeds(paths: &Paths, records: &[SeedRecord]) -> anyhow::Result<()> {
    write_json_atomic(&paths.seeds_file, records)
}

pub fn load_history(paths: &Paths) -> Vec<HistoryItem> {
    read_json(&paths.history_file)
}

pub fn save_history(paths: &Paths, items: &[HistoryItem]) -> anyhow::Result<()> {
    write_json_atomic(&paths.history_file, items)
}

pub fn safe_torrent_id(id: &str) -> Result<String, String> {
    let id = id.trim().to_ascii_lowercase();
    if id.is_empty() {
        return Err("torrent id cannot be empty".into());
    }
    if id.chars().all(|c| c.is_ascii_hexdigit()) && (id.len() == 32 || id.len() == 40) {
        return Ok(id);
    }
    if id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Ok(id);
    }
    Err("invalid torrent id".into())
}

pub fn torrent_meta_path(paths: &Paths, id: &str) -> std::path::PathBuf {
    let safe = safe_torrent_id(id).unwrap_or_else(|_| "invalid".into());
    paths.torrents_dir.join(format!("{safe}.torrent"))
}

pub fn save_torrent_meta(paths: &Paths, id: &str, data: &[u8]) -> anyhow::Result<()> {
    let safe = safe_torrent_id(id).map_err(|e| anyhow::anyhow!(e))?;
    let path = paths.torrents_dir.join(format!("{safe}.torrent"));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("torrent.tmp");
    std::fs::write(&tmp, data)?;
    replace_file(&tmp, &path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Paths;

    #[test]
    fn load_queue_returns_empty_on_corrupt_file() {
        let dir = std::env::temp_dir().join(format!("kovcheg-persist-{}", std::process::id()));
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
        std::fs::write(&paths.queue_file, "{not json").unwrap();
        assert!(load_queue(&paths).is_empty());
        // the broken file is kept aside instead of being overwritten by the next save
        let backups: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains(".corrupt-"))
            .collect();
        assert_eq!(backups.len(), 1, "expected one .corrupt- backup");
        assert!(
            !paths.queue_file.exists(),
            "the corrupt file should be moved away"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn queue_item_round_trips_through_json() {
        let item = QueueItem {
            id: "abc".into(),
            name: "Test".into(),
            source: Some("yts".into()),
            magnet: "magnet:?xt=urn:btih:abc".into(),
            dir: "/tmp".into(),
            status: DownloadStatus::Downloading,
            progress: 10,
            total_bytes: 100,
            downloaded_bytes: 10,
            speed: 1,
            peers: 2,
            eta: None,
            files: None,
            error: None,
            piece_map: None,
            added_at: 1,
        };
        let json = serde_json::to_string(&item).unwrap();
        let back: QueueItem = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "abc");
        assert_eq!(back.status, DownloadStatus::Downloading);
    }

    #[test]
    fn safe_torrent_id_accepts_hex_hash() {
        let hash = "abcdef0123456789abcdef0123456789abcdef01";
        assert_eq!(safe_torrent_id(hash).unwrap().len(), 40);
    }

    #[test]
    fn safe_torrent_id_rejects_path_traversal() {
        assert!(safe_torrent_id("../../etc/passwd").is_err());
    }

    #[test]
    fn torrent_meta_path_uses_safe_id() {
        let dir = std::env::temp_dir().join(format!("kovcheg-meta-{}", std::process::id()));
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
        let path = torrent_meta_path(&paths, "abc123def4567890abc123def4567890abc123de");
        assert!(path.ends_with("abc123def4567890abc123def4567890abc123de.torrent"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
