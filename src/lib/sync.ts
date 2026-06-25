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
  metacritic?: number;
  storeRating?: number;
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
  storeRating?: number; // IGN review score, normalized 0-100
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
  storeRating?: number; // the source store's own rating, 0-100
  metacritic?: number; // 0-100
}

// Edition/version qualifiers that one store appends but another doesn't, e.g.
// Steam's "Batman: Arkham Asylum Game of the Year Edition" vs IGN's plain
// "Batman: Arkham Asylum". Stripped only as a TRAILING run so a leading
// "Ultimate Marvel vs. Capcom 3" stays distinct from "Marvel vs. Capcom 3".
const EDITION_PHRASES = [
  "game of the year edition",
  "game of the year",
  "directors cut",
  "complete collection",
];
const EDITION_TRAILING =
  /\s(edition|goty|remastered|remaster|redux|deluxe|ultimate|complete|definitive|enhanced|gold|premium|anniversary|special|standard|collection|bundle|hd)\s*$/;

// Multi-character Roman numerals -> Arabic, so "Wolfenstein II" matches
// "Wolfenstein 2". Single letters (i, v, x) are intentionally excluded: they're
// too easily real words/letters in titles (e.g. "Mega Man X").
const ROMAN: Record<string, string> = {
  ii: "2", iii: "3", iv: "4", vi: "6", vii: "7", viii: "8", ix: "9",
  xi: "11", xii: "12", xiii: "13", xiv: "14", xv: "15", xvi: "16",
  xvii: "17", xviii: "18", xix: "19", xx: "20",
};

/**
 * Normalize a title for cross-store matching. Beyond lowercasing and stripping
 * punctuation it unifies common spelling variants across stores: "&" vs "and",
 * Roman vs Arabic numerals, bracketed qualifiers like "[2014]"/"(GOTY)", and
 * trailing edition words. Conservative — it keeps "Portal" and "Portal 2"
 * distinct (numbers are preserved) and only converts unambiguous numerals.
 */
function normalizeTitle(title: string): string {
  let s = title.toLowerCase();
  // Drop bracketed/parenthesized qualifiers, e.g. "[2014]", "(GOTY)". Targets the
  // disambiguation suffix without touching year-named games like "FIFA 2014".
  s = s.replace(/[[(][^\])]*[\])]/g, " ");
  s = s.replace(/&/g, " and ");
  s = ` ${s.replace(/[^a-z0-9]+/g, " ").replace(/\s+/g, " ").trim()} `;
  for (const p of EDITION_PHRASES) s = s.split(` ${p} `).join(" ");
  let prev: string;
  do {
    prev = s;
    s = s.replace(EDITION_TRAILING, " ");
  } while (s !== prev);
  s = s
    .trim()
    .split(" ")
    .map((t) => ROMAN[t] ?? t)
    .join(" ");
  return s.replace(/\s+/g, " ").trim();
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
      if (existing.storeRating === undefined && ig.storeRating !== undefined)
        existing.storeRating = ig.storeRating;
      if (existing.metacritic === undefined && ig.metacritic !== undefined)
        existing.metacritic = ig.metacritic;
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
        // No statusChangedAt/history: a sync doesn't tell us when the status
        // was actually chosen, so leave it unknown (shows "—") rather than
        // stamping today. Playtime stays undefined unless the source gave one.
        statusHistory: [],
        playtimeMinutes: ig.playtimeMinutes,
        storeRating: ig.storeRating,
        metacritic: ig.metacritic,
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

// Fold `dup` into `target` — the same game arriving from another source or as a
// different edition.
export function mergeDuplicate(target: Game, dup: Game): void {
  Object.assign(target.sources, dup.sources);
  // Keep the more meaningful status: one IGN carries (curated) and/or any
  // non-default status outranks a plain "backlog".
  const score = (g: Game) => (g.sources.ign ? 2 : 0) + (g.status !== "backlog" ? 1 : 0);
  if (score(dup) > score(target)) {
    target.status = dup.status;
    target.statusChangedAt = dup.statusChangedAt;
  }
  if (target.playtimeMinutes === undefined) {
    target.playtimeMinutes = dup.playtimeMinutes;
  } else if (dup.playtimeMinutes !== undefined) {
    target.playtimeMinutes = Math.max(target.playtimeMinutes, dup.playtimeMinutes);
  }
  if (!target.coverUrl) target.coverUrl = dup.coverUrl;
  if (target.storeRating === undefined) target.storeRating = dup.storeRating;
  if (target.metacritic === undefined) target.metacritic = dup.metacritic;
  target.statusHistory = [...target.statusHistory, ...dup.statusHistory];
  if (dup.addedAt < target.addedAt) target.addedAt = dup.addedAt;
  if (dup.lastSyncedAt && (!target.lastSyncedAt || dup.lastSyncedAt > target.lastSyncedAt)) {
    target.lastSyncedAt = dup.lastSyncedAt;
  }
}

/**
 * Collapse entries that resolve to the same normalized title into one game,
 * combining their source ids. Fixes duplicates created before edition-aware
 * matching existed (e.g. a Steam copy and an IGN copy sitting as two rows).
 * Returns how many duplicates were merged away.
 */
export function dedupeLibrary(library: Library): number {
  const byKey = new Map<string, Game>();
  const kept: Game[] = [];
  let merged = 0;
  for (const g of library.games) {
    const key = normalizeTitle(g.title);
    const existing = key ? byKey.get(key) : undefined;
    if (existing) {
      mergeDuplicate(existing, g);
      merged++;
    } else {
      kept.push(g);
      if (key) byKey.set(key, g);
    }
  }
  if (merged > 0) library.games = kept;
  return merged;
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
      storeRating: g.storeRating,
      metacritic: g.metacritic,
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
      storeRating: g.storeRating,
    })),
  );
}
