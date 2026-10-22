# Changelog

## 2.0.0 — rebrand to Kovcheg

The app is now **Kovcheg** (ковчег — the chest that keeps everything). Same code base, its own
name and icon: nothing user-facing, in package names, in the data folder or in updater paths
refers to `torlink` any more.

### Licensing
- `LICENSE` keeps the upstream MIT notice (`Copyright (c) 2026 bairon.dev`) and adds
  `Copyright (c) 2026 Alexander (Domanffe)` for the desktop client and library
- new `NOTICE` states that Kovcheg is a derivative of `baairon/torlink`
- "torlink" stays only in attribution and in `docs/upstream.md`

### Rebrand
- product, bundle, window, tray and package names → `kovcheg`; identifier → `app.kovcheg.desktop`
- state folder moves to `kovcheg`; on first launch the library, queue, history, seeds, covers
  and `config.json` are copied once from the old `torlink` folder
- env vars `TORLINK_STATE_DIR` / `TORLINK_SEARCH_PORT` → `KOVCHEG_*` (the old names are still read)
- new app icon and wordmark (chest glyph)

### Added
- **Library** — installed games as a cover grid with playtime, "playing now" and per-game actions (open folder, set `.exe`, refresh cover, remove)
- Play launches the game executable, watches the process and adds session time to the playtime
- **Repack auto-install** — after a download finishes, torlink detects Inno Setup / NSIS installers and installs them silently into the configured games folder; unknown installers are handed to the user instead of guessing flags
- Covers and Steam app ids from the public Steam store search, cached on disk and refreshable per game
- Settings: **Install games into** (empty = download folder)
- **TPB games** source — The Pirate Bay PC-games category with real swarm counts (`apibay.org`, category 401)
- **Import installed games** — point the library at a folder (e.g. `D:\Games`) and every subfolder with a playable `.exe` becomes a card with its cover
- **Install from downloads** — a finished download gets an "Install game" button right in the Downloads view, no detour through the library

### Changed
- The search sidecar honours `HTTPS_PROXY` / `HTTP_PROXY`: on networks that block FitGirl, nyaa or EZTV every such source used to fail while the same URL opened in a browser
- Source requests use a browser user agent instead of the package UA, which some CDNs answer with a challenge
- Game-title cleanup strips `[FitGirl Repack]`, `SteamRIP.com` and similar noise out of folder names

### Fixed
- `detect_exe` no longer mistakes games whose names contain helper words for installers (`CrashoutCrew.exe` was skipped as a "crash" binary)

## 1.5.3 — 2026-07-11

### Fixed
- **Xatab** — POST search (GET returned 404); parser updated for new grid layout
- **AniLibria** — migrated from deprecated `api.anilibria.tv/v3` (410) to `anilibria.top/api/v1`
- **TGx** — added mirror hosts (`tgx.rs`, `torrentgalaxy.one`, `torrentgalaxy.su`) with failover on 522/520

## 1.5.2 — 2026-07-11

### Added
- **Xatab** (Games) — repacks from byxatab.com
- **TorrentGalaxy / TGx** (Movies) — HTML search with seeders
- **Torlock** (TV) — searchable TV index with seeders
- **AniLibria** (Anime) — official API v3 with RU titles and swarm stats
- Desktop CLI launch: `torlink "magnet:..."` or `torlink file.torrent` queues download on startup
- Search filter **Seeded only** (toggle in results view, hotkey `a`)

### Changed
- Source metadata lives in `meta.ts` only; adapters export `search` via `buildSource()`
- EZTV filters latest API pages by title when you type a query (no longer browse-only)
- BitTorrented Movies/TV split results by release name patterns (no duplicate rows)

### Removed
- Unused Node helpers `src/util/clipboard.ts` and `src/util/openFolder.ts` (GUI uses `desktop.ts`)

### Notes
- TGx may return GalaxyFence (captcha) errors; the search continues with other sources

## 1.5.1 — 2026-07-11

### Added
- Each torrent downloads into its own subfolder under the configured download directory
- BitTorrented search source for Movies and TV (synced from upstream 1.4.0)
- EZTV "browse only" badge in search source strip
- Settings hint for per-torrent subfolders and restart notice when changing download folder

### Changed
- UI refresh: Vercel/Geist dark theme — neutral surfaces, white primary buttons, `#333` borders
- HTTP(S) tracker endpoints in default magnets for networks that block UDP (upstream 1.4.0)
- 1337x remembers its last working mirror between searches (upstream 1.4.0)
- Search streaming coalesces UI updates to avoid redraw bursts (upstream 1.4.0)
- Accessibility fallbacks for reduced motion preferences

### Fixed
- Download directory validation in settings (`set_config`)
- Torrent ID sanitization for cached `.torrent` metadata files
- Path re-validation on torrent resume and restore
- `SeedStatus::Missing` now persists correctly across restarts
- Search cache eviction (TTL + 200 entry cap)
- Sources without health data show "—" instead of misleading 0 seeders

## 1.5.0 — 2026-07-09

### Changed
- Removed deprecated Ink TUI and TypeScript download stack; desktop app is the only download path
- Hardened Tauri security (CSP, scoped shell capabilities, download path validation)
- Expanded test coverage for search, sources, and Rust backend
- Added Dependabot and dependency audits in CI

## 1.4.8 — 2026-07-08

### Fixed
- Desktop app no longer hangs on startup when torrent restore blocks on unreachable trackers
- Search source pages decode declared charsets correctly, fixing mojibake in titles
- CI and release workflows install Linux desktop deps and build sidecar binaries before Tauri

### Changed
- Updater endpoints prefer jsDelivr and raw.githubusercontent.com over GitHub Releases

## 1.4.7 — 2026-07-08

### Fixed
- Update check no longer shows "latest version" when the network or updater request fails
- Settings display the installed app version during update checks

### Changed
- Added fallback updater manifest at `updater/latest.json` on GitHub raw
- Release workflow syncs the updater mirror after each tagged build

## 1.4.6 — 2026-07-08

### Added
- Split CI and release workflows; tests run before release builds
- Open folder action in Downloads, History, and Seeding views
- Copy magnet button in search details

### Fixed
- Windows persistence: atomic JSON writes now replace existing files correctly
- Re-downloading from history no longer deletes completed torrent files
- Search sidecar no longer trusts a random process on port 3847
- Desktop client caches `.torrent` metadata for resume and reseed
- Main branch CI no longer requires updater signing keys

### Changed
- Tracker settings save through a single config path
- Version metadata synced across package, Tauri, and CLI entrypoints
