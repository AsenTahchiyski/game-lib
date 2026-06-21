// Reactive app state (Svelte 5 runes). The library lives here as the single
// source of truth; Rust only does file IO and (later) store syncs.
import { emptyLibrary, type Game, type Library, type Settings, type Status } from "./types";
import * as api from "./api";
import { mergeSteamGames, type MergeResult } from "./sync";

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

export async function persistSettings() {
  await api.saveSettings($state.snapshot(app.settings));
}

async function loadPath(path: string) {
  const lib = await api.readLibrary(path);
  app.library = lib;
  app.currentPath = path;
  app.loadedMtime = await api.fileMtime(path);
  app.dirty = false;
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
