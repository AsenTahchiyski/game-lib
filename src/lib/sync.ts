// Merge logic for store syncs. Rule: never overwrite a user-set status; new
// items get a sensible default. Mutates the passed library in place.
import type { Game, Library } from "./types";

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
