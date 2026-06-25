// Reactive app state (Svelte 5 runes). The library lives here as the single
// source of truth; Rust only does file IO and (later) store syncs.
import { emptyLibrary, type Game, type Library, type Settings, type Status } from "./types";
import * as api from "./api";
import {
  mergeSteamGames,
  mergeGogGames,
  mergeEpicGames,
  mergeIgnGames,
  dedupeLibrary,
  type MergeResult,
} from "./sync";

export const app = $state({
  library: emptyLibrary() as Library,
  currentPath: null as string | null,
  settings: {} as Settings,
  dirty: false,
  loadedMtime: null as number | null, // file mtime when we last read/wrote it
  busy: false,
  error: null as string | null,
});

function touch() {
  app.library.updatedAt = new Date().toISOString();
  app.dirty = true;
}

/** Change a game's status, recording the date and appending to history. */
export function setStatus(game: Game, status: Status) {
  if (game.status === status) return;
  const at = new Date().toISOString();
  game.status = status;
  game.statusChangedAt = at;
  game.statusHistory.push({ status, at });
  touch();
}

/** Remove a single game from the library. */
export function removeGame(game: Game) {
  app.library.games = app.library.games.filter((g) => g.id !== game.id);
  touch();
}

/** Manually add a game straight to the Wishlist. Optionally attach a Steam app
 *  id so it gets a cover (and can be matched/enriched by a later Steam sync). */
export function addManualGame(title: string, steamAppid?: number): Game {
  const now = new Date().toISOString();
  const game: Game = {
    id: crypto.randomUUID(),
    title: title.trim(),
    coverUrl: steamAppid
      ? `https://cdn.cloudflare.steamstatic.com/steam/apps/${steamAppid}/library_600x900.jpg`
      : undefined,
    sources: steamAppid ? { steam: { appid: steamAppid } } : {},
    status: "wishlist",
    statusChangedAt: now,
    statusHistory: [{ status: "wishlist", at: now }],
    addedAt: now,
  };
  app.library.games.push(game);
  touch();
  return game;
}

/** Rename a game. */
export function renameGame(game: Game, title: string) {
  const t = title.trim();
  if (!t || t === game.title) return;
  game.title = t;
  touch();
}

/** Set (or clear) a manual cover image URL for a game. */
export function setCover(game: Game, url: string) {
  const u = url.trim();
  game.coverUrl = u || undefined;
  touch();
}

/** Toggle an orthogonal tag (e.g. "coop", "casual") on a game. */
export function toggleTag(game: Game, tag: string) {
  const tags = game.tags ?? [];
  game.tags = tags.includes(tag) ? tags.filter((t) => t !== tag) : [...tags, tag];
  touch();
}

export async function persistSettings() {
  await api.saveSettings($state.snapshot(app.settings));
}

async function loadPath(path: string) {
  const lib = await api.readLibrary(path);
  app.library = lib;
  app.currentPath = path;
  app.loadedMtime = await api.fileMtime(path);
  // Clean up any duplicates from before edition-aware matching existed; if it
  // changed anything, leave the library marked dirty so the user can save it.
  const removed = dedupeLibrary(app.library);
  // Drop import-artifact history entries (older imports stamped a bogus
  // "<status> @ import date"). Such an entry has at === addedAt.
  let cleaned = 0;
  for (const g of app.library.games) {
    const before = g.statusHistory.length;
    g.statusHistory = g.statusHistory.filter((e) => e.at !== g.addedAt);
    cleaned += before - g.statusHistory.length;
  }
  app.dirty = removed > 0 || cleaned > 0;
  if (app.settings.lastLibraryPath !== path) {
    app.settings.lastLibraryPath = path;
    await persistSettings();
  }
}

/** Load settings on startup and re-open the last library if it still exists. */
export async function init() {
  app.settings = await api.loadSettings();
  if (app.settings.lastLibraryPath) {
    try {
      await loadPath(app.settings.lastLibraryPath);
    } catch {
      // The file may have moved/unmounted (NAS); start empty without nagging.
    }
  }
}

export async function openLibrary() {
  const path = await api.pickOpenPath();
  if (!path) return;
  app.busy = true;
  app.error = null;
  try {
    await loadPath(path);
  } catch (e) {
    app.error = String(e);
  } finally {
    app.busy = false;
  }
}

export function newLibrary() {
  app.library = emptyLibrary();
  app.currentPath = null;
  app.loadedMtime = null;
  app.dirty = true;
}

/** Pull the Steam library and merge it in. Returns counts of added/updated. */
export async function syncSteamLibrary(): Promise<MergeResult> {
  const { steamApiKey, steamId } = app.settings;
  if (!steamApiKey || !steamId) {
    throw new Error("Set your Steam API key and SteamID in Settings first.");
  }
  app.busy = true;
  app.error = null;
  try {
    const games = await api.syncSteam(steamApiKey, steamId);
    const result = mergeSteamGames(app.library, games);
    dedupeLibrary(app.library);
    app.library.updatedAt = new Date().toISOString();
    app.dirty = true;
    return result;
  } finally {
    app.busy = false;
  }
}

/** Exchange a pasted GOG auth code for a refresh token and store it. */
export async function gogConnect(code: string): Promise<void> {
  app.busy = true;
  app.error = null;
  try {
    const refreshToken = await api.gogExchangeCode(code);
    app.settings.gogRefreshToken = refreshToken;
    await persistSettings();
  } finally {
    app.busy = false;
  }
}

/** Pull the GOG library and merge it in. Returns counts of added/updated. */
export async function syncGogLibrary(): Promise<MergeResult> {
  const token = app.settings.gogRefreshToken;
  if (!token) {
    throw new Error("Connect your GOG account first.");
  }
  app.busy = true;
  app.error = null;
  try {
    const { refreshToken, games } = await api.gogSync(token);
    app.settings.gogRefreshToken = refreshToken; // GOG may rotate it
    await persistSettings();
    const result = mergeGogGames(app.library, games);
    dedupeLibrary(app.library);
    app.library.updatedAt = new Date().toISOString();
    app.dirty = true;
    return result;
  } finally {
    app.busy = false;
  }
}

/** Exchange a pasted Epic auth code for a refresh token and store it. */
export async function epicConnect(code: string): Promise<void> {
  app.busy = true;
  app.error = null;
  try {
    const refreshToken = await api.epicExchangeCode(code);
    app.settings.epicRefreshToken = refreshToken;
    await persistSettings();
  } finally {
    app.busy = false;
  }
}

/** Pull the Epic library and merge it in. Returns counts of added/updated. */
export async function syncEpicLibrary(): Promise<MergeResult> {
  const token = app.settings.epicRefreshToken;
  if (!token) {
    throw new Error("Connect your Epic account first.");
  }
  app.busy = true;
  app.error = null;
  try {
    const { refreshToken, games } = await api.epicSync(token);
    app.settings.epicRefreshToken = refreshToken;
    await persistSettings();
    const result = mergeEpicGames(app.library, games);
    dedupeLibrary(app.library);
    app.library.updatedAt = new Date().toISOString();
    app.dirty = true;
    return result;
  } finally {
    app.busy = false;
  }
}

/**
 * Import a public IGN Playlist by nickname/profile URL and merge it in. Unlike
 * the store syncs this carries curated statuses, so it seeds/updates status.
 * Persists the nickname for one-click re-imports.
 */
export async function syncIgnLibrary(): Promise<MergeResult> {
  const nickname = app.settings.ignNickname;
  if (!nickname) {
    throw new Error("Enter your IGN nickname in Settings first.");
  }
  app.busy = true;
  app.error = null;
  try {
    const games = await api.ignSync(nickname);
    const result = mergeIgnGames(app.library, games);
    dedupeLibrary(app.library);
    app.library.updatedAt = new Date().toISOString();
    app.dirty = true;
    return result;
  } finally {
    app.busy = false;
  }
}

export type SaveResult = "saved" | "cancelled" | "conflict";

/**
 * Persist the library. Returns "conflict" if the on-disk file is newer than the
 * copy we loaded (another device wrote to the NAS file) and `force` is false.
 */
export async function saveLibrary(force = false): Promise<SaveResult> {
  let path = app.currentPath;
  if (!path) {
    path = await api.pickSavePath();
    if (!path) return "cancelled";
  } else if (!force) {
    const onDisk = await api.fileMtime(path);
    if (onDisk !== null && app.loadedMtime !== null && onDisk > app.loadedMtime) {
      return "conflict";
    }
  }
  app.busy = true;
  app.error = null;
  try {
    app.library.updatedAt = new Date().toISOString();
    await api.writeLibrary(path, $state.snapshot(app.library));
    app.currentPath = path;
    app.loadedMtime = await api.fileMtime(path);
    app.dirty = false;
    if (app.settings.lastLibraryPath !== path) {
      app.settings.lastLibraryPath = path;
      await persistSettings();
    }
    return "saved";
  } catch (e) {
    app.error = String(e);
    return "cancelled";
  } finally {
    app.busy = false;
  }
}
