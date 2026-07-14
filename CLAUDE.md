# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A cross-platform game library **status** manager (Tauri 2 + SvelteKit + TypeScript, Rust backend). It tracks the status of games (Wishlist/Backlog/Playing/Beat/Quit/Paused/Free), auto-populating ownership from Steam/GOG/Epic and curated statuses from IGN Playlist. Ships on Linux, Windows, macOS, and Android (iOS deferred).

## Commands

```sh
npm ci
npm run check        # svelte-check type-check — the primary local verification
npm run tauri dev    # run the desktop app (needs Tauri system deps)
```

- **Do not attempt the full Tauri/Rust build locally** — the dev container is too small. All native builds run in GitHub Actions (`.github/workflows/build.yml` desktop, `android.yml` Android). The local feedback loop is `npm run check`; for Rust changes, rely on CI.
- There is no test suite.

## Releasing

Every push to `main` rebuilds all platforms and refreshes rolling prereleases (`latest-desktop`, `latest-android`) — pushing to main *is* releasing. So:

- Bump the version in `package.json` with each pushed batch of changes (semver, sized to the change). `src-tauri/tauri.conf.json` reads its version from `package.json`, so that's the single source of truth.
- The in-app update check (`latest_version` in `src-tauri/src/commands.rs`) fetches `package.json` from `main` on GitHub raw — a version bump is what makes users see "update available".
- Android signing runs off repo secrets (`ANDROID_KEYSTORE_*`); the build stays green without them, just unsigned/unpublished.

## Architecture

**The frontend owns the data model; Rust is a stateless HTTP/OAuth helper.**

- `src/lib/types.ts` — the shared JSON schema (`Library`, `Game`, `Settings`). The Rust side treats the library file as opaque JSON. `LIBRARY_VERSION` guards the format.
- Two files, deliberately separate:
  - **Library** — a single human-readable JSON file at a *user-chosen* path (often a NAS share, synced across devices). Cross-device conflicts use last-write-wins via `updatedAt` with a warning. Never write secrets into it.
  - **Settings/secrets** (Steam key, OAuth refresh tokens, last library path) — stored in the OS app-config dir via Rust commands (`commands.rs`).
- **Library file IO happens in JS** via `@tauri-apps/plugin-fs`, not Rust — on Android the file picker returns `content://` URIs that `std::fs` can't open; plugin-fs routes them through ContentResolver. Keep it that way.
- `src/lib/store.svelte.ts` — the reactive app state (Svelte 5 runes, module-level `$state` singleton `app`). All library mutations go through here and must call `touch()` (bumps `updatedAt`/`dirty`/`changeSeq`, which drives autosave + the undo window). `cleanLibrary` snapshots the last saved state for Undo.
- `src/lib/sync.ts` — cross-source merge/dedup rules. Load-bearing invariants:
  - Match incoming games first by that store's source id, then by `normalizeTitle` (strips trailing edition qualifiers, unifies Roman/Arabic numerals, keeps "Portal" vs "Portal 2" distinct). One game can hold multiple source ids.
  - Ownership-only syncs (Steam/GOG/Epic) never overwrite a user-set status; only IGN carries status. `userEdited: true` protects a game from sync overwrites.
  - Deleted games leave tombstones in `library.removed` (`"steam:123"`, `"title:<norm>"`) so syncs don't resurrect them.
- `src/lib/api.ts` — thin `invoke()` wrappers. **Adding a Rust command requires three touch points**: the `#[tauri::command]` fn in its module, registration in `src-tauri/src/lib.rs` `generate_handler![]`, and a wrapper here.
- `src-tauri/src/` — one module per external service: `steam.rs`, `gog.rs`, `epic.rs` (unofficial OAuth flows), `ign.rs` (maps IGN's status flags to our vocabulary), `hltb.rs` (HowLongToBeat). Native HTTP lives here to escape webview CORS.
- Source toggles (`SourceToggle` in types.ts) are stored as a **deny-list** (`disabledSources`) so newly added sources default to on; a disabled source disappears everywhere (badges, links, sync UI, exclusively-provided games).
- UI: `src/routes/+page.svelte` is the main view (list/grid, sort/filter, status editing); `Settings.svelte`, `GameDetails.svelte`, `AddGame.svelte` are the dialogs.

`samples/example-library.json` is a sample library file for reference.
