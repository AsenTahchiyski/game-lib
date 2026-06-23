<script lang="ts">
  import { onMount } from "svelte";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { app, init, openLibrary, newLibrary, saveLibrary, setStatus } from "$lib/store.svelte";
  import { STATUSES, STATUS_LABELS, type Status, type Game } from "$lib/types";
  import { formatPlaytime, formatDate } from "$lib/format";
  import Settings from "$lib/Settings.svelte";

  type SortKey = "title" | "status" | "since" | "playtime" | "rating" | "metacritic";
  const SOURCE_IDS = ["steam", "gog", "epic", "ign"] as const;

  let search = $state("");
  let statusFilter = $state<Status | "all">("all");
  let viewMode = $state<"list" | "grid">("list");
  let sortKey = $state<SortKey>("title");
  let sortAsc = $state(true);
  let sourceFilter = $state(new Set<string>());
  let openMenu = $state<string | null>(null);
  let showSettings = $state(false);

  onMount(init);

  function compareBy(a: Game, b: Game, key: SortKey): number {
    switch (key) {
      case "title":
        return a.title.localeCompare(b.title);
      case "status":
        return STATUSES.indexOf(a.status) - STATUSES.indexOf(b.status);
      case "since":
        return (Date.parse(a.statusChangedAt ?? "") || 0) - (Date.parse(b.statusChangedAt ?? "") || 0);
      case "playtime":
        return (a.playtimeMinutes ?? -1) - (b.playtimeMinutes ?? -1);
      case "rating":
        return (a.storeRating ?? -1) - (b.storeRating ?? -1);
      case "metacritic":
        return (a.metacritic ?? -1) - (b.metacritic ?? -1);
    }
  }

  const filtered = $derived.by(() => {
    const q = search.toLowerCase();
    const list = app.library.games.filter(
      (g) =>
        (statusFilter === "all" || g.status === statusFilter) &&
        g.title.toLowerCase().includes(q) &&
        (sourceFilter.size === 0 ||
          SOURCE_IDS.some((s) => sourceFilter.has(s) && (g.sources as Record<string, unknown>)[s])),
    );
    const dir = sortAsc ? 1 : -1;
    return list.sort((a, b) => dir * compareBy(a, b, sortKey));
  });

  // Count of games per status, for the filter chips.
  const counts = $derived.by(() => {
    const c: Record<string, number> = { all: app.library.games.length };
    for (const s of STATUSES) c[s] = 0;
    for (const g of app.library.games) c[g.status]++;
    return c;
  });

  async function handleSave() {
    const result = await saveLibrary();
    if (result === "conflict") {
      const overwrite = await confirm(
        "The library file on disk is newer than what you have open — another device may have saved changes.\n\nOverwrite it with your version anyway?",
        { title: "Newer version on disk", kind: "warning" },
      );
      if (overwrite) await saveLibrary(true);
    }
  }

  function onStatusChange(game: Game, e: Event) {
    setStatus(game, (e.target as HTMLSelectElement).value as Status);
  }

  async function handleNew() {
    if (app.library.games.length > 0 || app.dirty) {
      const ok = await confirm(
        "Start a new, empty library? The games currently loaded will be cleared, and any unsaved changes lost.",
        { title: "New library", kind: "warning" },
      );
      if (!ok) return;
    }
    newLibrary();
  }

  function setSort(key: SortKey, asc: boolean) {
    sortKey = key;
    sortAsc = asc;
    openMenu = null;
  }
  function toggleMenu(id: string) {
    openMenu = openMenu === id ? null : id;
  }
  function pickStatus(s: Status | "all") {
    statusFilter = s;
    openMenu = null;
  }
  function toggleSource(s: string) {
    const next = new Set(sourceFilter);
    if (next.has(s)) next.delete(s);
    else next.add(s);
    sourceFilter = next;
  }
  function sortArrow(key: SortKey): string {
    return sortKey === key ? (sortAsc ? " ▲" : " ▼") : "";
  }
  function onWindowClick(e: MouseEvent) {
    if (openMenu && !(e.target as Element).closest(".col-head")) openMenu = null;
  }
</script>

<svelte:window onclick={onWindowClick} />

<div class="app">
  <header>
    <div class="title">
      <strong>Game Library</strong>
      <span class="path">
        {app.currentPath ?? "No file"}{app.dirty ? " •" : ""}
      </span>
    </div>
    <div class="actions">
      <button onclick={handleNew}>New</button>
      <button onclick={openLibrary}>Open…</button>
      <button class="primary" onclick={handleSave} disabled={app.busy}>Save</button>
      <button onclick={() => (showSettings = true)}>Settings</button>
    </div>
  </header>

  {#if app.error}
    <div class="banner error">{app.error}</div>
  {/if}

  <div class="filters">
    <input class="search" type="search" placeholder="Search games…" bind:value={search} />
    <div class="chips">
      <button class:active={statusFilter === "all"} onclick={() => (statusFilter = "all")}>
        All <span class="count">{counts.all}</span>
      </button>
      {#each STATUSES as s}
        <button class:active={statusFilter === s} onclick={() => (statusFilter = s)}>
          {STATUS_LABELS[s]} <span class="count">{counts[s]}</span>
        </button>
      {/each}
    </div>
    <div class="view-toggle">
      <button class:active={viewMode === "list"} title="List view" onclick={() => (viewMode = "list")}>
        ☰
      </button>
      <button class:active={viewMode === "grid"} title="Grid view" onclick={() => (viewMode = "grid")}>
        ▦
      </button>
    </div>
  </div>

  <main>
    {#if app.library.games.length === 0}
      <div class="empty">
        <p>No games yet.</p>
        <p class="hint">
          Open a library file, or use a store sync (Settings → Steam) to pull your games in.
        </p>
      </div>
    {:else if filtered.length === 0}
      <div class="empty"><p>No games match your filter.</p></div>
    {:else if viewMode === "grid"}
      <div class="grid">
        {#each filtered as game (game.id)}
          <div class="card">
            <div class="card-cover">
              {#if game.coverUrl}
                <img
                  src={game.coverUrl}
                  alt=""
                  loading="lazy"
                  onerror={(e) => ((e.currentTarget as HTMLImageElement).style.display = "none")}
                />
              {/if}
            </div>
            <div class="card-title" title={game.title}>{game.title}</div>
            <select
              class="status status-{game.status}"
              value={game.status}
              onchange={(e) => onStatusChange(game, e)}
            >
              {#each STATUSES as s}
                <option value={s}>{STATUS_LABELS[s]}</option>
              {/each}
            </select>
          </div>
        {/each}
      </div>
    {:else}
      <table>
        <thead>
          <tr>
            <th>
              <div class="col-head">
                <button class="hbtn" onclick={() => toggleMenu("title")}>Title{sortArrow("title")} ▾</button>
                {#if openMenu === "title"}
                  <div class="menu">
                    <button onclick={() => setSort("title", true)}>Sort A → Z</button>
                    <button onclick={() => setSort("title", false)}>Sort Z → A</button>
                  </div>
                {/if}
              </div>
            </th>
            <th>
              <div class="col-head">
                <button class="hbtn" onclick={() => toggleMenu("status")}>Status{sortArrow("status")} ▾</button>
                {#if openMenu === "status"}
                  <div class="menu">
                    <button onclick={() => setSort("status", true)}>Sort ↑</button>
                    <button onclick={() => setSort("status", false)}>Sort ↓</button>
                    <div class="menu-sep"></div>
                    <button class:sel={statusFilter === "all"} onclick={() => pickStatus("all")}>All</button>
                    {#each STATUSES as s}
                      <button class:sel={statusFilter === s} onclick={() => pickStatus(s)}>
                        {STATUS_LABELS[s]}
                      </button>
                    {/each}
                  </div>
                {/if}
              </div>
            </th>
            <th>
              <div class="col-head">
                <button class="hbtn" onclick={() => toggleMenu("since")}>Since{sortArrow("since")} ▾</button>
                {#if openMenu === "since"}
                  <div class="menu">
                    <button onclick={() => setSort("since", false)}>Newest first</button>
                    <button onclick={() => setSort("since", true)}>Oldest first</button>
                  </div>
                {/if}
              </div>
            </th>
            <th>
              <div class="col-head">
                <button class="hbtn" onclick={() => toggleMenu("playtime")}>
                  Playtime{sortArrow("playtime")} ▾
                </button>
                {#if openMenu === "playtime"}
                  <div class="menu">
                    <button onclick={() => setSort("playtime", false)}>Most first</button>
                    <button onclick={() => setSort("playtime", true)}>Least first</button>
                  </div>
                {/if}
              </div>
            </th>
            <th>
              <div class="col-head">
                <button class="hbtn" onclick={() => toggleMenu("rating")}>
                  Rating{sortArrow("rating")} ▾
                </button>
                {#if openMenu === "rating"}
                  <div class="menu">
                    <button onclick={() => setSort("rating", false)}>Highest first</button>
                    <button onclick={() => setSort("rating", true)}>Lowest first</button>
                  </div>
                {/if}
              </div>
            </th>
            <th>
              <div class="col-head">
                <button class="hbtn" onclick={() => toggleMenu("metacritic")}>
                  Metacritic{sortArrow("metacritic")} ▾
                </button>
                {#if openMenu === "metacritic"}
                  <div class="menu">
                    <button onclick={() => setSort("metacritic", false)}>Highest first</button>
                    <button onclick={() => setSort("metacritic", true)}>Lowest first</button>
                  </div>
                {/if}
              </div>
            </th>
            <th>
              <div class="col-head">
                <button class="hbtn" onclick={() => toggleMenu("sources")}>
                  Sources{sourceFilter.size ? ` (${sourceFilter.size})` : ""} ▾
                </button>
                {#if openMenu === "sources"}
                  <div class="menu">
                    {#each SOURCE_IDS as s}
                      <label class="check">
                        <input type="checkbox" checked={sourceFilter.has(s)} onchange={() => toggleSource(s)} />
                        {s}
                      </label>
                    {/each}
                    {#if sourceFilter.size}
                      <div class="menu-sep"></div>
                      <button onclick={() => (sourceFilter = new Set())}>Clear filter</button>
                    {/if}
                  </div>
                {/if}
              </div>
            </th>
          </tr>
        </thead>
        <tbody>
          {#each filtered as game (game.id)}
            <tr>
              <td class="t-title">
                <div class="title-cell">
                  <div class="cover">
                    {#if game.coverUrl}
                      <img
                        src={game.coverUrl}
                        alt=""
                        loading="lazy"
                        onerror={(e) => ((e.currentTarget as HTMLImageElement).style.display = "none")}
                      />
                    {/if}
                  </div>
                  <span>{game.title}</span>
                </div>
              </td>
              <td>
                <select
                  class="status status-{game.status}"
                  value={game.status}
                  onchange={(e) => onStatusChange(game, e)}
                >
                  {#each STATUSES as s}
                    <option value={s}>{STATUS_LABELS[s]}</option>
                  {/each}
                </select>
              </td>
              <td class="muted">{formatDate(game.statusChangedAt)}</td>
              <td class="muted">{formatPlaytime(game.playtimeMinutes)}</td>
              <td class="muted">{game.storeRating ?? "—"}</td>
              <td class="muted">{game.metacritic ?? "—"}</td>
              <td class="sources">
                {#each Object.keys(game.sources) as src}
                  <span class="src">{src}</span>
                {/each}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </main>
</div>

{#if showSettings}
  <Settings onclose={() => (showSettings = false)} />
{/if}

<style>
  :global(body) {
    margin: 0;
  }
  :global(:root) {
    font-family: Inter, system-ui, Avenir, Helvetica, Arial, sans-serif;
    color: #e6e6e6;
    background: #1b1d22;
    /* Render native form controls (dropdowns, scrollbars) in dark mode. */
    color-scheme: dark;
  }
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    border-bottom: 1px solid #2c2f37;
    background: #21242b;
  }
  .title strong {
    font-size: 15px;
  }
  .path {
    margin-left: 10px;
    font-size: 12px;
    color: #8b909a;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  button {
    background: #2c2f37;
    color: #e6e6e6;
    border: 1px solid #3a3e48;
    border-radius: 7px;
    padding: 6px 12px;
    font-size: 13px;
    cursor: pointer;
  }
  button:hover {
    border-color: #5865f2;
  }
  button.primary {
    background: #5865f2;
    border-color: #5865f2;
    color: #fff;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .banner.error {
    background: #4a1f1f;
    color: #ffb4b4;
    padding: 8px 16px;
    font-size: 13px;
  }
  .filters {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    padding: 12px 16px;
    border-bottom: 1px solid #2c2f37;
  }
  .search {
    flex: none;
    width: 220px;
    max-width: 100%;
    box-sizing: border-box;
    background: #14161a;
    border: 1px solid #3a3e48;
    border-radius: 7px;
    padding: 8px 12px;
    color: #e6e6e6;
    font-size: 13px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chips button {
    padding: 4px 10px;
    font-size: 12px;
  }
  .chips button.active {
    background: #5865f2;
    border-color: #5865f2;
    color: #fff;
  }
  .count {
    opacity: 0.7;
    font-size: 11px;
  }
  .view-toggle {
    margin-left: auto;
    display: flex;
    gap: 4px;
  }
  .view-toggle button {
    padding: 4px 9px;
    font-size: 14px;
    line-height: 1;
  }
  .view-toggle button.active {
    background: #5865f2;
    border-color: #5865f2;
    color: #fff;
  }
  main {
    flex: 1;
    overflow: auto;
    padding: 0 16px 16px;
  }
  .empty {
    text-align: center;
    color: #8b909a;
    margin-top: 18vh;
  }
  .empty .hint {
    font-size: 13px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 14px;
  }
  th {
    text-align: left;
    position: sticky;
    top: 0;
    z-index: 2;
    background: #1b1d22;
    color: #8b909a;
    font-weight: 500;
    font-size: 12px;
    padding: 6px 8px;
    border-bottom: 1px solid #2c2f37;
  }
  .col-head {
    position: relative;
    display: inline-block;
  }
  .hbtn {
    background: none;
    border: none;
    color: #8b909a;
    font: inherit;
    font-weight: 500;
    padding: 4px 2px;
    cursor: pointer;
  }
  .hbtn:hover {
    color: #e6e6e6;
  }
  .menu {
    position: absolute;
    top: 100%;
    left: 0;
    z-index: 10;
    min-width: 140px;
    background: #21242b;
    border: 1px solid #3a3e48;
    border-radius: 8px;
    padding: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.4);
  }
  .menu button,
  .menu .check {
    text-align: left;
    background: none;
    border: none;
    color: #e6e6e6;
    font-size: 13px;
    padding: 6px 10px;
    border-radius: 6px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .menu button:hover,
  .menu .check:hover {
    background: #2c2f37;
  }
  .menu button.sel {
    color: #aeb6ff;
  }
  .menu-sep {
    height: 1px;
    background: #3a3e48;
    margin: 4px 2px;
  }
  td {
    padding: 8px;
    border-bottom: 1px solid #23262d;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 14px;
    padding-top: 14px;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .card-cover {
    aspect-ratio: 3 / 4;
    width: 100%;
    border-radius: 8px;
    background: #14161a;
    border: 1px solid #2c2f37;
    overflow: hidden;
  }
  .card-cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .card-title {
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .card .status {
    width: 100%;
    box-sizing: border-box;
  }
  .t-title {
    font-weight: 500;
  }
  .title-cell {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .cover {
    flex: none;
    width: 40px;
    height: 56px;
    border-radius: 4px;
    background: #14161a;
    border: 1px solid #2c2f37;
    overflow: hidden;
  }
  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .muted {
    color: #8b909a;
  }
  .status {
    background: #14161a;
    color: #e6e6e6;
    border: 1px solid #3a3e48;
    border-radius: 6px;
    padding: 4px 6px;
    font-size: 13px;
  }
  /* Make the opened dropdown list dark too (some platforms render the popup
     with a white background and light text otherwise). */
  .status option {
    background: #14161a;
    color: #e6e6e6;
  }
  .sources .src {
    display: inline-block;
    background: #2c2f37;
    border-radius: 4px;
    padding: 1px 6px;
    font-size: 11px;
    color: #b9bdc7;
    margin-right: 4px;
    text-transform: capitalize;
  }
</style>
