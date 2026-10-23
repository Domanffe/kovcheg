//! Spawns and manages the Node search sidecar bundled with the desktop app.

use std::time::Duration;

use anyhow::{anyhow, Context};
use tauri::AppHandle;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tracing::{info, warn};

const PORT_PREFIX: &str = "search-sidecar-port:";

pub struct SearchSidecar {
    port: u16,
    child: Option<CommandChild>,
}

impl SearchSidecar {
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn kill(&mut self) {
        if let Some(child) = self.child.take() {
            let _ = child.kill();
        }
    }
}

impl Drop for SearchSidecar {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Ensure the search API is reachable; spawn the bundled sidecar if needed.
pub async fn ensure_search_sidecar(app: &AppHandle) -> anyhow::Result<SearchSidecar> {
    match spawn_bundled_sidecar(app).await {
        Ok(sidecar) => Ok(sidecar),
        Err(e) => {
            warn!("bundled sidecar failed: {e:#}");
            #[cfg(debug_assertions)]
            {
                spawn_node_fallback(app).await
            }
            #[cfg(not(debug_assertions))]
            {
                Err(e)
            }
        }
    }
}

/// Proxy configured in the app, if any. The sidecar needs it as an env var because Node
/// does not read the system proxy the way a browser or curl does.
fn config_proxy() -> Option<String> {
    let paths = crate::config::resolve_paths();
    let proxy = crate::config::load_config(&paths).proxy.trim().to_string();
    (!proxy.is_empty()).then_some(proxy)
}

async fn spawn_bundled_sidecar(app: &AppHandle) -> anyhow::Result<SearchSidecar> {
    let sidecar = app
        .shell()
        .sidecar("search-sidecar")
        .context("search-sidecar binary not bundled (run npm run build:sidecar:bin)")?
        .args(["0"]);

    let sidecar = match config_proxy() {
        Some(proxy) => {
            info!("search sidecar uses the configured proxy");
            sidecar.env("KOVCHEG_PROXY", proxy)
        }
        None => sidecar,
    };

    let (mut rx, child) = sidecar
        .spawn()
        .context("failed to spawn search-sidecar process")?;

    let port = tokio::time::timeout(Duration::from_secs(45), async {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let text = String::from_utf8_lossy(&line);
                    for l in text.lines() {
                        if let Some(p) = l.strip_prefix(PORT_PREFIX) {
                            let port: u16 = p.trim().parse().context("invalid sidecar port")?;
                            return Ok(port);
                        }
                    }
                }
                CommandEvent::Stderr(line) => {
                    let text = String::from_utf8_lossy(&line);
                    if !text.trim().is_empty() {
                        tracing::debug!(target: "search-sidecar", "{}", text.trim());
                    }
                }
                CommandEvent::Error(err) => return Err(anyhow!("sidecar error: {err}")),
                CommandEvent::Terminated(payload) => {
                    return Err(anyhow!(
                        "sidecar exited before ready (code {:?})",
                        payload.code
                    ));
                }
                _ => {}
            }
        }
        Err(anyhow!("sidecar closed stdout without reporting port"))
    })
    .await
    .context("timed out waiting for search sidecar")??;

    // Wait until HTTP is actually up (pkg/node startup can lag behind stdout).
    for _ in 0..50 {
        if health_ok(port).await {
            info!(port, "search sidecar ready");
            return Ok(SearchSidecar {
                port,
                child: Some(child),
            });
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let _ = child.kill();
    Err(anyhow!(
        "search sidecar port {port} did not respond to /health"
    ))
}

#[cfg(debug_assertions)]
async fn spawn_node_fallback(app: &AppHandle) -> anyhow::Result<SearchSidecar> {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let js = manifest
        .parent()
        .context("project root")?
        .join("dist")
        .join("search-sidecar.cjs");

    if !js.exists() {
        return Err(anyhow!(
            "no search sidecar: run `npm run dev:search` or `npm run build:sidecar`"
        ));
    }

    info!(path = %js.display(), "spawning node search sidecar (dev fallback)");

    let command = app
        .shell()
        .command("node")
        .args([js.to_string_lossy().to_string(), "0".to_string()]);

    let command = match config_proxy() {
        Some(proxy) => command.env("KOVCHEG_PROXY", proxy),
        None => command,
    };

    let (mut rx, child) = command.spawn().context("failed to spawn node sidecar")?;

    let port = tokio::time::timeout(Duration::from_secs(30), async {
        while let Some(event) = rx.recv().await {
            if let CommandEvent::Stdout(line) = event {
                let text = String::from_utf8_lossy(&line);
                for l in text.lines() {
                    if let Some(p) = l.strip_prefix(PORT_PREFIX) {
                        return Ok(p.trim().parse::<u16>()?);
                    }
                }
            }
        }
        Err(anyhow!("node sidecar exited before ready"))
    })
    .await??;

    Ok(SearchSidecar {
        port,
        child: Some(child),
    })
}

async fn health_ok(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{port}/health");
    match reqwest::get(&url).await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}
