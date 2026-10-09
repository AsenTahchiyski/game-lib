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

// Everything that can be toggled off in Settings: the store integrations plus
// the external price-check links. Off = the source and its data disappear from
// the whole app (badges, links, sync sections, games it exclusively provides).
export type SourceToggle = StoreId | "allkeyshop" | "ggdeals";

export const SOURCE_TOGGLES: SourceToggle[] = [
  "steam",
  "gog",
  "epic",
  "ign",
  "allkeyshop",
  "ggdeals",
];

export const SOURCE_TOGGLE_LABELS: Record<SourceToggle, string> = {
  steam: "Steam",
  gog: "GOG",
  epic: "Epic",
  ign: "IGN",
  allkeyshop: "Allkeyshop",
  ggdeals: "GG.deals",
};

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

// HowLongToBeat completion times, in minutes (like playtimeMinutes). All three
// missing = HLTB was checked but had no entry; checkedAt records when, so a
// future manual refresh could re-query.
export interface HltbTimes {
  main?: number;
  extra?: number;
  completionist?: number;
  checkedAt: string; // ISO-8601
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
  releaseDate?: string; // ISO-8601 date (yyyy-mm-dd); undefined = unknown
  hltb?: HltbTimes; // HowLongToBeat times; undefined = never fetched
  tags?: string[]; // orthogonal labels, e.g. "coop", "casual"
  userEdited?: boolean; // user changed status/title/etc — protect from sync overwrite
  unreviewed?: true; // added by a sync; the user hasn't picked its status yet
  addedAt: string; // ISO-8601
  lastSyncedAt?: string; // ISO-8601
}

export const LIBRARY_VERSION = 1;

export interface Library {
  version: number;
  updatedAt: string; // ISO-8601, used for last-write-wins across devices
  games: Game[];
  customTags?: string[]; // user-created tags, in addition to the built-in TAGS
  removed?: string[]; // tombstones ("steam:123", "title:<norm>") to skip on sync
}

export interface Settings {
  steamApiKey?: string;
  steamId?: string;
  gogRefreshToken?: string;
  epicRefreshToken?: string;
  ignNickname?: string;
  lastLibraryPath?: string;
  viewMode?: "list" | "grid";
  // Sources the user toggled OFF in Settings (default: all enabled). Stored as
  // a deny-list so newly added sources default to on.
  disabledSources?: SourceToggle[];
}

export function emptyLibrary(): Library {
  return {
    version: LIBRARY_VERSION,
    updatedAt: new Date().toISOString(),
    games: [],
  };
}
