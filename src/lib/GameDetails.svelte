<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import {
    app,
    setStatus,
    removeGame,
    toggleTag,
    renameGame,
    setCover,
    availableTags,
    addCustomTag,
    deleteCustomTag,
    mergeRecords,
  } from "./store.svelte";
  import {
    STATUSES,
    STATUS_LABELS,
    TAGS,
    TAG_LABELS,
    type Status,
    type Tag,
    type Game,
  } from "./types";
  import { formatPlaytime, formatDate, allkeyshopUrl } from "./format";

  let { game, onclose }: { game: Game; onclose: () => void } = $props();

  let editingTitle = $state(false);
  let titleDraft = $state("");
  let coverDraft = $state("");
  let newTag = $state("");
  let mergeQuery = $state("");

  const BUILTIN: string[] = TAGS;
  const tagLabel = (t: string) => TAG_LABELS[t as Tag] ?? t;

  const mergeCandidates = $derived(
    mergeQuery.trim()
      ? app.library.games
          .filter(
            (g) => g.id !== game.id && g.title.toLowerCase().includes(mergeQuery.toLowerCase()),
          )
          .slice(0, 8)
      : [],
  );

  function createTag() {
    addCustomTag(newTag, game);
    newTag = "";
  }
  function doMerge(other: Game) {
    mergeRecords(game, other);
    mergeQuery = "";
  }
  // Pre-fill / reset the cover draft from the current game (and when it changes).
  $effect(() => {
    coverDraft = game.coverUrl ?? "";
  });

  // Only history entries with a real recorded date — import artifacts are dropped.
  const datedHistory = $derived(game.statusHistory.filter((e) => !!e.at));

  function startRename() {
    titleDraft = game.title;
    editingTitle = true;
  }
  function commitRename() {
    renameGame(game, titleDraft);
    editingTitle = false;
  }

  async function handleRemove() {
    const ok = await confirm(`Remove "${game.title}" from your library?`, {
      title: "Remove game",
      kind: "warning",
    });
    if (ok) {
      removeGame(game);
      onclose();
    }
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="modal" role="dialog" aria-modal="true" tabindex="-1">
    <div class="head">
      <div class="cover">
        {#if game.coverUrl}
          <img
            src={game.coverUrl}
            alt=""
            onerror={(e) => ((e.currentTarget as HTMLImageElement).style.display = "none")}
          />
        {/if}
      </div>
      <div class="head-info">
        {#if editingTitle}
          <input
            class="title-edit"
            bind:value={titleDraft}
            onkeydown={(e) => {
              if (e.key === "Enter") commitRename();
              if (e.key === "Escape") editingTitle = false;
            }}
          />
          <div class="title-actions">
            <button class="primary" onclick={commitRename}>Save</button>
            <button onclick={() => (editingTitle = false)}>Cancel</button>
          </div>
        {:else}
          <h2>{game.title} <button class="edit" title="Rename" onclick={startRename}>✎</button></h2>
        {/if}
        <div class="sources">
          {#each Object.keys(game.sources) as src}
            <span class="src">{src}</span>
          {/each}
        </div>
        <dl class="stats">
          <div><dt>Playtime</dt><dd>{formatPlaytime(game.playtimeMinutes)}</dd></div>
          <div><dt>Since</dt><dd>{formatDate(game.statusChangedAt)}</dd></div>
          <div><dt>Rating</dt><dd>{game.storeRating ?? "—"}</dd></div>
          <div><dt>Metacritic</dt><dd>{game.metacritic ?? "—"}</dd></div>
        </dl>
      </div>
    </div>

    <h3>Status</h3>
    <div class="chips">
      {#each STATUSES as s}
        <button
          class="st-{s}"
          class:active={game.status === s}
          onclick={() => setStatus(game, s)}
        >
          {STATUS_LABELS[s]}
        </button>
      {/each}
    </div>

    <h3>Tags</h3>
    <div class="chips">
      {#each availableTags() as t}
        <span class="tag-wrap">
          <button class:active={(game.tags ?? []).includes(t)} onclick={() => toggleTag(game, t)}>
            {tagLabel(t)}
          </button>
          {#if !BUILTIN.includes(t)}
            <button class="tag-del" title="Delete tag" onclick={() => deleteCustomTag(t)}>×</button>
          {/if}
        </span>
      {/each}
    </div>
    <div class="inline-add">
      <input bind:value={newTag} placeholder="New tag…" onkeydown={(e) => e.key === "Enter" && createTag()} />
      <button onclick={createTag} disabled={!newTag.trim()}>Add tag</button>
    </div>

    <h3>Cover image</h3>
    <div class="cover-edit">
      <input bind:value={coverDraft} placeholder="Paste an image URL…" />
      <button onclick={() => setCover(game, coverDraft)}>Set</button>
    </div>

    {#if game.status === "wishlist"}
      <h3>Wishlist</h3>
      <button class="full" onclick={() => openUrl(allkeyshopUrl(game.title))}>
        Check PC Steam key prices on Allkeyshop ↗
      </button>
    {/if}

    <h3>Merge another record into this one</h3>
    <div class="inline-add">
      <input bind:value={mergeQuery} placeholder="Find a game by title…" />
    </div>
    {#if mergeCandidates.length > 0}
      <ul class="merge-list">
        {#each mergeCandidates as g}
          <li>
            <button onclick={() => doMerge(g)}>
              {g.title}
              <span class="muted">{Object.keys(g.sources).join(", ") || "manual"}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}

    {#if datedHistory.length > 0}
      <h3>History</h3>
      <ul class="history">
        {#each datedHistory as e}
          <li><span class="st-{e.status}">{STATUS_LABELS[e.status as Status] ?? e.status}</span>
            <span class="muted">{formatDate(e.at)}</span></li>
        {/each}
      </ul>
    {/if}

    <div class="actions">
      <button class="danger" onclick={handleRemove}>Remove from library</button>
      <button class="primary" onclick={onclose}>Close</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .modal {
    background: #21242b;
    border: 1px solid #3a3e48;
    border-radius: 12px;
    padding: 26px;
    width: 680px;
    max-width: 94vw;
    max-height: 90vh;
    overflow-y: auto;
  }
  .head {
    display: flex;
    gap: 20px;
  }
  .cover {
    flex: none;
    width: 150px;
    height: 210px;
    border-radius: 6px;
    background: #14161a;
    border: 1px solid #2c2f37;
    overflow: hidden;
  }
  .cover img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    display: block;
  }
  .head-info {
    min-width: 0;
  }
  h2 {
    margin: 0 0 8px;
    font-size: 20px;
  }
  .edit {
    background: none;
    border: none;
    color: #8b909a;
    cursor: pointer;
    font-size: 14px;
  }
  .edit:hover {
    color: #e6e6e6;
  }
  .title-edit {
    width: 100%;
    box-sizing: border-box;
    background: #14161a;
    border: 1px solid #3a3e48;
    border-radius: 7px;
    padding: 8px 10px;
    color: #e6e6e6;
    font-size: 18px;
  }
  .title-actions {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }
  .title-actions button {
    border-radius: 6px;
    padding: 5px 12px;
    font-size: 12px;
    border: 1px solid #3a3e48;
    background: #2c2f37;
    color: #e6e6e6;
  }
  .title-actions .primary {
    background: #5865f2;
    border-color: #5865f2;
    color: #fff;
  }
  .cover-edit {
    display: flex;
    gap: 8px;
  }
  .cover-edit input {
    flex: 1;
    min-width: 0;
    background: #14161a;
    border: 1px solid #3a3e48;
    border-radius: 7px;
    padding: 8px 10px;
    color: #e6e6e6;
    font-size: 13px;
  }
  .cover-edit button {
    border: 1px solid #3a3e48;
    background: #2c2f37;
    color: #e6e6e6;
    border-radius: 7px;
    padding: 0 14px;
    font-size: 13px;
    cursor: pointer;
  }
  h3 {
    margin: 18px 0 8px;
    font-size: 13px;
    color: #8b909a;
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
  .stats {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px 14px;
    margin: 12px 0 0;
  }
  .stats div {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    font-size: 13px;
  }
  .stats dt {
    color: #8b909a;
    white-space: nowrap;
  }
  .stats dd {
    margin: 0;
    font-weight: 600;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chips button {
    background: #2c2f37;
    color: #e6e6e6;
    border: 1px solid #3a3e48;
    border-radius: 7px;
    padding: 4px 10px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .chips button.active {
    border-color: currentColor;
    background: #14161a;
  }
  .tag-wrap {
    display: inline-flex;
    align-items: center;
  }
  .tag-del {
    background: none;
    border: none;
    color: #8b909a;
    cursor: pointer;
    font-size: 14px;
    padding: 0 2px 0 4px;
    margin-left: -2px;
  }
  .tag-del:hover {
    color: #ffb4b4;
  }
  .inline-add {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }
  .inline-add input {
    flex: 1;
    min-width: 0;
    background: #14161a;
    border: 1px solid #3a3e48;
    border-radius: 7px;
    padding: 7px 10px;
    color: #e6e6e6;
    font-size: 13px;
  }
  .inline-add button {
    border: 1px solid #3a3e48;
    background: #2c2f37;
    color: #e6e6e6;
    border-radius: 7px;
    padding: 0 12px;
    font-size: 13px;
    cursor: pointer;
  }
  .inline-add button:disabled {
    opacity: 0.5;
  }
  .merge-list {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    border: 1px solid #3a3e48;
    border-radius: 8px;
    overflow: hidden;
  }
  .merge-list button {
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    border-bottom: 1px solid #2c2f37;
    color: #e6e6e6;
    padding: 8px 12px;
    font-size: 13px;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    gap: 10px;
  }
  .merge-list button:hover {
    background: #2c2f37;
  }
  .history {
    margin: 0;
    padding-left: 16px;
    font-size: 13px;
  }
  .history li {
    margin-bottom: 2px;
  }
  .muted {
    color: #8b909a;
    margin-left: 6px;
  }
  .actions {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    margin-top: 20px;
  }
  button {
    cursor: pointer;
  }
  .full {
    width: 100%;
    background: #2c2f37;
    color: #e6e6e6;
    border: 1px solid #3a3e48;
    border-radius: 7px;
    padding: 9px 14px;
    font-size: 13px;
  }
  .actions button {
    border-radius: 7px;
    padding: 8px 14px;
    font-size: 13px;
    border: 1px solid #3a3e48;
    background: #2c2f37;
    color: #e6e6e6;
  }
  .actions .primary {
    background: #5865f2;
    border-color: #5865f2;
    color: #fff;
  }
  .actions .danger {
    background: #4a1f1f;
    border-color: #6b2b2b;
    color: #ffb4b4;
  }
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
</style>
