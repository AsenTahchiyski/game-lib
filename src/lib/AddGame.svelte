<script lang="ts">
  import { addManualGame } from "./store.svelte";

  let { onclose }: { onclose: () => void } = $props();

  let title = $state("");
  let appId = $state("");

  function add() {
    const t = title.trim();
    if (!t) return;
    const parsed = parseInt(appId.trim(), 10);
    addManualGame(t, Number.isFinite(parsed) && parsed > 0 ? parsed : undefined);
    onclose();
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="modal" role="dialog" aria-modal="true" tabindex="-1">
    <h2>Add game to Wishlist</h2>
    <label>
      Title
      <!-- svelte-ignore a11y_autofocus -->
      <input bind:value={title} placeholder="e.g. Hollow Knight: Silksong" autofocus
        onkeydown={(e) => e.key === "Enter" && add()} />
    </label>
    <label>
      Steam App ID (optional)
      <input bind:value={appId} placeholder="e.g. 1030300 — adds cover art" inputmode="numeric" />
    </label>
    <p class="note">
      Added to Wishlist. A later Steam sync will match and enrich it by title (or by App ID if set).
    </p>
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
  input {
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
  .note {
    font-size: 12px;
    color: #8b909a;
    margin: 0 0 12px;
    line-height: 1.5;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  button {
    background: #2c2f37;
    color: #e6e6e6;
    border: 1px solid #3a3e48;
    border-radius: 7px;
    padding: 8px 14px;
    font-size: 13px;
    cursor: pointer;
  }
  button.primary {
    background: #5865f2;
    border-color: #5865f2;
    color: #fff;
  }
  button:disabled {
    opacity: 0.5;
  }
</style>
