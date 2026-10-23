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
- **Status: done.** Now `library/{mod 368, commands 374, scan 216, install 125+, covers 121}`,
  no file above 400 production lines. `launch.rs` stayed where it was (already its own module),
  the store (`Library` + its impl) stayed in `mod.rs`. `lib.rs` registers
  `library::commands::library_*`; all ten commands sit in `commands.rs`. One test moved next to
  its module (`best_match_*` → `covers.rs`) because it reads a private field.

## 3. `library_install` — 133 lines, five phases
- guard → plan → run with timeout → detect exe → register + cover + events in one function.
- Fix: `InstallPlan::resolve` (pure), `run_installer(plan)`, `register_installed(...)`.
- **Status: done.** The command is ~60 lines of orchestration; `InstallPlan::new` is pure with
  three tests (sanitized child folder, cannot escape the root, empty title → "Game"),
  `run_installer` owns the deadline and `InstallError::{Spawn,Exited,Timeout}`,
  `launch_for_user` keeps the unknown-installer path, `register_installed` adds the game and
  starts the cover lookup. User-visible messages unchanged.

## 4. Cover refresh implemented three ways
- `library.rs:840` (spawn_cover_lookup), `912-940` (inline chunked loop in import), `1030` (direct fetch_cover).
- Fix: one `refresh_covers(app, lib, ids)` with the concurrency cap inside.
- **Status: done (b6a91a4).** `spawn_cover_refresh(app, lib, paths, games)` owns
  `COVER_BATCH`/`COVER_PAUSE_MS` and the single UI refresh; add, import and install all call it.
  The synchronous `library_refresh_cover` keeps its direct fetch: it returns the app id.

## 5. `boot` duplicates seed reconstruction (`manager.rs:187-235`)
- Two near-identical loops; "queue wins over history" is implied by ordering only.
- `make_seed_item` takes 7 positional args (`manager.rs:123`).
- Fix: `SeedFields` struct + `HashMap` (queue first, history fills gaps), one loop.
- **Status: done (b6a91a4).** `SeedFields::{from_queue, from_history, into_item}`, one map,
  the ordering rule now stated in a comment instead of implied by loop order.

## 6. `tick` — 101 lines, five jobs (`manager.rs:681-782`)
- Stats mapping, queue update, seed update + stray detection, completion, persistence.
- Fix: `apply_torrent_stats`, `apply_seed_stats`, `detect_stray_seed` (pure, tested), `persist_if_changed`.
- **Status: done.** `tick` builds a `TickStats` snapshot and hands it to the two handlers;
  `stray_seed_confirmed(now, grace_until, straying, hits)` is the pure policy, covered by three
  tests (consecutive ticks, one clean tick resets the streak, grace window never counts).

## 7. `LibraryView` — one 260-line component (`LibraryView.tsx:38-297`)
- Toolbar, grid, card menu state, add sheet and install banner in one function.
- Fix: `GameCard` (menu inside), `AddSheet`, `LibraryToolbar`; page becomes composition only.
- **Status: done, with one deliberate omission.** `GameCard` (own menu state) and
  `LibraryAddSheet` extracted; `LibraryView` is 148 lines of composition. The toolbar stayed
  inline on purpose: three buttons with no state — a component would be a props-only wrapper.

## 8. `perSource` type claims completeness it does not have
- `Record<SourceId, SourceState>` filled only for active ids.
- **Status: done differently, and cheaper than proposed.** Narrowing to
  `Record<ActiveSourceId, ...>` would have rippled through `Source`, `LOADERS` and every adapter
  (~15 sites) to fix a lie that has a one-line answer: fill a row for **every** `SOURCE_IDS`,
  with disabled sources explicitly not loading. The type becomes true and nothing else moves.
  Guarded by `src/core/search-state.test.ts` (2 tests).

Status: done, pending the final verification pass (clippy, cargo test, tsc, vitest, live CDP run).
