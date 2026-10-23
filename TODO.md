# Code quality backlog

From the thermo-nuclear review of 2026-10-23. Order of work is deliberate:
gates first, then pure deletions, then mechanical moves, then the splits.

## 1. Gates not automated — CI has no fmt/clippy
- `ci.yml:44-47` runs `cargo test` and `cargo check` only.
- A dead `mut` (`manager.rs:269`) reached a release build and only showed up in the log.
- Fix: `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings` in CI.
- **Status: done.** Steps added to `ci.yml`; the seven clippy findings it immediately
  surfaced (redundant closure, dead `mut`, collapsible `if`, `sort_by_key`, struct init,
  two `?` conversions) are fixed. `cargo fmt` applied across the crate.

## 2. `library.rs` — 1192 production lines, 8 concerns, 10 commands
- Holds store, launcher, installer, cover fetch, folder scan, exe detection, import, events.
- Fix: split into `library/{mod,store,launch,install,scan,covers,commands}.rs`.
- No script or workflow hardcodes the path (checked `scripts/`, `.github/`, `package.json`).

## 3. `library_install` — 133 lines, five phases (`library.rs:1041-1173`)
- guard → plan → run with timeout → detect exe → register + cover + events in one function.
- Fix: `InstallPlan::resolve` (pure), `run_installer(plan)`, `register_installed(...)`.

## 4. Cover refresh implemented three ways
- `library.rs:840` (spawn_cover_lookup), `912-940` (inline chunked loop in import), `1030` (direct fetch_cover).
- Fix: one `refresh_covers(app, lib, ids)` with the concurrency cap inside.

## 5. `boot` duplicates seed reconstruction (`manager.rs:187-235`)
- Two near-identical loops; "queue wins over history" is implied by ordering only.
- `make_seed_item` takes 7 positional args (`manager.rs:123`).
- Fix: `SeedFields` struct + `HashMap` (queue first, history fills gaps), one loop.

## 6. `tick` — 101 lines, five jobs (`manager.rs:681-782`)
- Stats mapping, queue update, seed update + stray detection, completion, persistence.
- Fix: `apply_torrent_stats`, `apply_seed_stats`, `detect_stray_seed` (pure, tested), `persist_if_changed`.

## 7. `LibraryView` — one 260-line component (`LibraryView.tsx:38-297`)
- Toolbar, grid, card menu state, add sheet and install banner in one function.
- Fix: `GameCard` (menu inside), `AddSheet`, `LibraryToolbar`; page becomes composition only.
- Must be verified with a live run (CDP), not just types.

## 8. `perSource` type claims completeness it does not have (`search-state.ts:19-24`)
- `Record<SourceId, SourceState>` filled only for active ids.
- Fix: `ACTIVE_SOURCE_IDS as const` + `ActiveSourceId`, then `Record<ActiveSourceId, SourceState>`.

Status: open
