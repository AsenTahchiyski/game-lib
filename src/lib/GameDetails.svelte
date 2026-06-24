<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { setStatus, removeGame, toggleTag } from "./store.svelte";
  import {
    STATUSES,
    STATUS_LABELS,
    TAGS,
    TAG_LABELS,
    type Status,
    type Game,
  } from "./types";
  import { formatPlaytime, formatDate } from "./format";

  let { game, onclose }: { game: Game; onclose: () => void } = $props();

  const allkeyshopUrl = $derived(
    `https://www.allkeyshop.com/blog/?s=${encodeURIComponent(game.title)}`,
  );

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
        <h2>{game.title}</h2>
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
      {#each TAGS as t}
        <button class:active={(game.tags ?? []).includes(t)} onclick={() => toggleTag(game, t)}>
          {TAG_LABELS[t]}
        </button>
      {/each}
    </div>

    {#if game.status === "wishlist"}
      <h3>Wishlist</h3>
      <button class="full" onclick={() => openUrl(allkeyshopUrl)}>
        Check PC Steam key prices on Allkeyshop ↗
      </button>
    {/if}

    {#if game.statusHistory.length > 0}
      <h3>History</h3>
      <ul class="history">
        {#each game.statusHistory as e}
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
    padding: 22px;
    width: 480px;
    max-width: 92vw;
    max-height: 88vh;
    overflow-y: auto;
  }
  .head {
    display: flex;
    gap: 16px;
  }
  .cover {
    flex: none;
    width: 96px;
    height: 134px;
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
    font-size: 18px;
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
    font-size: 13px;
  }
  .stats dt {
    color: #8b909a;
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
