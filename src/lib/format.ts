import type { Game, StoreId } from "./types";

export function formatPlaytime(minutes: number | undefined): string {
  if (!minutes) return "—";
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  if (h === 0) return `${m}m`;
  if (m === 0) return `${h}h`;
  return `${h}h ${m}m`;
}

/** Allkeyshop "buy / compare prices" deep link for a game title. */
export function allkeyshopUrl(title: string): string {
  const slug = title
    .toLowerCase()
    .replace(/['’]/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return `https://www.allkeyshop.com/blog/buy-${slug}-cd-key-compare-prices/`;
}

/** GG.deals price-comparison link. Title search, not a /game/<slug>/ deep
 *  link: their slugs aren't reliably derivable from titles, and a search hit
 *  for an exact title lands on the game page anyway. */
export function ggdealsUrl(title: string): string {
  return `https://gg.deals/games/?title=${encodeURIComponent(title)}`;
}

export interface StoreLink {
  store: StoreId;
  label: string;
  url: string;
}

/**
 * Store-page links for a game, one per source it came from. Steam has a direct
 * app URL; GOG/Epic don't expose a page URL in their sync data, so those go to
 * a store search for the title. Wishlisted games always get a Steam link (the
 * store the user buys PC keys for), falling back to a Steam search when the
 * game isn't wishlisted on Steam itself.
 */
export function storeLinks(game: Game): StoreLink[] {
  const links: StoreLink[] = [];
  const q = encodeURIComponent(game.title);
  if (game.sources.steam) {
    links.push({
      store: "steam",
      label: "Steam",
      url: `https://store.steampowered.com/app/${game.sources.steam.appid}/`,
    });
  } else if (game.status === "wishlist") {
    links.push({
      store: "steam",
      label: "Find on Steam",
      url: `https://store.steampowered.com/search/?term=${q}`,
    });
  }
  if (game.sources.gog) {
    links.push({ store: "gog", label: "GOG", url: `https://www.gog.com/en/games?query=${q}` });
  }
  if (game.sources.epic) {
    links.push({
      store: "epic",
      label: "Epic",
      url: `https://store.epicgames.com/en-US/browse?q=${q}&sortBy=relevancy`,
    });
  }
  return links;
}

/**
 * Rating to display for a game: the store's own rating, falling back to
 * Metacritic. A 0 means "no reviews yet" (e.g. unreleased), not a real score,
 * so it counts as unknown.
 */
export function gameRating(game: Game): number | undefined {
  return game.storeRating || game.metacritic || undefined;
}

export function formatDate(iso: string | undefined): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "—";
  return d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}
