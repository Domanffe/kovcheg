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

pub mod commands;
mod covers;
mod install;
mod scan;

pub(crate) use commands::*;
pub(crate) use covers::*;
pub(crate) use install::*;
pub(crate) use scan::*;

const LIBRARY_FILE: &str = "library.json";
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
pub struct InstallProgress {
    pub title: String,
    pub phase: &'static str,
    pub message: String,
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
        let game = self.games[pos].clone();
        if delete_files {
            let dir = PathBuf::from(&game.dir);
            validate_target(&dir)?;
            anyhow::ensure!(dir != self.paths.data_dir, "refusing to delete app data");
            // A folder that holds other games is a container added by mistake: deleting it
            // would take the whole collection with it.
            let canon = dir.canonicalize().unwrap_or_else(|_| dir.clone());
            let nested: Vec<String> = self
                .games
                .iter()
                .filter(|g| g.id != id)
                .filter(|g| {
                    Path::new(&g.dir)
                        .canonicalize()
                        .unwrap_or_else(|_| PathBuf::from(&g.dir))
                        .starts_with(&canon)
                })
                .map(|g| g.title.clone())
                .collect();
            anyhow::ensure!(
                nested.is_empty(),
                "this folder contains other games ({}); remove them first",
                nested.join(", ")
            );
            if dir.is_dir() {
                std::fs::remove_dir_all(&dir)?;
            }
        }
        self.games.remove(pos);
        self.running.retain(|r| r.id != id);
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
        crate::path_guard::is_inside(dir, &self.download_root)
            || crate::path_guard::is_inside(dir, &self.install_root)
    }
}

fn load_games(paths: &Paths) -> Vec<Game> {
    let file = paths.data_dir.join(LIBRARY_FILE);
    let Ok(raw) = std::fs::read_to_string(&file) else {
        return Vec::new();
    };
    match serde_json::from_str::<Stored>(&raw) {
        Ok(stored) => stored.games,
        Err(err) => {
            // Never let a truncated file turn into a silently empty library that the next
            // save overwrites: keep the evidence and start clean.
            let backup = file.with_extension(format!("json.corrupt-{}", now_secs()));
            tracing::error!(
                "library.json is unreadable ({err:#}); moved to {}",
                backup.display()
            );
            let _ = std::fs::rename(&file, &backup);
            Vec::new()
        }
    }
}

fn game_id(dir: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(dir.to_lowercase().as_bytes());
    let hex = format!("{:x}", hasher.finalize());
    hex[..16].to_string()
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
    fn corrupt_library_is_set_aside_instead_of_wiped() {
        let dir = temp_dir("corrupt");
        let paths = Paths {
            config_file: dir.join("config.json"),
            queue_file: dir.join("queue.json"),
            history_file: dir.join("history.json"),
            seeds_file: dir.join("seeds.json"),
            torrents_dir: dir.join("torrents"),
            data_dir: dir.clone(),
        };
        let file = dir.join("library.json");
        std::fs::write(&file, "{\"games\": [ truncated").unwrap();

        assert!(load_games(&paths).is_empty());
        assert!(!file.exists(), "corrupt library.json must be moved away");
        let kept = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().contains(".corrupt-"));
        assert!(kept, "expected a .corrupt- backup next to the data");
        let _ = std::fs::remove_dir_all(&dir);
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
    fn remove_refuses_a_folder_that_holds_other_games() {
        let dir = temp_dir("nested");
        let paths = Paths {
            config_file: dir.join("config.json"),
            queue_file: dir.join("queue.json"),
            history_file: dir.join("history.json"),
            seeds_file: dir.join("seeds.json"),
            torrents_dir: dir.join("torrents"),
            data_dir: dir.clone(),
        };
        let parent = dir.join("Games");
        let child = parent.join("Game A");
        std::fs::create_dir_all(&child).unwrap();
        let mut lib = Library {
            paths,
            install_root: dir.clone(),
            download_root: dir.clone(),
            games: Vec::new(),
            running: Vec::new(),
        };
        let container = lib.add("Games".into(), &parent, None, None).unwrap();
        lib.add("Game A".into(), &child, None, None).unwrap();

        let err = lib.remove(&container.id, true).unwrap_err().to_string();
        assert!(err.contains("contains other games"), "{err}");
        assert!(parent.exists(), "the container folder must survive");
        assert_eq!(lib.games.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn remove_deletes_only_the_game_folder() {
        let dir = temp_dir("leaf-remove");
        let paths = Paths {
            config_file: dir.join("config.json"),
            queue_file: dir.join("queue.json"),
            history_file: dir.join("history.json"),
            seeds_file: dir.join("seeds.json"),
            torrents_dir: dir.join("torrents"),
            data_dir: dir.join("state"),
        };
        let game_dir = dir.join("Game A");
        std::fs::create_dir_all(&game_dir).unwrap();
        std::fs::write(game_dir.join("game.exe"), b"x").unwrap();
        let mut lib = Library {
            paths,
            install_root: dir.clone(),
            download_root: dir.clone(),
            games: Vec::new(),
            running: Vec::new(),
        };
        let game = lib.add("Game A".into(), &game_dir, None, None).unwrap();

        lib.remove(&game.id, true).unwrap();
        assert!(!game_dir.exists(), "the game folder should be gone");
        assert!(dir.exists(), "the parent folder stays");
        assert!(lib.games.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
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
