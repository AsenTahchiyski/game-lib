# game-lib

A cross-platform **game library status manager**. It tracks the *status* of every
game you own or want — not the files or installs — so you always know what's on
your plate. Think of it as a self-hosted replacement for IGN's Playlist app, with
your library auto-populated from the stores you actually buy games on.

Built with **Tauri 2 + SvelteKit + TypeScript**, with a **Rust** backend for all
native HTTP, OAuth, and file IO (no browser CORS limits, real file access).

## Features

- **Seven statuses** — Wishlist, Backlog, Playing, Beat, Quit, Paused, Free.
  Every status change records its date and is appended to a per-game history.
- **Auto-populated from your stores** — one-click sync pulls your library from:
  - **Steam** — official Web API (API key + SteamID). Includes playtime.
  - **GOG** — unofficial OAuth (log in once, paste the code; token stored on-device).
  - **Epic** — unofficial OAuth (log in once, paste the `authorizationCode`).
  - **IGN Playlist** — one-time migration of your curated statuses from a *public*
    profile (enter your nickname or profile URL — no login). This is the only
    source that carries status; the stores convey ownership only.
- **Cross-source de-duplication** — a game owned on Steam *and* listed in IGN
  becomes one entry holding both source ids (matched by id, then normalized
  title), so syncing in any order won't create duplicates.
- **Cover art** — box art from IGN, Steam (derived from appid), Epic, and GOG.
- **Your data, one file** — the library is a single human-readable JSON file you
  choose (e.g. on a NAS) so it can sync across devices and be shared. Secrets
  (Steam key, OAuth tokens) are stored separately in the OS app-config dir and
  are **never** written to the shared file.
- **Cross-device safe** — last-write-wins via `updatedAt`, with a warning before
  overwriting a file that another device saved more recently.

## How status mapping works (IGN import)

IGN stores status as a set of flags; they map to our vocabulary as:

| IGN        | game-lib |
|------------|----------|
| playing    | Playing  |
| completed  | Beat     |
| paused     | Paused   |
| wishlist   | Wishlist |
| archived   | Quit     |
| backlog    | Backlog  |

Note IGN's wishlist is a separate bucket from the main library and is fetched
explicitly so those games aren't missed. IGN has no "Free" equivalent.

## Building & running

> The full Tauri build (Rust + native bundling) runs in **GitHub Actions**, not
> locally — see [`.github/workflows`](.github/workflows). The fast local feedback
> loop is the frontend type-check.

Local development:

```sh
npm ci
npm run check        # type-check the frontend (fast, runs anywhere)
npm run tauri dev    # run the desktop app (needs the Tauri system deps)
```

CI produces downloadable artifacts on every push to `main`:

- **Desktop** (`build.yml`): Linux (`.AppImage`/`.deb`/`.rpm`), Windows, macOS.
- **Android** (`android.yml`): debug APK.

Download them from the run's **Artifacts** section. The Linux `.AppImage` needs
no install — `chmod +x` and run.

## Platform support

Linux, Windows, macOS, and Android are built and shipped. **iOS** is deferred —
the code is identical, but packaging requires a Mac and an Apple Developer account.

## Project layout

```
src/                 SvelteKit frontend
  routes/+page.svelte  library view (list, filters, status editing)
  lib/
    types.ts           data model (the shared JSON schema)
    sync.ts            cross-source merge + de-duplication
    store.svelte.ts    reactive app state, sync orchestration
    api.ts             Tauri command wrappers
    Settings.svelte    credentials + per-store sync UI
src-tauri/src/       Rust backend
  steam.rs gog.rs epic.rs ign.rs   store connectors
  commands.rs        file IO + settings persistence
```
