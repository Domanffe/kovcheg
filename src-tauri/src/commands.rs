#[tauri::command]
pub async fn open_folder(
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<(), String> {
    let download_root = state.torrents.lock().await.get_config().download_dir;
    crate::fs::open_folder(&path, &download_root).map_err(|e| e.to_string())
}
