use super::*;

pub(crate) async fn broadcast(app: &AppHandle, lib: &Arc<Mutex<Library>>) {
    let views = lib.lock().await.views();
    let _ = app.emit("library-changed", views);
}

pub(crate) fn emit_progress(app: &AppHandle, title: &str, phase: &'static str, message: String) {
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

    spawn_cover_refresh(
        app.clone(),
        lib.clone(),
        paths,
        vec![(game_id.clone(), title)],
    );
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
    candidates.sort_by_key(|c| c.0.to_lowercase());

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

    spawn_cover_refresh(app.clone(), lib.clone(), paths.clone(), added_ids);
    broadcast(&app, &lib).await;
    Ok(report)
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
        launch_for_user(&installer, &source)?;
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
    let plan = InstallPlan::new(installer, kind, &title, &root).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(plan.target()).map_err(|e| e.to_string())?;
    emit_progress(
        &app,
        &title,
        "running",
        format!("Installing to {}", plan.target().display()),
    );

    match run_installer(&plan, &source).await {
        Ok(()) => register_installed(&app, &lib, plan.target(), &title).await,
        Err(InstallError::Timeout) => {
            emit_progress(
                &app,
                &title,
                "failed",
                "The installer is still running after 45 minutes".into(),
            );
            Err(
                "the installer did not finish in 45 minutes — it is probably waiting for input"
                    .into(),
            )
        }
        Err(InstallError::Exited(status)) => {
            emit_progress(
                &app,
                &title,
                "failed",
                "Installer exited with an error".into(),
            );
            Err(format!("installer exited with {status}"))
        }
        Err(InstallError::Spawn(e)) => Err(e),
    }
}
