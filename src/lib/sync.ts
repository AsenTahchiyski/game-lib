// Merge logic for store syncs. Rules:
//  - Match an incoming game to an existing one first by this store's source id,
//    then by normalized title (so a game owned on Steam *and* listed in IGN
//    becomes ONE entry carrying both source ids, not two duplicates).
//  - Never overwrite a user-set status from an ownership-only sync; new items
//    default to backlog. IGN is special: it carries the curated status, so it
//    seeds/updates status.
// Mutates the passed library in place.
import type { Game, Library, Status, StoreId } from "./types";

export interface SteamGame {
  appid: number;
  name: string;
  playtimeMinutes: number;
}

export interface GogGame {
  id: string;
  title: string;
  coverUrl?: string;
}

export interface GogSyncResult {
  refreshToken: string;
  games: GogGame[];
}

export interface EpicGame {
  id: string;
  title: string;
  coverUrl?: string;
}

export interface EpicSyncResult {
  refreshToken: string;
  games: EpicGame[];
}

export interface IgnGame {
  id: string;
  title: string;
  status: Status; // already mapped to our vocabulary by the Rust side
  coverUrl?: string;
}

export interface MergeResult {
  added: number;
  updated: number;
}

/** One incoming game, normalized across the different store shapes. */
interface Incoming {
  id: string; // this store's id, as a string
  title: string;
  playtimeMinutes?: number; // only Steam exposes this
  status?: Status; // only IGN carries a curated status
  coverUrl?: string; // box art, where the source provides one
}

/**
 * Normalize a title for cross-store matching: lowercase, drop everything but
 * letters/digits, collapse whitespace. Deliberately conservative — it keeps
 * "Portal" and "Portal 2" distinct rather than risk false merges.
 */
function normalizeTitle(title: string): string {
  return title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

function getSourceId(game: Game, store: StoreId): string | undefined {
  switch (store) {
    case "steam":
      return game.sources.steam ? String(game.sources.steam.appid) : undefined;
    case "gog":
      return game.sources.gog?.id;
    case "epic":
      return game.sources.epic?.id;
    case "ign":
      return game.sources.ign?.id;
  }
}

function setSourceId(game: Game, store: StoreId, id: string): void {
  switch (store) {
    case "steam":
      game.sources.steam = { appid: Number(id) };
      break;
    case "gog":
      game.sources.gog = { id };
      break;
    case "epic":
      game.sources.epic = { id };
      break;
    case "ign":
      game.sources.ign = { id };
      break;
  }
}

function mergeGames(library: Library, store: StoreId, games: Incoming[]): MergeResult {
  const now = new Date().toISOString();
  let added = 0;
  let updated = 0;

  const bySource = new Map<string, Game>();
  const byTitle = new Map<string, Game>();
  for (const g of library.games) {
    const sid = getSourceId(g, store);
    if (sid) bySource.set(sid, g);
    const norm = normalizeTitle(g.title);
    if (norm && !byTitle.has(norm)) byTitle.set(norm, g);
  }

  for (const ig of games) {
    let existing = bySource.get(ig.id);
    if (!existing) {
      // Fall back to a title match against a game from a different source, but
      // only if that game doesn't already carry this store's id (which would
      // mean it's a distinct title-clashing entry, not the same game).
      const norm = normalizeTitle(ig.title);
      const match = norm ? byTitle.get(norm) : undefined;
      if (match && !getSourceId(match, store)) existing = match;
    }

    if (existing) {
      setSourceId(existing, store, ig.id);
      bySource.set(ig.id, existing);
      if (ig.playtimeMinutes !== undefined) existing.playtimeMinutes = ig.playtimeMinutes;
      if (!existing.coverUrl && ig.coverUrl) existing.coverUrl = ig.coverUrl;
      if (ig.status && existing.status !== ig.status) {
        existing.status = ig.status;
        existing.statusChangedAt = now;
        existing.statusHistory.push({ status: ig.status, at: now });
      }
      existing.lastSyncedAt = now;
      updated++;
    } else {
      const status: Status = ig.status ?? "backlog";
      const game: Game = {
        id: crypto.randomUUID(),
        title: ig.title,
        coverUrl: ig.coverUrl,
        sources: {},
        status,
        statusChangedAt: now,
        statusHistory: [{ status, at: now }],
        playtimeMinutes: ig.playtimeMinutes ?? 0,
        addedAt: now,
        lastSyncedAt: now,
      };
      setSourceId(game, store, ig.id);
      library.games.push(game);
      bySource.set(ig.id, game);
      const norm = normalizeTitle(game.title);
      if (norm && !byTitle.has(norm)) byTitle.set(norm, game);
      added++;
    }
  }

  return { added, updated };
}

export function mergeSteamGames(library: Library, games: SteamGame[]): MergeResult {
  return mergeGames(
    library,
    "steam",
    games.map((g) => ({
      id: String(g.appid),
      title: g.name || `Steam app ${g.appid}`,
      playtimeMinutes: g.playtimeMinutes,
      // Steam's portrait box art is derivable from the appid (no extra request).
      coverUrl: `https://cdn.cloudflare.steamstatic.com/steam/apps/${g.appid}/library_600x900.jpg`,
    })),
  );
}

export function mergeGogGames(library: Library, games: GogGame[]): MergeResult {
  return mergeGames(
    library,
    "gog",
    games.map((g) => ({ id: g.id, title: g.title || `GOG product ${g.id}`, coverUrl: g.coverUrl })),
  );
}

export function mergeEpicGames(library: Library, games: EpicGame[]): MergeResult {
  return mergeGames(
    library,
    "epic",
    games.map((g) => ({ id: g.id, title: g.title || `Epic item ${g.id}`, coverUrl: g.coverUrl })),
  );
}

export function mergeIgnGames(library: Library, games: IgnGame[]): MergeResult {
  return mergeGames(
    library,
    "ign",
    games.map((g) => ({
      id: g.id,
      title: g.title || `IGN game ${g.id}`,
      status: g.status,
      coverUrl: g.coverUrl,
    })),
  );
}
