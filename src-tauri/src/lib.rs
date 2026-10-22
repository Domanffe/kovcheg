mod commands;
mod config;
mod fs;
mod launch;
mod library;
mod path_guard;
mod persistence;
mod sidecar;
mod torrent;

use std::sync::Arc;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, RunEvent, WindowEvent,
};
use tokio::sync::Mutex;

use torrent::TorrentManager;

pub struct AppState {
    pub torrents: Arc<Mutex<TorrentManager>>,
    pub search: Mutex<sidecar::SearchSidecar>,
    pub library: Arc<Mutex<library::Library>>,
    pub(crate) pending_launch: Mutex<Option<launch::PendingLaunch>>,
}

fn is_russian_locale() -> bool {
    for key in ["LANG", "LC_ALL", "LC_MESSAGES", "LANGUAGE"] {
        if let Ok(val) = std::env::var(key) {
            let lower = val.to_lowercase();
            if lower.starts_with("ru") {
                return true;
            }
        }
    }
    false
}

fn tray_labels() -> (&'static str, &'static str) {
    if is_russian_locale() {
        ("Показать kovcheg", "Выход")
    } else {
        ("Show kovcheg", "Quit")
    }
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let (show_label, quit_label) = tray_labels();
    let show = MenuItem::with_id(app, "show", show_label, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", quit_label, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::FailedToReceiveMessage)?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("kovcheg")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let _ = librqbit::try_increase_nofile_limit();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            setup_tray(app)?;

            let handle = app.handle().clone();
            let library = library::Library::boot().map_err(|e| -> Box<dyn std::error::Error> {
                tracing::error!("failed to load library: {e:#}");
                format!("failed to load library: {e:#}").into()
            })?;
            let (search, manager) = tauri::async_runtime::block_on(async {
                let search = sidecar::ensure_search_sidecar(&handle).await?;
                let manager = TorrentManager::boot(handle.clone(), search.port()).await?;
                Ok::<_, anyhow::Error>((search, manager))
            })
            .map_err(|e| -> Box<dyn std::error::Error> {
                tracing::error!("failed to boot kovcheg: {e:#}");
                format!("failed to boot kovcheg: {e:#}").into()
            })?;

            app.manage(AppState {
                torrents: manager,
                search: Mutex::new(search),
                library: Arc::new(Mutex::new(library)),
                pending_launch: Mutex::new(launch::parse_startup_launch()),
            });

            if let Some(window) = app.get_webview_window("main") {
                let w = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = w.hide();
                    }
                });
                let _ = window.show();
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            torrent::commands::get_config,
            torrent::commands::set_config,
            torrent::commands::torrent_add,
            torrent::commands::torrent_list,
            torrent::commands::torrent_pause,
            torrent::commands::torrent_resume,
            torrent::commands::torrent_remove,
            torrent::commands::torrent_retry,
            torrent::commands::torrent_remove_history,
            torrent::commands::torrent_clear_history,
            torrent::commands::get_search_port,
            torrent::commands::get_version,
            torrent::commands::take_pending_launch,
            library::library_list,
            library::library_probe,
            library::library_open_folder,
            library::library_add,
            library::library_import,
            library::library_set_exe,
            library::library_remove,
            library::library_launch,
            library::library_refresh_cover,
            library::library_install,
            commands::open_folder,
        ])
        .build(tauri::generate_context!())
        .expect("error building kovcheg")
        .run(|app, event| {
            if matches!(event, RunEvent::Exit) {
                if let Some(state) = app.try_state::<AppState>() {
                    if let Ok(mut search) = state.search.try_lock() {
                        search.kill();
                    }
                }
            }
        });
}
