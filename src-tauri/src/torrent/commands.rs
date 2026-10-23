use tauri::State;

use crate::config::AppConfig;
use crate::torrent::TorrentListResponse;
use crate::AppState;

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    Ok(state.torrents.lock().await.get_config())
}

#[tauri::command]
pub async fn set_config(state: State<'_, AppState>, config: AppConfig) -> Result<(), String> {
    state
        .torrents
        .lock()
        .await
        .set_config(config)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn torrent_add(
    state: State<'_, AppState>,
    id: String,
    name: String,
    magnet: String,
    source: Option<String>,
    size_bytes: Option<u64>,
) -> Result<(), String> {
    state
        .torrents
        .lock()
        .await
        .add(id, name, magnet, String::new(), source, size_bytes)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn torrent_list(state: State<'_, AppState>) -> Result<TorrentListResponse, String> {
    Ok(state.torrents.lock().await.list())
}

#[tauri::command]
pub async fn torrent_pause(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .torrents
        .lock()
        .await
        .pause(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn torrent_resume(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .torrents
        .lock()
        .await
        .resume(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn torrent_remove(
    state: State<'_, AppState>,
    id: String,
    delete_files: bool,
) -> Result<(), String> {
    state
        .torrents
        .lock()
        .await
        .remove(&id, delete_files)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn torrent_retry(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .torrents
        .lock()
        .await
        .retry(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn torrent_remove_history(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .torrents
        .lock()
        .await
        .remove_history(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn torrent_clear_history(state: State<'_, AppState>) -> Result<(), String> {
    state
        .torrents
        .lock()
        .await
        .clear_history()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_search_port(state: State<'_, AppState>) -> Result<u16, String> {
    Ok(state.search.lock().await.port())
}

#[tauri::command]
pub fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
pub async fn take_pending_launch(
    state: State<'_, crate::AppState>,
) -> Result<Option<crate::launch::PendingLaunch>, String> {
    Ok(state.pending_launch.lock().await.take())
}
