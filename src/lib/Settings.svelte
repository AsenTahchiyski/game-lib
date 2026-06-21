<script lang="ts">
  import { app, persistSettings } from "./store.svelte";

  let { onclose }: { onclose: () => void } = $props();

  // Local editable copy so we only commit on Save.
  let steamApiKey = $state(app.settings.steamApiKey ?? "");
  let steamId = $state(app.settings.steamId ?? "");
  let saving = $state(false);

  async function save() {
    saving = true;
    app.settings.steamApiKey = steamApiKey.trim() || undefined;
    app.settings.steamId = steamId.trim() || undefined;
    await persistSettings();
    saving = false;
    onclose();
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div
  class="overlay"
  role="presentation"
  onclick={(e) => e.target === e.currentTarget && onclose()}
>
  <div class="modal" role="dialog" aria-modal="true" tabindex="-1">
    <h2>Settings</h2>

    <section>
      <h3>Steam</h3>
      <p class="note">
        Get an API key at
        <span class="mono">steamcommunity.com/dev/apikey</span>. Your SteamID is the 17-digit
        number from your profile URL. Both are stored only on this device, never in the shared
        library file.
      </p>
      <label>
        API key
        <input type="password" bind:value={steamApiKey} placeholder="0123456789ABCDEF…" />
      </label>
      <label>
        SteamID (17 digits)
        <input bind:value={steamId} placeholder="76561198000000000" />
      </label>
    </section>

    <div class="actions">
      <button onclick={onclose}>Cancel</button>
      <button class="primary" onclick={save} disabled={saving}>Save</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
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
    width: 460px;
    max-width: 90vw;
  }
  h2 {
    margin: 0 0 16px;
    font-size: 18px;
  }
  h3 {
    margin: 0 0 8px;
    font-size: 14px;
  }
  .note {
    font-size: 12px;
    color: #8b909a;
    margin: 0 0 12px;
    line-height: 1.5;
  }
  .mono {
    font-family: monospace;
    color: #b9bdc7;
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
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
  }
  button {
    background: #2c2f37;
    color: #e6e6e6;
    border: 1px solid #3a3e48;
    border-radius: 7px;
    padding: 7px 14px;
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
