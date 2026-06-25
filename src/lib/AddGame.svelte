<script lang="ts">
  import { addManualGame } from "./store.svelte";
  import { STATUSES, STATUS_LABELS, type Status, type StoreId } from "./types";

  let { onclose }: { onclose: () => void } = $props();

  let title = $state("");
  let status = $state<Status>("wishlist");
  let store = $state<"" | StoreId>("");
  let storeId = $state("");

  function add() {
    const t = title.trim();
    if (!t) return;
    addManualGame({
      title: t,
      status,
      store: store || undefined,
      storeId: storeId.trim() || undefined,
    });
    onclose();
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="modal" role="dialog" aria-modal="true" tabindex="-1">
    <h2>Add game</h2>
    <label>
      Title
      <!-- svelte-ignore a11y_autofocus -->
      <input bind:value={title} placeholder="e.g. Hollow Knight: Silksong" autofocus
        onkeydown={(e) => e.key === "Enter" && add()} />
    </label>
    <label>
      Status
      <select bind:value={status}>
        {#each STATUSES as s}
          <option value={s}>{STATUS_LABELS[s]}</option>
        {/each}
      </select>
    </label>
    <label>
      Store {status === "wishlist" ? "(optional)" : ""}
      <select bind:value={store}>
        <option value="">— none —</option>
        <option value="steam">Steam</option>
        <option value="gog">GOG</option>
        <option value="epic">Epic</option>
        <option value="ign">IGN</option>
      </select>
    </label>
    {#if store}
      <label>
        {store === "steam" ? "Steam App ID" : "Store ID"}
        <input bind:value={storeId} placeholder={store === "steam" ? "e.g. 1030300 — adds cover" : "store product id"}
          inputmode={store === "steam" ? "numeric" : "text"} />
      </label>
    {/if}
    <div class="actions">
      <button onclick={onclose}>Cancel</button>
      <button class="primary" onclick={add} disabled={!title.trim()}>Add</button>
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
    width: 420px;
    max-width: 92vw;
  }
  h2 {
    margin: 0 0 16px;
    font-size: 18px;
  }
  label {
    display: block;
    font-size: 12px;
    color: #8b909a;
    margin-bottom: 12px;
  }
  input,
  select {
    display: block;
    width: 100%;
    box-sizing: border-box;
    margin-top: 4px;
    background: #14161a;
    border: 1px solid #3a3e48;
    border-radius: 7px;
    padding: 8px 10px;
    color: #e6e6e6;
    font-size: 13px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
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
  .actions button:disabled {
    opacity: 0.5;
  }
</style>
