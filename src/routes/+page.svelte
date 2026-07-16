<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import {
    app,
    init,
    openLibrary,
    saveLibrary,
    saveLibraryAs,
    undoChanges,
    checkForUpdate,
    setStatus,
    availableTags,
    sourceEnabled,
    gameVisible,
  } from "$lib/store.svelte";
  import { STATUSES, STATUS_LABELS, TAG_LABELS, type Tag, type Status, type Game } from "$lib/types";
  import { formatPlaytime, formatDate, allkeyshopUrl, ggdealsUrl, gameRating, releaseUrl } from "$lib/format";
  import Settings from "$lib/Settings.svelte";
  import GameDetails from "$lib/GameDetails.svelte";
  import AddGame from "$lib/AddGame.svelte";
  import StoreIcon from "$lib/StoreIcon.svelte";
  import RatingIcon from "$lib/RatingIcon.svelte";
  import { coverFallback } from "$lib/cover";

  type SortKey = "title" | "status" | "playtime" | "released" | "rating";
  const SORT_LABELS: Record<SortKey, string> = {
    title: "Title",
    status: "Status",
    playtime: "Playtime",
    released: "Released",
    rating: "Rating",
  };
  const SOURCE_IDS = ["steam", "gog", "epic", "ign"] as const;

  let search = $state("");
  let statusFilter = $state<Status | "all">("all");
  let viewMode = $state<"list" | "grid">("list");
  let coverScale = $state(1);
  let sortKey = $state<SortKey>("title");
  let sortAsc = $state(true);
  let sourceFilter = $state(new Set<string>());
  let openMenu = $state<string | null>(null);
  let openStatusFor = $state<string | null>(null);
  let tagFilter = $state(new Set<string>());
  let showSettings = $state(false);
  let showAdd = $state(false);
  let selectedGame = $state<Game | null>(null);
  let version = $state("");

  onMount(async () => {
    await init();
    version = await getVersion();
    // Best-effort update check; offline or rate-limited is just "no banner".
    checkForUpdate(version).catch(() => {});
  });

  function compareBy(a: Game, b: Game, key: SortKey): number {
    switch (key) {
      case "title":
        return a.title.localeCompare(b.title);
      case "status":
        return STATUSES.indexOf(a.status) - STATUSES.indexOf(b.status);
      case "playtime":
        return (a.playtimeMinutes ?? -1) - (b.playtimeMinutes ?? -1);
      case "released":
        return (Date.parse(a.releaseDate ?? "") || 0) - (Date.parse(b.releaseDate ?? "") || 0);
      case "rating":
        return (gameRating(a) ?? -1) - (gameRating(b) ?? -1);
    }
  }

  // Stores that aren't toggled off in Settings; disabled ones vanish from the
  // whole page (badges, sources column, and games they exclusively provide).
  const enabledStores = $derived(SOURCE_IDS.filter(sourceEnabled));
  const visibleGames = $derived(app.library.games.filter(gameVisible));

  const filtered = $derived.by(() => {
    const q = search.toLowerCase();
    const list = visibleGames.filter(
      (g) =>
        (statusFilter === "all" || g.status === statusFilter) &&
        g.title.toLowerCase().includes(q) &&
        (sourceFilter.size === 0 ||
          enabledStores.some(
            (s) => sourceFilter.has(s) && (g.sources as Record<string, unknown>)[s],
          )) &&
        (tagFilter.size === 0 || (g.tags ?? []).some((t) => tagFilter.has(t))),
    );
    const dir = sortAsc ? 1 : -1;
    return list.sort((a, b) => dir * compareBy(a, b, sortKey));
  });

  // Count of games per status, for the filter chips.
  const counts = $derived.by(() => {
    const c: Record<string, number> = { all: visibleGames.length };
    for (const s of STATUSES) c[s] = 0;
    for (const g of visibleGames) c[g.status]++;
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
    } else if (result === "no-access") {
      const repick = await confirm(
        "The app lost write access to the library file — Android revokes it after the app is reinstalled or updated.\n\nRe-select the file now to restore access? (Pick the same file; nothing is lost.)",
        { title: "File access lost", kind: "warning" },
      );
      if (repick) await saveLibraryAs();
    }
  }

  // ---- Autosave with an undo window ----------------------------------------
  // Every library change opens (or refreshes) a 10 s window announced by a
  // toast; when it lapses the library autosaves. Undo restores the state of
  // the last save/load. A brand-new library (no file yet) still uses the
  // explicit Save button instead.
  const UNDO_SECONDS = 10;
  let toast = $state<"none" | "pending" | "saved">("none");
  let undoSecs = $state(UNDO_SECONDS);
  let undoTicker: ReturnType<typeof setInterval> | undefined;
  let savedTimer: ReturnType<typeof setTimeout> | undefined;

  let lastSeq = 0;
  $effect(() => {
    const seq = app.changeSeq;
    // Only a NEW change opens the window — not a re-run from the path or
    // dirty flag changing (e.g. after open/save-as, or an undo).
    if (seq === 0 || seq === lastSeq || !app.currentPath || !app.dirty) return;
    lastSeq = seq;
    startUndoWindow();
  });

  function startUndoWindow() {
    clearInterval(undoTicker);
    clearTimeout(savedTimer);
    toast = "pending";
    undoSecs = UNDO_SECONDS;
    undoTicker = setInterval(() => {
      undoSecs--;
      if (undoSecs <= 0) {
        clearInterval(undoTicker);
        toast = "none";
        void autosaveNow();
      }
    }, 1000);
  }

  function undoNow() {
    clearInterval(undoTicker);
    toast = "none";
    selectedGame = null; // it may point at a game the undo just rewrote
    undoChanges();
  }

  async function autosaveNow() {
    if (!app.dirty) return; // e.g. a different file was opened mid-window
    await handleSave();
    if (!app.dirty) {
      toast = "saved";
      savedTimer = setTimeout(() => {
        if (toast === "saved") toast = "none";
      }, 2500);
    }
  }

  function chooseStatus(game: Game, s: Status) {
    setStatus(game, s);
    openStatusFor = null;
  }

  function toggleTagFilter(t: string) {
    const next = new Set(tagFilter);
    if (next.has(t)) next.delete(t);
    else next.add(t);
    tagFilter = next;
  }

  function openAllkeyshop(game: Game) {
    openUrl(allkeyshopUrl(game.title));
  }

  function openGgdeals(game: Game) {
    openUrl(ggdealsUrl(game.title));
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
  // The active sort column shows only its direction triangle (and is bolded
  // via .sorted); inactive columns show the menu caret.
  function headArrow(key: SortKey): string {
    return sortKey === key ? (sortAsc ? " ▲" : " ▼") : " ▾";
  }
  function onWindowClick(e: MouseEvent) {
    const t = e.target as Element;
    if (openMenu && !t.closest(".col-head")) openMenu = null;
    if (openStatusFor && !t.closest(".status-dd")) openStatusFor = null;
  }
</script>

<svelte:window onclick={onWindowClick} />

{#snippet statusControl(game: Game)}
  <div class="status-dd">
    <button
      class="status-btn st-{game.status}"
      onclick={() => (openStatusFor = openStatusFor === game.id ? null : game.id)}
    >
      {STATUS_LABELS[game.status]} ▾
    </button>
    {#if openStatusFor === game.id}
      <div class="status-menu">
        {#each STATUSES as s}
          <button class="st-{s}" onclick={() => chooseStatus(game, s)}>{STATUS_LABELS[s]}</button>
        {/each}
      </div>
    {/if}
  </div>
{/snippet}

{#snippet storeBadges(game: Game)}
  {#each Object.keys(game.sources).filter(sourceEnabled) as src}
    <span class="store-badge" title={src}><StoreIcon store={src} size={13} /></span>
  {/each}
{/snippet}

<div class="app">
  <header>
    <div class="title">
      <strong>Game Library</strong>
      {#if version}<span class="ver">v{version}</span>{/if}
      {#if app.dirty}<span class="dirty" title="Unsaved changes">•</span>{/if}
      <span class="path">{app.currentPath ?? "No file"}</span>
    </div>
    <div class="actions">
      <!-- With a file loaded, autosave replaces Save and switching files
           moves to Settings → Library file; the header stays minimal. -->
      {#if !app.currentPath}
        <button onclick={openLibrary}>Open…</button>
      {/if}
      <button onclick={() => (showAdd = true)}>+ Add</button>
      {#if !app.currentPath}
        <button class="primary" onclick={handleSave} disabled={app.busy}>Save</button>
      {/if}
      <button onclick={() => (showSettings = true)}>Settings</button>
    </div>
  </header>

  {#if app.error}
    <div class="banner error">
      {app.error}
      <button class="dismiss" title="Dismiss" onclick={() => (app.error = null)}>×</button>
    </div>
  {/if}

  {#if app.updateAvailable}
    <div class="banner update">
      Version {app.updateAvailable} is available (you have {version}).
      <button class="get" onclick={() => openUrl(releaseUrl())}>Download ↗</button>
      <button class="dismiss" title="Dismiss" onclick={() => (app.updateAvailable = null)}>×</button>
    </div>
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
    <div class="chips tag-chips">
      {#each availableTags() as t}
        <button class:active={tagFilter.has(t)} onclick={() => toggleTagFilter(t)}>
          {TAG_LABELS[t as Tag] ?? t}
        </button>
      {/each}
    </div>
    {#if viewMode === "list"}
      <input
        class="scale"
        type="range"
        min="0.6"
        max="2.5"
        step="0.1"
        bind:value={coverScale}
        title="Row image size"
      />
    {:else}
      <!-- The list view sorts via its column headers; the grid has no headers,
           so it gets an explicit sort control. -->
      <div class="sort-ctl">
        <span class="sort-label">Sort</span>
        <select bind:value={sortKey}>
          {#each Object.entries(SORT_LABELS) as [key, label]}
            <option value={key}>{label}</option>
          {/each}
        </select>
        <button title="Toggle sort direction" onclick={() => (sortAsc = !sortAsc)}>
          {sortAsc ? "▲" : "▼"}
        </button>
      </div>
    {/if}
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
            <button class="card-cover coverbtn" onclick={() => (selectedGame = game)}>
              {#if game.coverUrl}
                {#key game.coverUrl}
                  <img
                    src={game.coverUrl}
                    alt=""
                    loading="lazy"
                    onerror={coverFallback}
                  />
                {/key}
              {/if}
              {#if gameRating(game) !== undefined}
                <span class="card-rating"><RatingIcon rating={gameRating(game)} /></span>
              {/if}
            </button>
            <button class="card-title title-link" title={game.title} onclick={() => (selectedGame = game)}>
              {@render storeBadges(game)}{game.title}
            </button>
            {@render statusControl(game)}
          </div>
        {/each}
      </div>
    {:else}
      <table style="--cover-scale: {coverScale}">
        <thead>
          <tr>
            <th>
              <div class="col-head">
                <button class="hbtn" class:sorted={sortKey === "title"} onclick={() => toggleMenu("title")}>
                  Title{headArrow("title")}
                </button>
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
                <button class="hbtn" class:sorted={sortKey === "status"} onclick={() => toggleMenu("status")}>
                  Status{headArrow("status")}
                </button>
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
            <th class="opt">
              <div class="col-head">
                <button class="hbtn" class:sorted={sortKey === "playtime"} onclick={() => toggleMenu("playtime")}>
                  Playtime{headArrow("playtime")}
                </button>
                {#if openMenu === "playtime"}
                  <div class="menu">
                    <button onclick={() => setSort("playtime", false)}>Most first</button>
                    <button onclick={() => setSort("playtime", true)}>Least first</button>
                  </div>
                {/if}
              </div>
            </th>
            <th class="opt">
              <div class="col-head">
                <button class="hbtn" class:sorted={sortKey === "released"} onclick={() => toggleMenu("released")}>
                  Released{headArrow("released")}
                </button>
                {#if openMenu === "released"}
                  <div class="menu">
                    <button onclick={() => setSort("released", false)}>Newest first</button>
                    <button onclick={() => setSort("released", true)}>Oldest first</button>
                  </div>
                {/if}
              </div>
            </th>
            <th class="opt">
              <div class="col-head">
                <button class="hbtn" class:sorted={sortKey === "rating"} onclick={() => toggleMenu("rating")}>
                  Rating{headArrow("rating")}
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
                <button class="hbtn" onclick={() => toggleMenu("sources")}>
                  Sources{sourceFilter.size ? ` (${sourceFilter.size})` : ""} ▾
                </button>
                {#if openMenu === "sources"}
                  <div class="menu">
                    {#each enabledStores as s}
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
            <tr class="row-{game.status}">
              <td class="t-title">
                <div class="title-cell">
                  <button class="cover coverbtn" onclick={() => (selectedGame = game)}>
                    {#if game.coverUrl}
                      {#key game.coverUrl}
                        <img
                          src={game.coverUrl}
                          alt=""
                          loading="lazy"
                          onerror={coverFallback}
                        />
                      {/key}
                    {/if}
                  </button>
                  <button class="title-link" onclick={() => (selectedGame = game)}>{game.title}</button>
                  {#if game.status === "wishlist"}
                    {#if sourceEnabled("allkeyshop")}
                      <button class="aks" title="Key prices on Allkeyshop" onclick={() => openAllkeyshop(game)}>💰</button>
                    {/if}
                    {#if sourceEnabled("ggdeals")}
                      <button class="aks" title="Deals on GG.deals" onclick={() => openGgdeals(game)}>🏷️</button>
                    {/if}
                  {/if}
                  <span class="title-rating"><RatingIcon rating={gameRating(game)} /></span>
                </div>
              </td>
              <td>{@render statusControl(game)}</td>
              <td class="muted opt">{formatPlaytime(game.playtimeMinutes)}</td>
              <td class="muted opt">{formatDate(game.releaseDate)}</td>
              <td class="muted opt">{gameRating(game) ?? "—"}</td>
              <td class="sources">
                {#each Object.keys(game.sources).filter(sourceEnabled) as src}
                  <span class="src-icon" title={src}><StoreIcon store={src} /></span>
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

{#if showAdd}
  <AddGame onclose={() => (showAdd = false)} />
{/if}

{#if selectedGame}
  <GameDetails game={selectedGame} onclose={() => (selectedGame = null)} />
{/if}

{#if toast !== "none"}
  <div class="toast" role="status">
    {#if toast === "pending"}
      <span>Autosaving in {undoSecs}s…</span>
      <button class="undo" onclick={undoNow}>Undo</button>
    {:else}
      <span>Saved ✓</span>
    {/if}
  </div>
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
    /* dvh (not vh) so the layout matches the *visible* viewport on Android,
       which excludes the system status/navigation bars. */
    height: 100dvh;
    display: flex;
    flex-direction: column;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    /* Pad past the status bar / notch so the header isn't drawn under it
       (env() insets are non-zero thanks to viewport-fit=cover). */
    padding: calc(10px + env(safe-area-inset-top)) calc(16px + env(safe-area-inset-right))
      10px calc(16px + env(safe-area-inset-left));
    border-bottom: 1px solid #2c2f37;
    background: #21242b;
  }
  .title {
    display: flex;
    align-items: baseline;
    /* Let the title shrink and the path ellipsis instead of shoving the
       action buttons off the right edge. */
    min-width: 0;
    flex: 1;
  }
  .title strong {
    font-size: 15px;
    flex: none;
  }
  .ver {
    margin-left: 6px;
    font-size: 11px;
    color: #6b7280;
    flex: none;
  }
  .dirty {
    margin-left: 6px;
    color: #5865f2;
    flex: none;
  }
  .path {
    margin-left: 10px;
    font-size: 12px;
    color: #8b909a;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .actions {
    display: flex;
    gap: 8px;
    flex: none;
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
  .banner.update {
    background: #1f2a4a;
    color: #c7d2fe;
    padding: 8px 16px;
    font-size: 13px;
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .banner .get {
    padding: 3px 10px;
    font-size: 12px;
  }
  .banner .dismiss {
    margin-left: auto;
    background: none;
    border: none;
    color: inherit;
    font-size: 16px;
    padding: 0 4px;
  }
  .toast {
    position: fixed;
    left: 50%;
    transform: translateX(-50%);
    bottom: calc(18px + env(safe-area-inset-bottom));
    z-index: 1100; /* above the modal overlays, so Undo works mid-edit */
    display: flex;
    align-items: center;
    gap: 12px;
    background: #2c2f37;
    border: 1px solid #3a3e48;
    border-radius: 10px;
    padding: 10px 16px;
    font-size: 13px;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.45);
  }
  .toast .undo {
    background: none;
    border: none;
    color: #aeb6ff;
    font-weight: 700;
    padding: 0;
  }
  .toast .undo:hover {
    text-decoration: underline;
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
  .scale {
    width: 90px;
    accent-color: #5865f2;
  }
  .sort-ctl {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .sort-label {
    font-size: 12px;
    color: #8b909a;
  }
  .sort-ctl select {
    background: #14161a;
    border: 1px solid #3a3e48;
    border-radius: 7px;
    padding: 4px 8px;
    color: #e6e6e6;
    font-size: 12px;
  }
  .sort-ctl button {
    padding: 4px 9px;
    font-size: 11px;
    line-height: 1.4;
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
    /* Keep the last row clear of the system navigation bar at the bottom. */
    padding: 0 calc(16px + env(safe-area-inset-right))
      calc(16px + env(safe-area-inset-bottom)) calc(16px + env(safe-area-inset-left));
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
  .hbtn.sorted {
    color: #e6e6e6;
    font-weight: 700;
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
    position: relative;
    aspect-ratio: 3 / 4;
    width: 100%;
    border-radius: 8px;
    background: #14161a;
    border: 1px solid #2c2f37;
    overflow: hidden;
  }
  .card-rating {
    position: absolute;
    top: 5px;
    right: 5px;
    display: inline-flex;
    padding: 3px;
    border-radius: 5px;
    background: rgba(20, 22, 26, 0.8);
  }
  .card-cover img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    display: block;
  }
  .card-title {
    display: block;
    width: 100%;
    font-size: 13px;
    font-weight: 700;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .card .status-dd,
  .card .status-btn {
    width: 100%;
    box-sizing: border-box;
  }
  .store-badge {
    display: inline-flex;
    vertical-align: middle;
    margin-right: 5px;
  }
  .src-icon {
    display: inline-flex;
    vertical-align: middle;
    margin-right: 5px;
  }
  .t-title {
    font-weight: 700;
  }
  .title-cell {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .coverbtn {
    padding: 0;
    display: block;
    cursor: pointer;
  }
  .title-link {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    color: #e6e6e6;
    font-family: inherit;
    font-size: inherit;
    font-weight: 700;
    text-align: left;
  }
  .title-link:hover {
    text-decoration: underline;
  }
  .aks {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 13px;
    padding: 0 2px;
    flex: none;
  }
  .title-rating {
    margin-left: auto;
    flex: none;
    display: inline-flex;
  }
  .cover {
    flex: none;
    width: calc(40px * var(--cover-scale, 1));
    height: calc(56px * var(--cover-scale, 1));
    border-radius: 4px;
    background: #14161a;
    border: 1px solid #2c2f37;
    overflow: hidden;
  }
  .cover img {
    width: 100%;
    height: 100%;
    /* contain, not cover: box art often embeds the title, which a crop slices */
    object-fit: contain;
    display: block;
  }
  .muted {
    color: #8b909a;
  }
  /* Custom status dropdown (replaces the native <select>, which mis-selected on
     click near the viewport bottom and couldn't be colour-coded reliably). */
  .status-dd {
    position: relative;
    display: inline-block;
  }
  .status-btn {
    background: #14161a;
    border: 1px solid #3a3e48;
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    text-align: left;
  }
  .status-btn:hover {
    border-color: #5865f2;
  }
  .status-menu {
    position: absolute;
    top: 100%;
    left: 0;
    z-index: 10;
    margin-top: 2px;
    min-width: 130px;
    background: #21242b;
    border: 1px solid #3a3e48;
    border-radius: 8px;
    padding: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.4);
  }
  .status-menu button {
    text-align: left;
    background: none;
    border: none;
    font-size: 13px;
    font-weight: 600;
    padding: 6px 10px;
    border-radius: 6px;
    cursor: pointer;
  }
  .status-menu button:hover {
    background: #2c2f37;
  }
  /* Status colour coding (used by the dropdown trigger, menu items, row accent). */
  .st-wishlist {
    color: #c084fc;
  }
  .st-backlog {
    color: #94a3b8;
  }
  .st-playing {
    color: #4ade80;
  }
  .st-beat {
    color: #60a5fa;
  }
  .st-quit {
    color: #f87171;
  }
  .st-paused {
    color: #fbbf24;
  }
  .st-free {
    color: #2dd4bf;
  }
  tbody tr td:first-child {
    border-left: 3px solid transparent;
  }
  .row-wishlist td:first-child {
    border-left-color: #c084fc;
  }
  .row-backlog td:first-child {
    border-left-color: #94a3b8;
  }
  .row-playing td:first-child {
    border-left-color: #4ade80;
  }
  .row-beat td:first-child {
    border-left-color: #60a5fa;
  }
  .row-quit td:first-child {
    border-left-color: #f87171;
  }
  .row-paused td:first-child {
    border-left-color: #fbbf24;
  }
  .row-free td:first-child {
    border-left-color: #2dd4bf;
  }

  /* Narrow screens (phones / the Android app): drop secondary columns and the
     file-path clutter so the core Title / Status / Sources list fits. */
  @media (max-width: 640px) {
    header {
      flex-wrap: wrap;
    }
    .actions {
      flex-wrap: wrap;
    }
    .path,
    .ver {
      display: none;
    }
    .opt {
      display: none;
    }
  }
</style>
