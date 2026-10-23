use super::*;

/// An installer that is still running after this long is waiting for a prompt, not working.
pub(crate) const INSTALL_TIMEOUT_SECS: u64 = 45 * 60;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallerKind {
    Inno,
    Nsis,
    Unknown,
}

pub(crate) fn validate_existing_dir(dir: &str) -> anyhow::Result<PathBuf> {
    let path = PathBuf::from(dir);
    anyhow::ensure!(path.is_absolute(), "path must be absolute");
    anyhow::ensure!(path.is_dir(), "folder not found: {dir}");
    Ok(path)
}

/// Rejects drive roots, `..` segments and relative paths: used before creating or deleting directories.
pub(crate) fn validate_target(target: &Path) -> anyhow::Result<()> {
    anyhow::ensure!(target.is_absolute(), "path must be absolute");
    crate::path_guard::reject_parent_components(&target.to_string_lossy())
        .map_err(anyhow::Error::msg)?;
    let depth = target
        .components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .count();
    anyhow::ensure!(depth >= 1, "refusing to touch a drive root");
    Ok(())
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

pub(crate) fn read_head_tail(path: &Path, window: u64) -> Vec<u8> {
    let mut out = Vec::new();
    let Ok(mut file) = std::fs::File::open(path) else {
        return out;
    };
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    let mut head = vec![0u8; window as usize];
    if let Ok(read) = file.read(&mut head) {
        out.extend_from_slice(&head[..read]);
    }
    if len > window && file.seek(SeekFrom::Start(len - window)).is_ok() {
        let mut tail = vec![0u8; window as usize];
        if let Ok(read) = file.read(&mut tail) {
            out.extend_from_slice(&tail[..read]);
        }
    }
    out
}

pub(crate) fn contains(haystack: &[u8], needle: &[u8]) -> bool {
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

pub(crate) fn silent_args(kind: &InstallerKind, target: &Path) -> Vec<String> {
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

pub(crate) fn same_dir(a: &str, b: &Path) -> bool {
    let (Ok(a), Ok(b)) = (Path::new(a).canonicalize(), b.canonicalize()) else {
        return a.eq_ignore_ascii_case(&b.to_string_lossy());
    };
    a == b
}

/// Everything about an install that can be decided before a process is spawned.
/// Deliberately pure: the risky part (running an arbitrary installer) starts only after this
/// is built and checked, and this is what the tests can exercise.
pub(crate) struct InstallPlan {
    installer: PathBuf,
    target: PathBuf,
    args: Vec<String>,
}

impl InstallPlan {
    pub(crate) fn new(
        installer: PathBuf,
        kind: InstallerKind,
        title: &str,
        root: &Path,
    ) -> anyhow::Result<Self> {
        let folder = sanitize_dir_name(title);
        let folder = if folder.is_empty() {
            "Game".to_string()
        } else {
            folder
        };
        let target = root.join(folder);
        validate_target(&target)?;
        Ok(Self {
            args: silent_args(&kind, &target),
            installer,
            target,
        })
    }

    pub(crate) fn target(&self) -> &Path {
        &self.target
    }
}

pub(crate) enum InstallError {
    /// The installer could not be started at all.
    Spawn(String),
    /// It ran and failed.
    Exited(String),
    /// It never returned: almost certainly waiting for a hidden prompt.
    Timeout,
}

/// Runs the planned installer silently with a hard deadline.
pub(crate) async fn run_installer(plan: &InstallPlan, source: &Path) -> Result<(), InstallError> {
    let (installer, args, source) = (
        plan.installer.clone(),
        plan.args.clone(),
        source.to_path_buf(),
    );
    let wait = tauri::async_runtime::spawn_blocking(move || {
        let mut cmd = Command::new(&installer);
        cmd.current_dir(&source).args(&args);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        cmd.status()
    });
    match tokio::time::timeout(Duration::from_secs(INSTALL_TIMEOUT_SECS), wait).await {
        Err(_) => Err(InstallError::Timeout),
        Ok(joined) => {
            let status = joined
                .map_err(|e| InstallError::Spawn(e.to_string()))?
                .map_err(|e| InstallError::Spawn(e.to_string()))?;
            if status.success() {
                Ok(())
            } else {
                Err(InstallError::Exited(status.to_string()))
            }
        }
    }
}

/// An installer with no documented silent switches is handed to the user, never guessed at.
pub(crate) fn launch_for_user(installer: &Path, source: &Path) -> Result<(), String> {
    let mut cmd = Command::new(installer);
    cmd.current_dir(source);
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Adds the freshly installed folder to the library and starts its cover lookup.
pub(crate) async fn register_installed(
    app: &AppHandle,
    lib: &Arc<Mutex<Library>>,
    target: &Path,
    title: &str,
) -> Result<String, String> {
    let added = {
        let mut guard = lib.lock().await;
        match detect_exe(target) {
            Some(exe) => {
                let game = guard
                    .add(
                        title.to_string(),
                        target,
                        Some(exe.to_string_lossy().into_owned()),
                        Some("repack".into()),
                    )
                    .map_err(|e| e.to_string())?;
                Some((game.id.clone(), guard.paths().clone()))
            }
            None => None,
        }
    };
    let Some((id, paths)) = added else {
        emit_progress(
            app,
            title,
            "failed",
            "No game executable found after install".into(),
        );
        return Err(format!(
            "Nothing executable appeared in {}. Install finished; add the folder manually.",
            target.display()
        ));
    };
    emit_progress(app, title, "done", "Installed".into());
    spawn_cover_refresh(
        app.clone(),
        lib.clone(),
        paths,
        vec![(id.clone(), title.to_string())],
    );
    broadcast(app, lib).await;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("kovcheg-plan-{name}"));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn plan_puts_the_game_in_a_sanitized_child_folder() {
        let root = temp_root("sanitize");
        let plan = InstallPlan::new(
            PathBuf::from("C:/x/setup.exe"),
            InstallerKind::Inno,
            "Game: Deluxe [FitGirl Repack]",
            &root,
        )
        .unwrap();
        assert_eq!(plan.target().parent(), Some(root.as_path()));
        let folder = plan
            .target()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert!(folder.starts_with("Game Deluxe"), "{folder}");
        assert!(!folder.contains(':'), "{folder}");
        assert!(plan.args.iter().any(|a| a.starts_with("/DIR=")));
    }

    #[test]
    fn plan_cannot_escape_the_install_root() {
        let root = temp_root("escape");
        let plan = InstallPlan::new(
            PathBuf::from("setup.exe"),
            InstallerKind::Nsis,
            "../../etc",
            &root,
        )
        .unwrap();
        assert_eq!(plan.target().parent(), Some(root.as_path()));
    }

    #[test]
    fn plan_falls_back_to_a_plain_name_for_an_empty_title() {
        let root = temp_root("empty");
        let plan = InstallPlan::new(
            PathBuf::from("setup.exe"),
            InstallerKind::Nsis,
            "  ..  ",
            &root,
        )
        .unwrap();
        assert_eq!(plan.target(), root.join("Game"));
    }
}
