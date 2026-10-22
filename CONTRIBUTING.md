# Contributing to kovcheg

kovcheg stays small on purpose. Read the code you're about to touch, match how it already works, and keep your change tight.

## Set up

```sh
git clone https://github.com/Domanffe/kovcheg.git
cd kovcheg-gui
npm install
npm run dev:tauri      # desktop app (recommended)
npm run dev:stack      # browser GUI + search sidecar only
```

## Before you open a PR

```sh
npm run typecheck   # tsc --noEmit, zero errors
npm test            # vitest, all green
```

## The standards

### Match the existing grain

Reuse what's there before you write something new. Desktop UI lives in `src/gui/`. Search adapters live in `src/sources/`. Downloads are handled in Rust (`src-tauri/`).

### Cross-platform or it doesn't ship

kovcheg runs on Windows, macOS, and Linux. Desktop shell actions use `src/gui/util/desktop.ts` (Tauri invoke + browser APIs).

### Fail soft, never crash

When something the user can't control goes wrong, degrade gracefully and say so. Reach for a clear message and a fallback before you reach for an exception.

### Test the logic

Non-trivial logic gets a vitest test. Pure functions are easy — see `src/util/format.test.ts`. For code that shells out, mock the node built-in — see `src/gui/util/desktop.test.ts`.

### Respect the calm theme

The desktop GUI is pastel-violet and quiet. Keep new UI restrained and consistent with `src/gui/`.

## Commits and pull requests

- Use [Conventional Commits](https://www.conventionalcommits.org) prefixes: `feat:`, `fix:`, `docs:`, `chore:`, `refactor:`.
- Say why, not just what. The diff already shows the what.
- One concern per pull request.

Thanks for helping keep kovcheg sharp.
