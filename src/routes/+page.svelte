<script lang="ts">
  import { onMount } from "svelte";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { app, init, openLibrary, newLibrary, saveLibrary, setStatus } from "$lib/store.svelte";
  import { STATUSES, STATUS_LABELS, type Status, type Game } from "$lib/types";
  import { formatPlaytime, formatDate } from "$lib/format";
  import Settings from "$lib/Settings.svelte";

  let search = $state("");
  let statusFilter = $state<Status | "all">("all");
  let showSettings = $state(false);

  onMount(init);

  const filtered = $derived(
    app.library.games
      .filter((g) => statusFilter === "all" || g.status === statusFilter)
      .filter((g) => g.title.toLowerCase().includes(search.toLowerCase()))
      .sort((a, b) => a.title.localeCompare(b.title)),
  );

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
</script>

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
    {:else}
      <table>
        <thead>
          <tr>
            <th>Title</th>
            <th>Status</th>
            <th>Since</th>
            <th>Playtime</th>
            <th>Sources</th>
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
    background: #1b1d22;
    color: #8b909a;
    font-weight: 500;
    font-size: 12px;
    padding: 10px 8px;
    border-bottom: 1px solid #2c2f37;
  }
  td {
    padding: 8px;
    border-bottom: 1px solid #23262d;
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
