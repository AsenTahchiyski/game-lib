# game-lib

A cross-platform **game library status manager**. It tracks the *status* of every
game you own or want — not the files or installs — so you always know what's on
your plate. Think of it as a self-hosted replacement for IGN's Playlist app, with
your library auto-populated from the stores you actually buy games on.

Built with **Tauri 2 + SvelteKit + TypeScript**, with a **Rust** backend for all
native HTTP, OAuth, and file IO (no browser CORS limits, real file access).

## Features

- **Seven statuses** — Wishlist, Backlog, Playing, Beat, Quit, Paused, Free.
  A status *you* set records its date and appends to a per-game history; imported
  statuses are left dateless (shown as `—`) rather than faking an import date.
- **Auto-populated from your stores** — one-click sync pulls your library from:
  - **Steam** — official Web API (API key + SteamID). Includes playtime.
  - **GOG** — unofficial OAuth (log in once, paste the code; token stored on-device).
  - **Epic** — unofficial OAuth (log in once, paste the `authorizationCode`).
  - **IGN Playlist** — one-time migration of your curated statuses from a *public*
    profile (enter your nickname or profile URL — no login). This is the only
    source that carries status; the stores convey ownership only.
- **Cross-source de-duplication** — a game owned on Steam *and* listed in IGN
  becomes one entry holding both source ids. Matched by source id, then by
  normalized title with trailing edition qualifiers stripped (so "Batman: Arkham
  Asylum **Game of the Year Edition**" merges with plain "Batman: Arkham Asylum"),
  while keeping distinct numbered titles apart ("Portal" vs "Portal 2"). Existing
  duplicates are collapsed on file open.
- **Ratings** — each game shows the source store's own rating plus a Metacritic
  score where available: IGN's review score, Steam's review % and Metacritic
  (best-effort; partial when Steam rate-limits). Sortable.
- **List & grid views** — toggle between a dense sortable table and a cover-art
  grid. Every column header sorts; Status and Sources filter.
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

CI builds on every push to `main`:

- **Desktop** (`build.yml`): Linux (`.AppImage`/`.deb`/`.rpm`), Windows, macOS.
- **Android** (`android.yml`): signed, optimized arm64 release APK (~15 MB).

Each build also refreshes a rolling **prerelease** with a stable, login-free
download URL (the GitHub mobile app can't download Actions artifacts, but it can
download releases):

- Linux: [`latest-desktop`](../../releases/tag/latest-desktop) — `.deb` + `.AppImage`
- Android: [`latest-android`](../../releases/tag/latest-android) — the APK

On Linux Mint/Ubuntu, `sudo apt install ./Game.Library_*.deb` is the most reliable
(the `.AppImage` needs `libfuse2`). Full bundles are also on each run's **Artifacts**.

### Android signing

The release APK is signed in CI with a keystore supplied via repo secrets:
`ANDROID_KEYSTORE_BASE64` (base64 of a JKS/PKCS#12 keystore),
`ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_PASSWORD`, `ANDROID_KEY_ALIAS`. Without
them the build still succeeds but skips the signing/publish steps.

## Platform support

Linux, Windows, macOS, and Android are built and shipped. **iOS** is deferred —
the code is identical, but packaging requires a Mac and an Apple Developer account.

## Project layout

```
src/                 SvelteKit frontend
  routes/+page.svelte  library view (list/grid, sort/filter, status editing)
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
