# Torlink-Gui

<img width="1130" height="741" alt="image" src="https://github.com/user-attachments/assets/86747e21-eb71-4334-b9d3-d4c2737ded5c" />


Curated torrent search with a native desktop client.

This is a fork of [baairon/torlink](https://github.com/baairon/torlink). The search side is the same family of adapters — FitGirl, YTS, 1337x, and the rest — but downloads run in a **Tauri** app through **librqbit**, not WebTorrent in the terminal.

## Install

Pick a build from [Releases](https://github.com/Domanffe/torlink-gui/releases):

- Windows — `.msi` or `.exe` (NSIS)
- macOS — `.dmg`
- Linux — `.AppImage`

No Node required for the desktop app.

If you only need search, `npx torlnk` still opens the web UI in a browser (Node 22+). Downloads from the browser build are not supported — use the desktop app for that.

The npm CLI package is named **torlnk** (historical); the desktop app and Rust backend use **torlink**.

## Desktop app

Search by category, queue downloads, pause and resume, seed when done. Settings (⚙ or `Ctrl+,`) let you change the download folder, add extra trackers, and pick the interface language (**System**, **English**, **Russian**). Closing the window sends the app to the system tray.

Each torrent is saved in its own subfolder inside the download directory (for example `Downloads/torlink/My Game/`).

**Downloads** — pause, resume, retry on failure, cancel with a choice to keep or delete files on disk.

**Library** — games you installed, as a cover grid. Add a folder you already have, or pick a finished download and install it: torlink runs Inno Setup / NSIS repacks silently into your games folder, then shows the game with its cover. Play launches the executable and tracks playtime.

**History** — finished torrents sit under “Recently downloaded”; you can grab them again or clear the list.

**Shortcuts** (when the main view is focused): `/` focus search · `Tab` next section · `d` download · `p` pause · `f` retry · `c` cancel

Open the desktop app with a magnet or `.torrent` file: `torlink "magnet:?xt=..."` queues the download immediately.

## Build

```sh
git clone https://github.com/Domanffe/torlink-gui.git
cd torlink-gui
npm install
npm run dev:tauri      # Vite GUI + search sidecar + Tauri window
npm run build:tauri    # installer in src-tauri/target/release/bundle/
npm test
```

The installer bundles **librqbit** (torrents) and a packaged **search-sidecar** binary.

`dev:stack` runs only the Vite dev server and search sidecar (without Tauri) if you need that split.

## Sources

| Category | Sources |
| --- | --- |
| Games | FitGirl, [Online-Fix](https://online-fix.me), Xatab |
| Movies | YTS, The Pirate Bay, 1337x, BitTorrented, TorrentGalaxy |
| TV | EZTV, The Pirate Bay, 1337x, BitTorrented, Torlock |
| Anime | Nyaa, SubsPlease, AniLibria |

Online-Fix and Xatab resolve magnets from torrent files at search time. TorrentGalaxy may hit GalaxyFence captcha occasionally — other sources keep working. If a source is down, the rest still answer.

## Upstream

Based on [baairon/torlink](https://github.com/baairon/torlink) (MIT). Search adapters are occasionally synced from upstream — see [docs/upstream.md](docs/upstream.md).

## Privacy

Nothing goes through our servers. torlink talks to trackers and peers directly. Completed torrents seed until you pause or stop them in the Seeding tab.
