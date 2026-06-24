// Core data model for the game library. The frontend owns this schema; the Rust
// side treats the library file as opaque JSON.

export type Status =
  | "wishlist"
  | "backlog"
  | "playing"
  | "beat"
  | "quit"
  | "paused"
  | "free";

export const STATUSES: Status[] = [
  "wishlist",
  "backlog",
  "playing",
  "beat",
  "quit",
  "paused",
  "free",
];

export const STATUS_LABELS: Record<Status, string> = {
  wishlist: "Wishlist",
  backlog: "Backlog",
  playing: "Playing",
  beat: "Beat",
  quit: "Quit",
  paused: "Paused",
  free: "Free",
};

// Tags are an orthogonal dimension to status: a game can be e.g. both "Beat"
// and "Coop". Free-form in the data, but these are the built-in ones.
export type Tag = "coop" | "casual";

export const TAGS: Tag[] = ["coop", "casual"];

export const TAG_LABELS: Record<Tag, string> = {
  coop: "Coop",
  casual: "Casual",
};

export type StoreId = "steam" | "gog" | "epic" | "ign";

export interface Sources {
  steam?: { appid: number };
  gog?: { id: string };
  epic?: { id: string };
  ign?: { id: string };
}

export interface StatusEvent {
  status: Status;
  at: string; // ISO-8601
}

export interface Game {
  id: string;
  title: string;
  coverUrl?: string; // box art / header image from whichever source has one
  sources: Sources;
  status: Status;
  // Date the CURRENT status was set. Undefined when unknown — e.g. imported
  // games, where the source doesn't tell us when the status was chosen.
  statusChangedAt?: string; // ISO-8601
  statusHistory: StatusEvent[];
  playtimeMinutes?: number; // undefined = unknown (most sources don't expose it)
  storeRating?: number; // 0-100, the source store's own rating
  metacritic?: number; // 0-100
  tags?: string[]; // orthogonal labels, e.g. "coop", "casual"
  addedAt: string; // ISO-8601
  lastSyncedAt?: string; // ISO-8601
}

export const LIBRARY_VERSION = 1;

export interface Library {
  version: number;
  updatedAt: string; // ISO-8601, used for last-write-wins across devices
  games: Game[];
}

export interface Settings {
  steamApiKey?: string;
  steamId?: string;
  gogRefreshToken?: string;
  epicRefreshToken?: string;
  ignNickname?: string;
  lastLibraryPath?: string;
}

export function emptyLibrary(): Library {
  return {
    version: LIBRARY_VERSION,
    updatedAt: new Date().toISOString(),
    games: [],
  };
}
