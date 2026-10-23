use super::*;

pub(crate) const COVERS_DIR: &str = "covers";
pub(crate) const STEAM_SEARCH: &str = "https://store.steampowered.com/api/storesearch/";
pub(crate) const STEAM_CDN: &str = "https://cdn.akamai.steamstatic.com/steam/apps";
pub(crate) fn cover_data_url(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.is_empty() {
        return None;
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:image/jpeg;base64,{encoded}"))
}

#[derive(Deserialize)]
pub(crate) struct SteamSearch {
    #[serde(default)]
    items: Vec<SteamItem>,
}

#[derive(Deserialize)]
pub(crate) struct SteamItem {
    id: u32,
    #[serde(default)]
    name: String,
}

pub(crate) fn best_match<'a>(items: &'a [SteamItem], title: &str) -> Option<&'a SteamItem> {
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
pub(crate) async fn fetch_cover(
    paths: &Paths,
    id: &str,
    title: &str,
) -> anyhow::Result<Option<u32>> {
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

/// How many Steam cover lookups may be in flight at once. Importing a whole games folder
/// must not fire fifty parallel requests at the store and collect 429s.
pub(crate) const COVER_BATCH: usize = 3;
pub(crate) const COVER_PAUSE_MS: u64 = 400;

/// Fetches missing Steam covers for `games` (id, title) in batches, then refreshes the UI once.
/// Fire-and-forget: the caller does not wait and does not need the result.
pub(crate) fn spawn_cover_refresh(
    app: AppHandle,
    lib: Arc<Mutex<Library>>,
    paths: Paths,
    games: Vec<(String, String)>,
) {
    if games.is_empty() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        for chunk in games.chunks(COVER_BATCH) {
            let mut tasks = Vec::new();
            for (id, title) in chunk {
                let (task_lib, task_paths) = (lib.clone(), paths.clone());
                let (id, title) = (id.clone(), title.clone());
                tasks.push(tauri::async_runtime::spawn(async move {
                    match fetch_cover(&task_paths, &id, &title).await {
                        Ok(Some(app_id)) => {
                            task_lib.lock().await.set_steam_app_id(&id, app_id);
                        }
                        Ok(None) => tracing::info!(title = %title, "no steam match for cover"),
                        Err(e) => tracing::warn!(title = %title, "cover lookup failed: {e:#}"),
                    }
                }));
            }
            for task in tasks {
                let _ = task.await;
            }
            tokio::time::sleep(Duration::from_millis(COVER_PAUSE_MS)).await;
        }
        broadcast(&app, &lib).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
