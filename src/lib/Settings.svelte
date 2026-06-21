<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import {
    app,
    persistSettings,
    syncSteamLibrary,
    gogConnect,
    syncGogLibrary,
    epicConnect,
    syncEpicLibrary,
  } from "./store.svelte";
  import { gogLoginUrl, epicLoginUrl } from "./api";

  let { onclose }: { onclose: () => void } = $props();

  // Local editable copies so we only commit on Save.
  let steamApiKey = $state(app.settings.steamApiKey ?? "");
  let steamId = $state(app.settings.steamId ?? "");
  let saving = $state(false);

  let steamSyncing = $state(false);
  let steamMsg = $state("");
  let steamErr = $state("");

  let gogCode = $state("");
  let gogBusy = $state(false);
  let gogMsg = $state("");
  let gogErr = $state("");
  const gogConnected = $derived(!!app.settings.gogRefreshToken);

  let epicCode = $state("");
  let epicBusy = $state(false);
  let epicMsg = $state("");
  let epicErr = $state("");
  const epicConnected = $derived(!!app.settings.epicRefreshToken);

  async function commitFields() {
    app.settings.steamApiKey = steamApiKey.trim() || undefined;
    app.settings.steamId = steamId.trim() || undefined;
    await persistSettings();
  }

  async function save() {
    saving = true;
    await commitFields();
    saving = false;
    onclose();
  }

  async function syncSteamNow() {
    steamSyncing = true;
    steamMsg = "";
    steamErr = "";
    try {
      await commitFields();
      const { added, updated } = await syncSteamLibrary();
      steamMsg = `Synced: ${added} added, ${updated} updated. Remember to Save.`;
    } catch (e) {
      steamErr = String(e);
    } finally {
      steamSyncing = false;
    }
  }

  async function openGogLogin() {
    await openUrl(await gogLoginUrl());
  }

  async function connectGog() {
    gogBusy = true;
    gogMsg = "";
    gogErr = "";
    try {
      await gogConnect(gogCode);
      gogCode = "";
      gogMsg = "GOG connected. You can sync now.";
    } catch (e) {
      gogErr = String(e);
    } finally {
      gogBusy = false;
    }
  }

  async function syncGogNow() {
    gogBusy = true;
    gogMsg = "";
    gogErr = "";
    try {
      const { added, updated } = await syncGogLibrary();
      gogMsg = `Synced: ${added} added, ${updated} updated. Remember to Save.`;
    } catch (e) {
      gogErr = String(e);
    } finally {
      gogBusy = false;
    }
  }

  async function openEpicLogin() {
    await openUrl(await epicLoginUrl());
  }

  async function connectEpic() {
    epicBusy = true;
    epicMsg = "";
    epicErr = "";
    try {
      await epicConnect(epicCode);
      epicCode = "";
      epicMsg = "Epic connected. You can sync now.";
    } catch (e) {
      epicErr = String(e);
    } finally {
      epicBusy = false;
    }
  }

  async function syncEpicNow() {
    epicBusy = true;
    epicMsg = "";
    epicErr = "";
    try {
      const { added, updated } = await syncEpicLibrary();
      epicMsg = `Synced: ${added} added, ${updated} updated. Remember to Save.`;
    } catch (e) {
      epicErr = String(e);
    } finally {
      epicBusy = false;
    }
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
      <button class="full" onclick={syncSteamNow} disabled={steamSyncing || !steamApiKey || !steamId}>
        {steamSyncing ? "Syncing…" : "Sync Steam library now"}
      </button>
      {#if steamMsg}<p class="ok">{steamMsg}</p>{/if}
      {#if steamErr}<p class="err">{steamErr}</p>{/if}
    </section>

    <section>
      <h3>GOG {#if gogConnected}<span class="badge">connected</span>{/if}</h3>
      <p class="note">
        Log in once in your browser, then copy the <span class="mono">code</span> value from the
        address bar after login (the page URL ends with <span class="mono">?…&code=XXXX</span>) and
        paste it below. Only the resulting token is stored, on this device.
      </p>
      <button class="full" onclick={openGogLogin}>Open GOG login in browser</button>
      <div class="row">
        <input bind:value={gogCode} placeholder="Paste GOG code here" />
        <button onclick={connectGog} disabled={gogBusy || !gogCode}>Connect</button>
      </div>
      <button class="full" onclick={syncGogNow} disabled={gogBusy || !gogConnected}>
        {gogBusy ? "Working…" : "Sync GOG library now"}
      </button>
      {#if gogMsg}<p class="ok">{gogMsg}</p>{/if}
      {#if gogErr}<p class="err">{gogErr}</p>{/if}
    </section>

    <section>
      <h3>Epic {#if epicConnected}<span class="badge">connected</span>{/if}</h3>
      <p class="note">
        Log in once in your browser. Epic then shows a page of JSON containing an
        <span class="mono">authorizationCode</span> — copy that value and paste it below. Only the
        resulting token is stored, on this device.
      </p>
      <button class="full" onclick={openEpicLogin}>Open Epic login in browser</button>
      <div class="row">
        <input bind:value={epicCode} placeholder="Paste Epic authorizationCode" />
        <button onclick={connectEpic} disabled={epicBusy || !epicCode}>Connect</button>
      </div>
      <button class="full" onclick={syncEpicNow} disabled={epicBusy || !epicConnected}>
        {epicBusy ? "Working…" : "Sync Epic library now"}
      </button>
      {#if epicMsg}<p class="ok">{epicMsg}</p>{/if}
      {#if epicErr}<p class="err">{epicErr}</p>{/if}
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
    max-height: 88vh;
    overflow-y: auto;
  }
  h2 {
    margin: 0 0 16px;
    font-size: 18px;
  }
  h3 {
    margin: 18px 0 8px;
    font-size: 14px;
  }
  .badge {
    font-size: 11px;
    color: #6ee7a0;
    border: 1px solid #2f5d44;
    border-radius: 10px;
    padding: 1px 8px;
    margin-left: 6px;
    vertical-align: middle;
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
  .row {
    display: flex;
    gap: 8px;
    align-items: stretch;
    margin: 8px 0;
  }
  .row input {
    margin-top: 0;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
  button {
    background: #2c2f37;
    color: #e6e6e6;
    border: 1px solid #3a3e48;
    border-radius: 7px;
    padding: 7px 14px;
    font-size: 13px;
    cursor: pointer;
    white-space: nowrap;
  }
  button.primary {
    background: #5865f2;
    border-color: #5865f2;
    color: #fff;
  }
  button:disabled {
    opacity: 0.5;
  }
  button.full {
    width: 100%;
    margin-top: 4px;
  }
  .ok {
    color: #6ee7a0;
    font-size: 12px;
    margin: 8px 0 0;
  }
  .err {
    color: #ffb4b4;
    font-size: 12px;
    margin: 8px 0 0;
  }
</style>
