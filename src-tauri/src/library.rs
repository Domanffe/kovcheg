use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::config::{self, Paths};
use crate::fs::replace_file;

const LIBRARY_FILE: &str = "library.json";
const COVERS_DIR: &str = "covers";
const STEAM_SEARCH: &str = "https://store.steampowered.com/api/storesearch/";
const STEAM_CDN: &str = "https://cdn.akamai.steamstatic.com/steam/apps";
const SCAN_DEPTH: u8 = 2;
/// Hard stop so a wrong path (a drive root) cannot turn into an endless directory walk.
const SCAN_LIMIT: usize = 4000;
/// Helper binaries that sit next to a game but are never it. Matched as substrings on the
/// stem, so they must stay specific: "crash" alone would swallow "CrashoutCrew.exe".
const EXE_SKIP: [&str; 6] = [
    "unins",
    "vcredist",
    "vc_redist",
    "dxsetup",
    "dxwebsetup",
    "dotnet",
];

fn is_helper_exe(stem: &str) -> bool {
    EXE_SKIP.iter().any(|skip| stem.contains(skip))
        || stem.ends_with("setup")
        || stem.contains("setup_")
        || stem.contains("installer")
        || stem.contains("redist")
        || stem.contains("crashhandler")
        || stem.contains("crashreport")
        || stem.contains("crashpad")
        || stem.contains("unitycrash")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: String,
    pub title: String,
    pub dir: String,
    #[serde(default)]
    pub exe: Option<String>,
    #[serde(default)]
    pub steam_app_id: Option<u32>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub added_at: u64,
    #[serde(default)]
    pub last_played_at: Option<u64>,
    #[serde(default)]
    pub playtime_secs: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameView {
    pub id: String,
    pub title: String,
    pub dir: String,
    pub exe: Option<String>,
    pub cover: Option<String>,
    pub steam_app_id: Option<u32>,
    pub source: Option<String>,
    pub added_at: u64,
    pub last_played_at: Option<u64>,
    pub playtime_secs: u64,
    pub running: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderProbe {
    pub installer: Option<String>,
    pub exe: Option<String>,
    pub silent: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub title: String,
    pub phase: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallerKind {
    Inno,
    Nsis,
    Unknown,
}

struct Running {
    id: String,
    started_at: u64,
}

#[derive(Serialize, Deserialize, Default)]
struct Stored {
    #[serde(default)]
    games: Vec<Game>,
}

pub struct Library {
    paths: Paths,
    install_root: PathBuf,
    download_root: PathBuf,
    games: Vec<Game>,
    running: Vec<Running>,
}

impl Library {
    pub fn boot() -> anyhow::Result<Self> {
        let paths = config::resolve_paths();
        config::ensure_dirs(&paths)?;
        let cfg = config::load_config(&paths);
        let install_root = if cfg.install_dir.trim().is_empty() {
            PathBuf::from(&cfg.download_dir)
        } else {
            PathBuf::from(cfg.install_dir.trim())
        };
        let download_root = PathBuf::from(&cfg.download_dir);
        let games = load_games(&paths);
        Ok(Self {
            paths,
            install_root,
            download_root,
            games,
            running: Vec::new(),
        })
    }

    fn save(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.paths.data_dir)?;
        let file = self.paths.data_dir.join(LIBRARY_FILE);
        let tmp = file.with_extension("json.tmp");
        std::fs::write(
            &tmp,
            serde_json::to_string_pretty(&Stored {
                games: self.games.clone(),
            })?,
        )?;
        replace_file(&tmp, &file)?;
        Ok(())
    }

    fn cover_file(&self, id: &str) -> PathBuf {
        self.paths
            .data_dir
            .join(COVERS_DIR)
            .join(format!("{id}.jpg"))
    }

    pub fn views(&self) -> Vec<GameView> {
        self.games
            .iter()
            .map(|g| {
                let running = self.running.iter().any(|r| r.id == g.id);
                let mut playtime = g.playtime_secs;
                if let Some(r) = self.running.iter().find(|r| r.id == g.id) {
                    playtime += now_secs().saturating_sub(r.started_at);
                }
                GameView {
                    id: g.id.clone(),
                    title: g.title.clone(),
                    dir: g.dir.clone(),
                    exe: g.exe.clone(),
                    cover: cover_data_url(&self.cover_file(&g.id)),
                    steam_app_id: g.steam_app_id,
                    source: g.source.clone(),
                    added_at: g.added_at,
                    last_played_at: g.last_played_at,
                    playtime_secs: playtime,
                    running,
                }
            })
            .collect()
    }

    pub fn view(&self, id: &str) -> Option<GameView> {
        self.views().into_iter().find(|g| g.id == id)
    }

    pub fn add(
        &mut self,
        title: String,
        dir: &Path,
        exe: Option<String>,
        source: Option<String>,
    ) -> anyhow::Result<Game> {
        let dir_str = dir.to_string_lossy().into_owned();
        let id = game_id(&dir_str);
        if let Some(existing) = self.games.iter_mut().find(|g| g.id == id) {
            existing.title = title;
            if exe.is_some() {
                existing.exe = exe;
            }
            let clone = existing.clone();
            self.save()?;
            return Ok(clone);
        }
        let game = Game {
            id,
            title,
            dir: dir_str,
            exe: exe.or_else(|| detect_exe(dir).map(|p| p.to_string_lossy().into_owned())),
            steam_app_id: None,
            source,
            added_at: now_secs(),
            last_played_at: None,
            playtime_secs: 0,
        };
        self.games.push(game.clone());
        self.save()?;
        Ok(game)
    }

    pub fn set_exe(&mut self, id: &str, exe: &str) -> anyhow::Result<()> {
        let game = self
            .games
            .iter_mut()
            .find(|g| g.id == id)
            .ok_or_else(|| anyhow::anyhow!("game not found"))?;
        game.exe = Some(exe.to_string());
        self.save()
    }

    pub fn remove(&mut self, id: &str, delete_files: bool) -> anyhow::Result<()> {
        let pos = self
            .games
            .iter()
            .position(|g| g.id == id)
            .ok_or_else(|| anyhow::anyhow!("game not found"))?;
        let game = self.games.remove(pos);
        self.running.retain(|r| r.id != id);
        if delete_files {
            let dir = PathBuf::from(&game.dir);
            validate_target(&dir)?;
            anyhow::ensure!(dir != self.paths.data_dir, "refusing to delete app data");
            if dir.is_dir() {
                std::fs::remove_dir_all(&dir)?;
            }
        }
        self.save()?;
        let _ = std::fs::remove_file(self.cover_file(id));
        Ok(())
    }

    /// Spawns the game executable and returns the child so the caller can watch it.
    pub fn launch(&mut self, id: &str) -> anyhow::Result<Child> {
        let (dir, exe_override, title) = {
            let game = self
                .games
                .iter()
                .find(|g| g.id == id)
                .ok_or_else(|| anyhow::anyhow!("game not found"))?;
            (
                PathBuf::from(&game.dir),
                game.exe.clone(),
                game.title.clone(),
            )
        };

        let exe = match exe_override.map(PathBuf::from).filter(|p| p.is_file()) {
            Some(p) => p,
            None => detect_exe(&dir)
                .ok_or_else(|| anyhow::anyhow!("no game executable found in {}", dir.display()))?,
        };

        let mut cmd = Command::new(&exe);
        cmd.current_dir(&dir);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        let child = cmd
            .spawn()
            .map_err(|e| anyhow::anyhow!("failed to start {}: {e}", exe.display()))?;

        let exe_str = exe.to_string_lossy().into_owned();
        if let Some(game) = self.games.iter_mut().find(|g| g.id == id) {
            game.exe = Some(exe_str);
            game.last_played_at = Some(now_secs());
        }
        self.running.push(Running {
            id: id.to_string(),
            started_at: now_secs(),
        });
        self.save()?;
        tracing::info!(title = %title, exe = %exe.display(), "launched game");
        Ok(child)
    }

    pub fn finish_play(&mut self, id: &str) {
        let Some(pos) = self.running.iter().position(|r| r.id == id) else {
            return;
        };
        let run = self.running.remove(pos);
        let now = now_secs();
        let elapsed = now.saturating_sub(run.started_at);
        if let Some(game) = self.games.iter_mut().find(|g| g.id == id) {
            game.playtime_secs += elapsed;
            game.last_played_at = Some(now);
        }
        if let Err(e) = self.save() {
            tracing::warn!("failed to persist playtime: {e:#}");
        }
    }

    pub fn set_steam_app_id(&mut self, id: &str, app_id: u32) {
        if let Some(game) = self.games.iter_mut().find(|g| g.id == id) {
            game.steam_app_id = Some(app_id);
            if let Err(e) = self.save() {
                tracing::warn!("failed to persist steam id: {e:#}");
            }
        }
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    pub fn install_root(&self) -> &Path {
        &self.install_root
    }

    /// Installers only ever run from folders this app created: a download folder or an
    /// install target. Pointing the command at `C:\` must not scan or launch anything there.
    pub fn install_source_allowed(&self, dir: &Path) -> bool {
        inside(dir, &self.download_root) || inside(dir, &self.install_root)
    }
}

fn inside(child: &Path, root: &Path) -> bool {
    let (Ok(child), Ok(root)) = (child.canonicalize(), root.canonicalize()) else {
        return false;
    };
    child.starts_with(root)
}

fn load_games(paths: &Paths) -> Vec<Game> {
    let file = paths.data_dir.join(LIBRARY_FILE);
    std::fs::read_to_string(file)
        .ok()
        .and_then(|raw| serde_json::from_str::<Stored>(&raw).ok())
        .map(|s| s.games)
        .unwrap_or_default()
}

fn game_id(dir: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(dir.to_lowercase().as_bytes());
    let hex = format!("{:x}", hasher.finalize());
    hex[..16].to_string()
}

fn cover_data_url(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.is_empty() {
        return None;
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:image/jpeg;base64,{encoded}"))
}

fn validate_existing_dir(dir: &str) -> anyhow::Result<PathBuf> {
    let path = PathBuf::from(dir);
    anyhow::ensure!(path.is_absolute(), "path must be absolute");
    anyhow::ensure!(path.is_dir(), "folder not found: {dir}");
    Ok(path)
}

/// Rejects drive roots and `..` segments: used before creating or deleting directories.
fn validate_target(target: &Path) -> anyhow::Result<()> {
    anyhow::ensure!(target.is_absolute(), "path must be absolute");
    anyhow::ensure!(
        !target
            .components()
            .any(|c| matches!(c, Component::ParentDir)),
        "path must not contain .."
    );
    let depth = target
        .components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .count();
    anyhow::ensure!(depth >= 1, "refusing to touch a drive root");
    Ok(())
}

pub fn sanitize_dir_name(title: &str) -> String {
    let mut out: String = title
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => ' ',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    while out.contains("  ") {
        out = out.replace("  ", " ");
    }
    out.trim().trim_matches('.').to_string()
}

/// Folder names on disk carry site and repack noise: "Call of Duty - MW4 [FitGirl Repack]",
/// "GORN-2-SteamRIP.com". Strip it so the card and the Steam lookup see a plain title.
pub fn clean_title(raw: &str) -> String {
    let mut text = raw.to_string();
    while let (Some(start), Some(end)) = (text.find('['), text.find(']')) {
        if end > start {
            text.replace_range(start..=end, " ");
        } else {
            break;
        }
    }
    let mut text = text;
    for noise in [
        "SteamRIP.com",
        "SteamRIP",
        "FitGirl Repack",
        "FitGirl",
        "Online-Fix",
        "OnlineFix",
        "Repack",
        "GOG",
        "RIP",
    ] {
        text = text.replace(noise, " ");
    }
    if let (Some(start), Some(end)) = (text.find('('), text.find(')')) {
        if end > start {
            let inner = text[start + 1..end].to_lowercase();
            if inner.contains("repack") || inner.contains("rip") || inner.contains("fitgirl") {
                text.replace_range(start..=end, " ");
            }
        }
    }
    let text = text
        .replace(['.', '_'], " ")
        .replace(" - ", "\u{1}")
        .replace('-', " ")
        .replace('\u{1}', " - ");
    let mut out = String::with_capacity(text.len());
    let mut prev_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !prev_space && !out.is_empty() {
                out.push(' ');
                prev_space = true;
            }
            continue;
        }
        prev_space = false;
        out.push(ch);
    }
    out.trim().trim_matches('-').trim().to_string()
}

fn normalize_words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 1)
        .filter(|w| {
            !matches!(
                *w,
                "the"
                    | "and"
                    | "for"
                    | "repack"
                    | "fitgirl"
                    | "steamrip"
                    | "edition"
                    | "game"
                    | "pc"
                    | "rus"
                    | "eng"
                    | "multi"
            )
        })
        .map(|w| w.to_string())
        .collect()
}

fn collect_files(dir: &Path, depth: u8, out: &mut Vec<PathBuf>) {
    if depth > SCAN_DEPTH || out.len() >= SCAN_LIMIT {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= SCAN_LIMIT {
            return;
        }
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if name.contains("redist") || name == "__macosx" {
                continue;
            }
            collect_files(&path, depth + 1, out);
        } else if path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("exe"))
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
}

/// Picks the game's own executable, skipping installers, redistributables and crash handlers.
pub fn detect_exe(dir: &Path) -> Option<PathBuf> {
    let mut files = Vec::new();
    collect_files(dir, 0, &mut files);
    let folder_words = normalize_words(
        &dir.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );

    let mut best: Option<(i32, u64, PathBuf)> = None;
    for file in files {
        let stem = file
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if is_helper_exe(&stem) {
            continue;
        }
        let words = normalize_words(&stem);
        let mut score = 0i32;
        if words.iter().any(|w| folder_words.contains(w)) {
            score += 10;
        }
        if words.len() == folder_words.len() && words == folder_words {
            score += 10;
        }
        if stem.contains("launcher") {
            score -= 3;
        }
        let size = file.metadata().map(|m| m.len()).unwrap_or(0);
        let better = match &best {
            None => true,
            Some((best_score, best_size, _)) => {
                score > *best_score || (score == *best_score && size > *best_size)
            }
        };
        if better {
            best = Some((score, size, file));
        }
    }
    best.map(|(_, _, path)| path)
}

/// Only files whose name says "setup/install" count as repack installers; a lone game exe
/// in a download folder must never be launched as if it were one.
pub fn looks_like_setup(path: &Path) -> bool {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    (stem.contains("setup") || stem.contains("install")) && !stem.contains("unins")
}

pub fn find_installer(dir: &Path) -> Option<PathBuf> {
    let mut files = Vec::new();
    collect_files(dir, 0, &mut files);
    let mut best: Option<(u64, PathBuf)> = None;
    for file in files {
        if !looks_like_setup(&file) {
            continue;
        }
        let size = file.metadata().map(|m| m.len()).unwrap_or(0);
        if best.as_ref().map(|(s, _)| size > *s).unwrap_or(true) {
            best = Some((size, file));
        }
    }
    best.map(|(_, p)| p)
}

fn read_head_tail(path: &Path, window: u64) -> Vec<u8> {
    let mut out = Vec::new();
    let Ok(mut file) = std::fs::File::open(path) else {
        return out;
    };
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    let mut head = vec![0u8; window as usize];
    if let Ok(read) = file.read(&mut head) {
        out.extend_from_slice(&head[..read]);
    }
    if len > window {
        if file.seek(SeekFrom::Start(len - window)).is_ok() {
            let mut tail = vec![0u8; window as usize];
            if let Ok(read) = file.read(&mut tail) {
                out.extend_from_slice(&tail[..read]);
            }
        }
    }
    out
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

pub fn installer_kind(exe: &Path) -> InstallerKind {
    let bytes = read_head_tail(exe, 1024 * 1024);
    if contains(&bytes, b"Inno Setup") {
        InstallerKind::Inno
    } else if contains(&bytes, b"Nullsoft Install System") || contains(&bytes, b"NullsoftInst") {
        InstallerKind::Nsis
    } else {
        InstallerKind::Unknown
    }
}

fn silent_args(kind: &InstallerKind, target: &Path) -> Vec<String> {
    let dir = target.to_string_lossy().into_owned();
    match kind {
        InstallerKind::Inno => vec![
            "/VERYSILENT".into(),
            "/SUPPRESSMSGBOXES".into(),
            "/NORESTART".into(),
            "/SP-".into(),
            format!("/DIR={dir}"),
        ],
        InstallerKind::Nsis => vec!["/S".into(), format!("/D={dir}")],
        InstallerKind::Unknown => vec![],
    }
}

pub fn probe_folder(dir: &Path) -> FolderProbe {
    let installer = find_installer(dir);
    let silent = installer
        .as_ref()
        .map(|exe| installer_kind(exe) != InstallerKind::Unknown)
        .unwrap_or(false);
    FolderProbe {
        installer: installer.map(|p| p.to_string_lossy().into_owned()),
        exe: detect_exe(dir).map(|p| p.to_string_lossy().into_owned()),
        silent,
    }
}

#[derive(Deserialize)]
struct SteamSearch {
    #[serde(default)]
    items: Vec<SteamItem>,
}

#[derive(Deserialize)]
struct SteamItem {
    id: u32,
    #[serde(default)]
    name: String,
}

fn best_match<'a>(items: &'a [SteamItem], title: &str) -> Option<&'a SteamItem> {
    let want = normalize_words(title);
    let mut best: Option<(usize, &SteamItem)> = None;
    for item in items {
        let have = normalize_words(&item.name);
        let score = want.iter().filter(|w| have.contains(w)).count();
        if best.map(|(s, _)| score > s).unwrap_or(true) {
            best = Some((score, item));
        }
    }
    best.filter(|(score, _)| *score > 0)
        .map(|(_, item)| item)
        .or_else(|| items.first())
}

/// Finds the Steam app id and caches its portrait cover. Network failures are not fatal.
async fn fetch_cover(paths: &Paths, id: &str, title: &str) -> anyhow::Result<Option<u32>> {
    let client = reqwest::Client::builder()
        .user_agent("kovcheg")
        .timeout(Duration::from_secs(20))
        .build()?;
    let url = format!(
        "{STEAM_SEARCH}?term={}&l=english&cc=US",
        urlencoding::encode(title)
    );
    let search: SteamSearch = client.get(url).send().await?.json().await?;
    let Some(item) = best_match(&search.items, title) else {
        return Ok(None);
    };
    let app_id = item.id;

    let covers = paths.data_dir.join(COVERS_DIR);
    std::fs::create_dir_all(&covers)?;
    let dest = covers.join(format!("{id}.jpg"));

    for name in ["library_600x900.jpg", "header.jpg"] {
        let url = format!("{STEAM_CDN}/{app_id}/{name}");
        let resp = client.get(url).send().await?;
        if !resp.status().is_success() {
            continue;
        }
        let bytes = resp.bytes().await?;
        if bytes.len() > 512 {
            std::fs::write(&dest, &bytes)?;
            return Ok(Some(app_id));
        }
    }
    Ok(None)
}

async fn broadcast(app: &AppHandle, lib: &Arc<Mutex<Library>>) {
    let views = lib.lock().await.views();
    let _ = app.emit("library-changed", views);
}

fn emit_progress(app: &AppHandle, title: &str, phase: &'static str, message: String) {
    let _ = app.emit(
        "library-install",
        InstallProgress {
            title: title.to_string(),
            phase,
            message,
        },
    );
}

#[tauri::command]
pub async fn library_list(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<GameView>, String> {
    Ok(state.library.lock().await.views())
}

#[tauri::command]
pub async fn library_probe(dir: String) -> Result<FolderProbe, String> {
    let path = validate_existing_dir(&dir).map_err(|e| e.to_string())?;
    Ok(probe_folder(&path))
}

#[tauri::command]
pub async fn library_open_folder(
    state: tauri::State<'_, crate::AppState>,
    id: String,
) -> Result<(), String> {
    let dir = state
        .library
        .lock()
        .await
        .view(&id)
        .map(|game| game.dir)
        .ok_or_else(|| "game not found".to_string())?;
    crate::fs::reveal(&dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn library_add(
    app: AppHandle,
    state: tauri::State<'_, crate::AppState>,
    title: String,
    dir: String,
    exe: Option<String>,
    source: Option<String>,
) -> Result<GameView, String> {
    let path = validate_existing_dir(&dir).map_err(|e| e.to_string())?;
    let lib = state.library.clone();
    let title = if title.trim().is_empty() {
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Game".into())
    } else {
        title.trim().to_string()
    };

    let (game_id, paths) = {
        let mut guard = lib.lock().await;
        let game = guard
            .add(title.clone(), &path, exe, source)
            .map_err(|e| e.to_string())?;
        (game.id.clone(), guard.paths().clone())
    };

    spawn_cover_lookup(app.clone(), lib.clone(), paths, game_id.clone(), title);
    broadcast(&app, &lib).await;

    let guard = lib.lock().await;
    guard
        .view(&game_id)
        .ok_or_else(|| "game not found".to_string())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub added: Vec<GameView>,
    pub skipped: Vec<String>,
}

/// Imports every direct subfolder that looks like an installed game.
/// Folders without an executable (a repack still sitting as `fg-*.bin`) are reported, not added.
#[tauri::command]
pub async fn library_import(
    app: AppHandle,
    state: tauri::State<'_, crate::AppState>,
    dir: String,
) -> Result<ImportReport, String> {
    let root = validate_existing_dir(&dir).map_err(|e| e.to_string())?;
    let lib = state.library.clone();

    let mut report = ImportReport {
        added: Vec::new(),
        skipped: Vec::new(),
    };
    let mut candidates: Vec<(String, PathBuf, PathBuf)> = Vec::new();
    let entries = std::fs::read_dir(&root).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let folder = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        match detect_exe(&path) {
            Some(exe) => {
                let title = clean_title(&folder);
                candidates.push((if title.is_empty() { folder } else { title }, path, exe));
            }
            None => report.skipped.push(folder),
        }
    }
    candidates.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

    let mut added_ids: Vec<(String, String)> = Vec::new();
    let paths;
    {
        let mut guard = lib.lock().await;
        paths = guard.paths().clone();
        for (title, path, exe) in candidates {
            if guard.games.iter().any(|g| same_dir(&g.dir, &path)) {
                continue;
            }
            let exe = Some(exe.to_string_lossy().into_owned());
            match guard.add(title, &path, exe, Some("import".into())) {
                Ok(game) => added_ids.push((game.id, game.title)),
                Err(e) => report.skipped.push(e.to_string()),
            }
        }
        let views = guard.views();
        report.added = views
            .into_iter()
            .filter(|v| added_ids.iter().any(|(id, _)| *id == v.id))
            .collect();
    }

    for (id, title) in added_ids {
        spawn_cover_lookup(app.clone(), lib.clone(), paths.clone(), id, title);
    }
    broadcast(&app, &lib).await;
    Ok(report)
}

fn same_dir(a: &str, b: &Path) -> bool {
    let (Ok(a), Ok(b)) = (Path::new(a).canonicalize(), b.canonicalize()) else {
        return a.eq_ignore_ascii_case(&b.to_string_lossy());
    };
    a == b
}

#[tauri::command]
pub async fn library_set_exe(
    app: AppHandle,
    state: tauri::State<'_, crate::AppState>,
    id: String,
    exe: String,
) -> Result<(), String> {
    let lib = state.library.clone();
    lib.lock()
        .await
        .set_exe(&id, &exe)
        .map_err(|e| e.to_string())?;
    broadcast(&app, &lib).await;
    Ok(())
}

#[tauri::command]
pub async fn library_remove(
    app: AppHandle,
    state: tauri::State<'_, crate::AppState>,
    id: String,
    delete_files: bool,
) -> Result<(), String> {
    let lib = state.library.clone();
    lib.lock()
        .await
        .remove(&id, delete_files)
        .map_err(|e| e.to_string())?;
    broadcast(&app, &lib).await;
    Ok(())
}

#[tauri::command]
pub async fn library_launch(
    app: AppHandle,
    state: tauri::State<'_, crate::AppState>,
    id: String,
) -> Result<(), String> {
    let lib = state.library.clone();
    let child = lib.lock().await.launch(&id).map_err(|e| e.to_string())?;
    broadcast(&app, &lib).await;

    let watcher_app = app.clone();
    let watcher_lib = lib.clone();
    let watched_id = id.clone();
    tauri::async_runtime::spawn(async move {
        let mut child = child;
        loop {
            tokio::time::sleep(Duration::from_secs(3)).await;
            match child.try_wait() {
                Ok(Some(_)) | Err(_) => break,
                Ok(None) => {}
            }
        }
        watcher_lib.lock().await.finish_play(&watched_id);
        broadcast(&watcher_app, &watcher_lib).await;
    });
    Ok(())
}

#[tauri::command]
pub async fn library_refresh_cover(
    app: AppHandle,
    state: tauri::State<'_, crate::AppState>,
    id: String,
) -> Result<Option<u32>, String> {
    let lib = state.library.clone();
    let (paths, title) = {
        let guard = lib.lock().await;
        let game = guard
            .view(&id)
            .ok_or_else(|| "game not found".to_string())?;
        (guard.paths().clone(), game.title)
    };
    let app_id = fetch_cover(&paths, &id, &title)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(app_id) = app_id {
        lib.lock().await.set_steam_app_id(&id, app_id);
    }
    broadcast(&app, &lib).await;
    Ok(app_id)
}

#[tauri::command]
pub async fn library_install(
    app: AppHandle,
    state: tauri::State<'_, crate::AppState>,
    dir: String,
    title: String,
    install_dir: Option<String>,
) -> Result<String, String> {
    let source = validate_existing_dir(&dir).map_err(|e| e.to_string())?;
    let lib = state.library.clone();
    if !lib.lock().await.install_source_allowed(&source) {
        return Err("installing is only allowed from the download or install folder".into());
    }

    let installer =
        find_installer(&source).ok_or_else(|| "no installer found in this folder".to_string())?;
    if !looks_like_setup(&installer) {
        return Err("no setup executable found in this folder".into());
    }
    let kind = installer_kind(&installer);
    if kind == InstallerKind::Unknown {
        // No documented silent switches: hand the installer to the user instead of guessing.
        let mut cmd = Command::new(&installer);
        cmd.current_dir(&source);
        cmd.spawn().map_err(|e| e.to_string())?;
        emit_progress(
            &app,
            &title,
            "manual",
            "Installer launched — finish it, then add the folder".into(),
        );
        return Err(
            "This installer has no known silent mode. It was launched for you: finish it, then add the install folder.".into(),
        );
    }

    let root = match install_dir
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty())
    {
        Some(dir) => PathBuf::from(dir),
        None => lib.lock().await.install_root().to_path_buf(),
    };
    let folder = sanitize_dir_name(&title);
    let folder = if folder.is_empty() {
        "Game".to_string()
    } else {
        folder
    };
    let target = root.join(&folder);
    validate_target(&target).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;

    let args = silent_args(&kind, &target);
    emit_progress(
        &app,
        &title,
        "running",
        format!("Installing to {}", target.display()),
    );

    let installer_clone = installer.clone();
    let source_clone = source.clone();
    let status = tauri::async_runtime::spawn_blocking(move || {
        let mut cmd = Command::new(&installer_clone);
        cmd.current_dir(&source_clone).args(&args);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        cmd.status()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    if !status.success() {
        emit_progress(
            &app,
            &title,
            "failed",
            "Installer exited with an error".into(),
        );
        return Err(format!("installer exited with {status}"));
    }

    let mut guard = lib.lock().await;
    let exe = detect_exe(&target);
    if exe.is_none() {
        emit_progress(
            &app,
            &title,
            "failed",
            "No game executable found after install".into(),
        );
        return Err(format!(
            "Nothing executable appeared in {}. Install finished; add the folder manually.",
            target.display()
        ));
    }
    let game = guard
        .add(
            title.clone(),
            &target,
            exe.map(|p| p.to_string_lossy().into_owned()),
            Some("repack".into()),
        )
        .map_err(|e| e.to_string())?;
    let id = game.id.clone();
    let paths = guard.paths().clone();
    drop(guard);

    emit_progress(&app, &title, "done", "Installed".into());
    spawn_cover_lookup(app.clone(), lib.clone(), paths, id.clone(), title);
    broadcast(&app, &lib).await;
    Ok(id)
}

fn spawn_cover_lookup(
    app: AppHandle,
    lib: Arc<Mutex<Library>>,
    paths: Paths,
    id: String,
    title: String,
) {
    tauri::async_runtime::spawn(async move {
        match fetch_cover(&paths, &id, &title).await {
            Ok(Some(app_id)) => {
                lib.lock().await.set_steam_app_id(&id, app_id);
                broadcast(&app, &lib).await;
            }
            Ok(None) => tracing::info!(title = %title, "no steam match for cover"),
            Err(e) => tracing::warn!(title = %title, "cover lookup failed: {e:#}"),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kovcheg-lib-{name}-{}-{}",
            std::process::id(),
            now_secs()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn detect_exe_prefers_game_over_installer() {
        let dir = temp_dir("detect");
        let game = dir.join("My Game");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("unins000.exe"), vec![0u8; 2048]).unwrap();
        std::fs::write(game.join("setup.exe"), vec![0u8; 4096]).unwrap();
        std::fs::write(game.join("MyGame.exe"), vec![0u8; 1024]).unwrap();

        let found = detect_exe(&game).unwrap();
        assert_eq!(found.file_name().unwrap(), "MyGame.exe");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_exe_keeps_names_that_merely_contain_helper_words() {
        let dir = temp_dir("detect-crash");
        let game = dir.join("Crashout Crew");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("CrashoutCrew.exe"), vec![0u8; 1024]).unwrap();
        std::fs::write(game.join("UnityCrashHandler64.exe"), vec![0u8; 512]).unwrap();
        std::fs::write(game.join("CrashReporter.exe"), vec![0u8; 512]).unwrap();

        let found = detect_exe(&game).unwrap();
        assert_eq!(found.file_name().unwrap(), "CrashoutCrew.exe");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn installer_kind_reads_markers() {
        let dir = temp_dir("kind");
        let inno = dir.join("setup.exe");
        std::fs::write(&inno, b"MZ....Inno Setup Setup Data (6.3.0)....").unwrap();
        assert_eq!(installer_kind(&inno), InstallerKind::Inno);

        let nsis = dir.join("install.exe");
        std::fs::write(&nsis, b"MZ....Nullsoft Install System v3.08....").unwrap();
        assert_eq!(installer_kind(&nsis), InstallerKind::Nsis);

        let other = dir.join("weird.exe");
        std::fs::write(&other, b"MZ....nothing here....").unwrap();
        assert_eq!(installer_kind(&other), InstallerKind::Unknown);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_installer_skips_uninstaller() {
        let dir = temp_dir("installer");
        std::fs::write(dir.join("unins000.exe"), vec![0u8; 9000]).unwrap();
        std::fs::write(dir.join("setup-fitgirl.exe"), vec![0u8; 100]).unwrap();
        let found = find_installer(&dir).unwrap();
        assert_eq!(found.file_name().unwrap(), "setup-fitgirl.exe");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn silent_args_follow_installer_flavour() {
        let target = PathBuf::from("D:/Games/Test Game");
        let inno = silent_args(&InstallerKind::Inno, &target);
        assert!(inno.iter().any(|a| a == "/VERYSILENT"));
        assert!(inno.iter().any(|a| a == "/DIR=D:/Games/Test Game"));
        let nsis = silent_args(&InstallerKind::Nsis, &target);
        assert_eq!(nsis.last().unwrap(), "/D=D:/Games/Test Game");
        assert!(silent_args(&InstallerKind::Unknown, &target).is_empty());
    }

    #[test]
    fn validate_target_rejects_roots_and_parent_segments() {
        assert!(validate_target(Path::new("/")).is_err());
        assert!(validate_target(Path::new("relative/path")).is_err());

        let dir = temp_dir("target");
        assert!(validate_target(&dir.join("Games")).is_ok());
        assert!(validate_target(&dir.join("Games").join("..").join("Windows")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sanitize_strips_reserved_characters() {
        assert_eq!(sanitize_dir_name("A/B: C*?"), "A B C");
        assert_eq!(sanitize_dir_name("  ..  "), "");
    }

    #[test]
    fn best_match_ignores_repack_noise() {
        let items = vec![
            SteamItem {
                id: 1,
                name: "Unrelated Tool".into(),
            },
            SteamItem {
                id: 2,
                name: "Cyberpunk 2077".into(),
            },
        ];
        let picked = best_match(&items, "Cyberpunk 2077 [FitGirl Repack]").unwrap();
        assert_eq!(picked.id, 2);
    }

    #[test]
    fn library_round_trips_and_counts_playtime() {
        let dir = temp_dir("store");
        let paths = Paths {
            config_file: dir.join("config.json"),
            queue_file: dir.join("queue.json"),
            history_file: dir.join("history.json"),
            seeds_file: dir.join("seeds.json"),
            torrents_dir: dir.join("torrents"),
            data_dir: dir.clone(),
        };
        let game_dir = dir.join("Some Game");

        let mut lib = Library {
            paths: paths.clone(),
            install_root: dir.clone(),
            download_root: dir.clone(),
            games: Vec::new(),
            running: Vec::new(),
        };
        let game = lib.add("Some Game".into(), &game_dir, None, None).unwrap();
        lib.running.push(Running {
            id: game.id.clone(),
            started_at: now_secs().saturating_sub(30),
        });
        lib.finish_play(&game.id);

        let reloaded = Library {
            paths: paths.clone(),
            install_root: dir.clone(),
            download_root: dir.clone(),
            games: load_games(&paths),
            running: Vec::new(),
        };
        let view = reloaded.view(&game.id).unwrap();
        assert!(view.playtime_secs >= 30);
        assert!(view.last_played_at.is_some());
        assert!(!view.running);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn install_is_refused_outside_download_and_install_roots() {
        let dir = temp_dir("scope");
        let paths = Paths {
            config_file: dir.join("config.json"),
            queue_file: dir.join("queue.json"),
            history_file: dir.join("history.json"),
            seeds_file: dir.join("seeds.json"),
            torrents_dir: dir.join("torrents"),
            data_dir: dir.clone(),
        };
        let downloads = dir.join("downloads");
        let installs = dir.join("installs");
        std::fs::create_dir_all(downloads.join("Some Repack")).unwrap();
        std::fs::create_dir_all(installs.join("Some Game")).unwrap();
        std::fs::create_dir_all(dir.join("elsewhere")).unwrap();

        let lib = Library {
            paths,
            install_root: installs.clone(),
            download_root: downloads.clone(),
            games: Vec::new(),
            running: Vec::new(),
        };

        assert!(lib.install_source_allowed(&downloads.join("Some Repack")));
        assert!(lib.install_source_allowed(&installs.join("Some Game")));
        assert!(!lib.install_source_allowed(&dir.join("elsewhere")));
        assert!(!lib.install_source_allowed(Path::new("C:/")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_setup_like_files_count_as_installers() {
        assert!(looks_like_setup(Path::new("C:/dl/GAME/setup.exe")));
        assert!(looks_like_setup(Path::new("C:/dl/GAME/setup-fitgirl.exe")));
        assert!(looks_like_setup(Path::new("C:/dl/GAME/INSTALL.EXE")));
        assert!(!looks_like_setup(Path::new("C:/dl/GAME/Game.exe")));
        assert!(!looks_like_setup(Path::new("C:/dl/GAME/unins000.exe")));
        assert!(!looks_like_setup(Path::new("C:/dl/GAME/vcredist_x64.exe")));
    }

    #[test]
    fn clean_title_drops_site_and_repack_noise() {
        assert_eq!(
            clean_title("Call of Duty - MW4 [FitGirl Repack]"),
            "Call of Duty - MW4"
        );
        assert_eq!(clean_title("GORN-2-SteamRIP.com"), "GORN 2");
        assert_eq!(clean_title("I-Am-Cat-SteamRIP.com"), "I Am Cat");
        assert_eq!(clean_title("Game.Name.v1.2-OFME"), "Game Name v1 2 OFME");
        assert_eq!(clean_title("Plain Game"), "Plain Game");
    }

    #[test]
    fn add_is_idempotent_for_same_directory() {
        let dir = temp_dir("dedupe");
        let paths = Paths {
            config_file: dir.join("config.json"),
            queue_file: dir.join("queue.json"),
            history_file: dir.join("history.json"),
            seeds_file: dir.join("seeds.json"),
            torrents_dir: dir.join("torrents"),
            data_dir: dir.clone(),
        };
        let mut lib = Library {
            paths,
            install_root: dir.clone(),
            download_root: dir.clone(),
            games: Vec::new(),
            running: Vec::new(),
        };
        let game_dir = dir.join("Game");
        let first = lib.add("Game".into(), &game_dir, None, None).unwrap();
        let second = lib
            .add("Game renamed".into(), &game_dir, None, None)
            .unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(lib.games.len(), 1);
        assert_eq!(lib.games[0].title, "Game renamed");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
