<script lang="ts">
  // Games a sync added since the last review, each with its own status pick.
  // The pick starts at whatever the sync seeded (Backlog, or IGN's/Steam
  // wishlist's status); applying settles it and clears the "new" flag.
  import { app, gameVisible, reviewGame } from "./store.svelte";
  import { STATUSES, STATUS_LABELS, type Status } from "./types";
  import StoreIcon from "./StoreIcon.svelte";
  import { coverFallback } from "./cover";

  let { onclose }: { onclose: () => void } = $props();

  // Snapshot the list on open so rows don't vanish while picking.
  const games = app.library.games.filter((g) => g.unreviewed && gameVisible(g));
  let choices = $state<Record<string, Status>>(Object.fromEntries(games.map((g) => [g.id, g.status])));

  function setAll(s: Status) {
    for (const g of games) choices[g.id] = s;
  }

  function apply() {
    for (const g of games) reviewGame(g, choices[g.id]);
    onclose();
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="modal" role="dialog" aria-modal="true" tabindex="-1">
    <h2>New games ({games.length})</h2>
    <p class="hint">Added by a sync. Pick a status for each one.</p>
    <label class="all">
      Set all to
      <select onchange={(e) => setAll((e.currentTarget as HTMLSelectElement).value as Status)}>
        <option value="" disabled selected>—</option>
        {#each STATUSES as s}
          <option value={s}>{STATUS_LABELS[s]}</option>
        {/each}
      </select>
    </label>
    <div class="rows">
      {#each games as g (g.id)}
        <div class="row">
          {#if g.coverUrl}
            <img src={g.coverUrl} alt="" loading="lazy" onerror={coverFallback} />
          {:else}
            <span class="noimg"></span>
          {/if}
          {#each Object.keys(g.sources) as src}
            <StoreIcon store={src} size={13} />
          {/each}
          <span class="gtitle">{g.title}</span>
          <select bind:value={choices[g.id]}>
            {#each STATUSES as s}
              <option value={s}>{STATUS_LABELS[s]}</option>
            {/each}
          </select>
        </div>
      {/each}
    </div>
    <div class="actions">
      <button onclick={onclose}>Later</button>
      <button class="primary" onclick={apply}>Apply</button>
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
    width: 560px;
    max-width: 92vw;
    max-height: 90dvh;
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
  }
  h2 {
    margin: 0 0 6px;
    font-size: 18px;
  }
  .hint {
    font-size: 12px;
    color: #8b909a;
    margin: 0 0 12px;
  }
  .all {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: #8b909a;
    margin-bottom: 10px;
  }
  select {
    background: #14161a;
    border: 1px solid #3a3e48;
    border-radius: 7px;
    padding: 5px 8px;
    color: #e6e6e6;
    font-size: 13px;
  }
  .rows {
    overflow-y: auto;
    border: 1px solid #3a3e48;
    border-radius: 7px;
    margin-bottom: 12px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    font-size: 13px;
    border-bottom: 1px solid #2c2f37;
  }
  .row:last-child {
    border-bottom: none;
  }
  .row img,
  .noimg {
    width: 24px;
    height: 32px;
    object-fit: cover;
    border-radius: 3px;
    flex: none;
    background: #14161a;
  }
  .gtitle {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row select {
    flex: none;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .actions button {
    background: #2c2f37;
    color: #e6e6e6;
    border: 1px solid #3a3e48;
    border-radius: 7px;
    padding: 8px 14px;
    font-size: 13px;
    cursor: pointer;
  }
  .actions button.primary {
    background: #5865f2;
    border-color: #5865f2;
    color: #fff;
  }
</style>
