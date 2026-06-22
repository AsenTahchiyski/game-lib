// Merge logic for store syncs. Rule: never overwrite a user-set status; new
// items get a sensible default. Mutates the passed library in place.
import type { Game, Library, Status } from "./types";

export interface SteamGame {
  appid: number;
  name: string;
  playtimeMinutes: number;
}

export interface GogGame {
  id: string;
  title: string;
}

export interface GogSyncResult {
  refreshToken: string;
  games: GogGame[];
}

export interface EpicGame {
  id: string;
  title: string;
}

export interface EpicSyncResult {
  refreshToken: string;
  games: EpicGame[];
}

export interface IgnGame {
  id: string;
  title: string;
  status: Status; // already mapped to our vocabulary by the Rust side
}

export interface MergeResult {
  added: number;
  updated: number;
}

export function mergeSteamGames(library: Library, games: SteamGame[]): MergeResult {
  const now = new Date().toISOString();
  let added = 0;
  let updated = 0;

  const byAppid = new Map<number, Game>();
  for (const g of library.games) {
    if (g.sources.steam) byAppid.set(g.sources.steam.appid, g);
  }

  for (const sg of games) {
    const existing = byAppid.get(sg.appid);
    if (existing) {
      // Refresh playtime; leave the user's status untouched.
      existing.playtimeMinutes = sg.playtimeMinutes;
      existing.lastSyncedAt = now;
      updated++;
    } else {
      library.games.push({
        id: crypto.randomUUID(),
        title: sg.name || `Steam app ${sg.appid}`,
        sources: { steam: { appid: sg.appid } },
        status: "backlog",
        statusChangedAt: now,
        statusHistory: [{ status: "backlog", at: now }],
        playtimeMinutes: sg.playtimeMinutes,
        addedAt: now,
        lastSyncedAt: now,
      });
      added++;
    }
  }

  return { added, updated };
}

export function mergeGogGames(library: Library, games: GogGame[]): MergeResult {
  const now = new Date().toISOString();
  let added = 0;
  let updated = 0;

  const byId = new Map<string, Game>();
  for (const g of library.games) {
    if (g.sources.gog) byId.set(g.sources.gog.id, g);
  }

  for (const gg of games) {
    const existing = byId.get(gg.id);
    if (existing) {
      // Keep the user's status; just record that we saw it.
      existing.lastSyncedAt = now;
      updated++;
    } else {
      library.games.push({
        id: crypto.randomUUID(),
        title: gg.title || `GOG product ${gg.id}`,
        sources: { gog: { id: gg.id } },
        status: "backlog",
        statusChangedAt: now,
        statusHistory: [{ status: "backlog", at: now }],
        playtimeMinutes: 0, // GOG's owned-games API doesn't expose playtime
        addedAt: now,
        lastSyncedAt: now,
      });
      added++;
    }
  }

  return { added, updated };
}

export function mergeEpicGames(library: Library, games: EpicGame[]): MergeResult {
  const now = new Date().toISOString();
  let added = 0;
  let updated = 0;

  const byId = new Map<string, Game>();
  for (const g of library.games) {
    if (g.sources.epic) byId.set(g.sources.epic.id, g);
  }

  for (const eg of games) {
    const existing = byId.get(eg.id);
    if (existing) {
      existing.lastSyncedAt = now;
      updated++;
    } else {
      library.games.push({
        id: crypto.randomUUID(),
        title: eg.title || `Epic item ${eg.id}`,
        sources: { epic: { id: eg.id } },
        status: "backlog",
        statusChangedAt: now,
        statusHistory: [{ status: "backlog", at: now }],
        playtimeMinutes: 0, // Epic's library API doesn't expose playtime
        addedAt: now,
        lastSyncedAt: now,
      });
      added++;
    }
  }

  return { added, updated };
}

// IGN is unlike the other syncs: it carries the user's *curated status*, which
// is the whole point of migrating from IGN's Playlist app. So new games are
// created with their IGN status (not a default), and a re-import updates a
// previously-imported game's status if it changed in IGN.
export function mergeIgnGames(library: Library, games: IgnGame[]): MergeResult {
  const now = new Date().toISOString();
  let added = 0;
  let updated = 0;

  const byId = new Map<string, Game>();
  for (const g of library.games) {
    if (g.sources.ign) byId.set(g.sources.ign.id, g);
  }

  for (const ig of games) {
    const existing = byId.get(ig.id);
    if (existing) {
      if (existing.status !== ig.status) {
        existing.status = ig.status;
        existing.statusChangedAt = now;
        existing.statusHistory.push({ status: ig.status, at: now });
      }
      existing.lastSyncedAt = now;
      updated++;
    } else {
      library.games.push({
        id: crypto.randomUUID(),
        title: ig.title || `IGN game ${ig.id}`,
        sources: { ign: { id: ig.id } },
        status: ig.status,
        statusChangedAt: now,
        statusHistory: [{ status: ig.status, at: now }],
        playtimeMinutes: 0, // IGN's playlist doesn't expose playtime
        addedAt: now,
        lastSyncedAt: now,
      });
      added++;
    }
  }

  return { added, updated };
}
