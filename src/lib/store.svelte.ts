// Reactive app state (Svelte 5 runes). The library lives here as the single
// source of truth; Rust only does file IO and (later) store syncs.
import {
  emptyLibrary,
  TAGS,
  type Game,
  type Library,
  type Settings,
  type SourceToggle,
  type Status,
  type StoreId,
} from "./types";
import * as api from "./api";
import {
  mergeSteamGames,
  mergeGogGames,
  mergeEpicGames,
  mergeIgnGames,
  mergeDuplicate,
  dedupeLibrary,
  normalizeTitle,
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
  changeSeq: 0, // bumped on every library mutation; drives the autosave/undo window
  updateAvailable: null as string | null, // newer released version, if any
});

// Plain (non-proxied) copy of the library as of the last load/save — what
// "Undo" restores. Refreshed after every successful read/write.
let cleanLibrary: Library = emptyLibrary();

function markClean() {
  cleanLibrary = $state.snapshot(app.library) as Library;
}

/** Revert every change made since the last save/load (the autosave undo window). */
export function undoChanges() {
  app.library = structuredClone(cleanLibrary);
  app.dirty = false;
}

const BUILTIN_TAGS: string[] = TAGS;

function steamCover(appid: number): string {
  return `https://cdn.cloudflare.steamstatic.com/steam/apps/${appid}/library_600x900.jpg`;
}

function touch() {
  app.library.updatedAt = new Date().toISOString();
  app.dirty = true;
  app.changeSeq++;
}

/** Change a game's status, recording the date and appending to history. */
export function setStatus(game: Game, status: Status) {
  if (game.status === status) return;
  const at = new Date().toISOString();
  game.status = status;
  game.statusChangedAt = at;
  game.statusHistory.push({ status, at });
  game.userEdited = true;
  touch();
}

/** Remove a single game. Records tombstones so a later sync won't re-add it. */
export function removeGame(game: Game) {
  const removed = new Set(app.library.removed ?? []);
  if (game.sources.steam) removed.add(`steam:${game.sources.steam.appid}`);
  if (game.sources.gog) removed.add(`gog:${game.sources.gog.id}`);
  if (game.sources.epic) removed.add(`epic:${game.sources.epic.id}`);
  if (game.sources.ign) removed.add(`ign:${game.sources.ign.id}`);
  removed.add(`title:${normalizeTitle(game.title)}`);
  app.library.removed = [...removed];
  app.library.games = app.library.games.filter((g) => g.id !== game.id);
  touch();
}

/** Manually add a game. Defaults to the Wishlist but the status is choosable;
 *  optionally attach a store source (Steam also yields a cover). */
export function addManualGame(opts: {
  title: string;
  status?: Status;
  store?: StoreId;
  storeId?: string;
}): Game {
  const now = new Date().toISOString();
  const status = opts.status ?? "wishlist";
  const sources: Game["sources"] = {};
  let coverUrl: string | undefined;
  if (opts.store && opts.storeId) {
    if (opts.store === "steam") {
      const appid = parseInt(opts.storeId, 10);
      if (Number.isFinite(appid) && appid > 0) {
        sources.steam = { appid };
        coverUrl = steamCover(appid);
      }
    } else {
      sources[opts.store] = { id: opts.storeId };
    }
  }
  const game: Game = {
    id: crypto.randomUUID(),
    title: opts.title.trim(),
    coverUrl,
    sources,
    status,
    statusChangedAt: now,
    statusHistory: [{ status, at: now }],
    userEdited: true,
    addedAt: now,
  };
  app.library.games.push(game);
  touch();
  // The guessed Steam cover 404s for newly listed apps (their art only exists
  // at a hashed CDN path), so swap in the real URL once Steam answers. Mutate
  // the in-library proxy (not the raw `game`) so the UI reacts.
  if (sources.steam) {
    const appid = sources.steam.appid;
    void api
      .steamCoverUrl(appid)
      .then((url) => {
        const g = app.library.games.find((x) => x.id === game.id);
        if (url && g && g.coverUrl === steamCover(appid)) {
          g.coverUrl = url;
          touch();
        }
      })
      .catch(() => {});
  }
  return game;
}

/** Merge `other` into `target` (manual de-dup) and drop `other`. */
export function mergeRecords(target: Game, other: Game) {
  if (target.id === other.id) return;
  mergeDuplicate(target, other);
  app.library.games = app.library.games.filter((g) => g.id !== other.id);
  touch();
}

/** All tags available for use: built-ins plus the library's custom ones. */
export function availableTags(): string[] {
  return [...BUILTIN_TAGS, ...(app.library.customTags ?? [])];
}

/** Create a custom tag (and apply it to a game if given). */
export function addCustomTag(name: string, game?: Game) {
  const tag = name.trim().toLowerCase();
  if (!tag) return;
  const custom = app.library.customTags ?? [];
  if (!BUILTIN_TAGS.includes(tag) && !custom.includes(tag)) {
    app.library.customTags = [...custom, tag];
  }
  if (game) {
    const tags = game.tags ?? [];
    if (!tags.includes(tag)) game.tags = [...tags, tag];
    game.userEdited = true;
  }
  touch();
}

/** Delete a custom tag and strip it from every game. Built-ins can't be removed. */
export function deleteCustomTag(name: string) {
  if (BUILTIN_TAGS.includes(name)) return;
  app.library.customTags = (app.library.customTags ?? []).filter((t) => t !== name);
  for (const g of app.library.games) {
    if (g.tags?.includes(name)) g.tags = g.tags.filter((t) => t !== name);
  }
  touch();
}

/** Rename a game. */
export function renameGame(game: Game, title: string) {
  const t = title.trim();
  if (!t || t === game.title) return;
  game.title = t;
  game.userEdited = true;
  touch();
}

/** Set (or clear) a manual cover image URL for a game. */
export function setCover(game: Game, url: string) {
  const u = url.trim();
  game.coverUrl = u || undefined;
  game.userEdited = true;
  touch();
}

/** Toggle an orthogonal tag (e.g. "coop", "casual") on a game. */
export function toggleTag(game: Game, tag: string) {
  const tags = game.tags ?? [];
  game.tags = tags.includes(tag) ? tags.filter((t) => t !== tag) : [...tags, tag];
  game.userEdited = true;
  touch();
}

/** Whether a source (store or price site) is enabled. Default: on. */
export function sourceEnabled(id: string): boolean {
  return !(app.settings.disabledSources ?? []).includes(id as SourceToggle);
}

/** Flip a source toggle and persist immediately (it's device config, not library data). */
export function toggleSourceEnabled(id: SourceToggle) {
  const cur = app.settings.disabledSources ?? [];
  app.settings.disabledSources = cur.includes(id) ? cur.filter((s) => s !== id) : [...cur, id];
  void persistSettings();
}

/**
 * A game is hidden when every source it came from is toggled off. Manual
 * entries (no sources) are always visible.
 */
export function gameVisible(game: Game): boolean {
  const srcs = Object.keys(game.sources);
  return srcs.length === 0 || srcs.some(sourceEnabled);
}

/**
 * Fetch HowLongToBeat times for a game and cache them on the record (also for
 * "no entry found", so we don't re-query on every details open). Network
 * errors leave the game untouched — the next open retries.
 */
export async function fetchHltb(game: Game): Promise<void> {
  try {
    const times = await api.hltbSearch(game.title);
    game.hltb = { ...(times ?? {}), checkedAt: new Date().toISOString() };
    touch();
  } catch {
    // HLTB unreachable or its API shape changed; stay quiet, retry next time.
  }
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
  markClean();
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
      // The file may have moved/unmounted (NAS); start empty. On Android the
      // OS revokes access to picked files after a reinstall — silence there
      // would look like data loss, so say how to get the library back.
      if (app.settings.lastLibraryPath.startsWith("content://")) {
        app.error =
          "Android revoked the app's access to your library file. Use Open… and re-select it to restore access — the data is still there.";
      }
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
  markClean();
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
    touch();
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
    touch();
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
    touch();
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
    touch();
    return result;
  } finally {
    app.busy = false;
  }
}

export type SaveResult = "saved" | "cancelled" | "conflict" | "no-access";

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
    markClean();
    if (app.settings.lastLibraryPath !== path) {
      app.settings.lastLibraryPath = path;
      await persistSettings();
    }
    return "saved";
  } catch (e) {
    // Android revokes SAF/MediaStore write grants after a reinstall (and
    // sometimes a restart) — "com.asen.gamelib has no access to content://…".
    // Signal the UI to have the user re-pick the file, which re-grants access.
    if (path.startsWith("content://")) {
      return "no-access";
    }
    app.error = String(e);
    return "cancelled";
  } finally {
    app.busy = false;
  }
}

/** Pick a (new) location and save there, regardless of the current path.
 *  Re-picking the same file is how Android access is re-granted. */
export async function saveLibraryAs(): Promise<SaveResult> {
  const path = await api.pickSavePath();
  if (!path) return "cancelled";
  app.currentPath = path;
  app.loadedMtime = null;
  return saveLibrary(true);
}

/** Compare dotted version strings; true when `latest` is newer. */
function newerVersion(current: string, latest: string): boolean {
  const a = current.split(".").map(Number);
  const b = latest.split(".").map(Number);
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    const d = (b[i] ?? 0) - (a[i] ?? 0);
    if (d !== 0) return d > 0;
  }
  return false;
}

/**
 * Check GitHub for a newer released version (installers are rebuilt from main
 * on every push, so main's package.json version == the newest build). Sets
 * app.updateAvailable; returns it. Throws on network failure.
 */
export async function checkForUpdate(current: string): Promise<string | null> {
  const latest = await api.latestVersion();
  app.updateAvailable = newerVersion(current, latest) ? latest : null;
  return app.updateAvailable;
}
