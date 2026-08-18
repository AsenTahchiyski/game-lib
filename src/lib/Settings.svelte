<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { getVersion } from "@tauri-apps/api/app";
  import {
    app,
    persistSettings,
    openLibrary,
    newLibrary,
    checkForUpdate,
    syncSteamLibrary,
    gogConnect,
    syncGogLibrary,
    epicConnect,
    syncEpicLibrary,
    syncIgnLibrary,
    sourceEnabled,
    toggleSourceEnabled,
  } from "./store.svelte";
  import { SOURCE_TOGGLES, SOURCE_TOGGLE_LABELS } from "./types";
  import { gogLoginUrl, epicLoginUrl } from "./api";
  import { releaseUrl } from "./format";

  let { onclose }: { onclose: () => void } = $props();

  let version = $state("");
  getVersion().then((v) => (version = v));

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

  let ignNickname = $state(app.settings.ignNickname ?? "");
  let ignBusy = $state(false);
  let ignMsg = $state("");
  let ignErr = $state("");

  async function openDifferentFile() {
    const before = app.currentPath;
    await openLibrary();
    if (app.currentPath !== before) onclose();
  }

  async function startNewLibrary() {
    const ok = await confirm(
      "Start a new, empty library? The current file keeps its games; the new library isn't written anywhere until you save it.",
      { title: "New library", kind: "warning" },
    );
    if (ok) {
      newLibrary();
      onclose();
    }
  }

  let checkingUpdate = $state(false);
  let updateMsg = $state("");
  let updateErr = $state("");
  async function checkUpdatesNow() {
    checkingUpdate = true;
    updateMsg = "";
    updateErr = "";
    try {
      const latest = await checkForUpdate(version);
      updateMsg = latest ? `Version ${latest} is available.` : "You're up to date.";
    } catch (e) {
      updateErr = String(e);
    } finally {
      checkingUpdate = false;
    }
  }

  async function commitFields() {
    app.settings.steamApiKey = steamApiKey.trim() || undefined;
    app.settings.steamId = steamId.trim() || undefined;
    app.settings.ignNickname = ignNickname.trim() || undefined;
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
      steamMsg = `Synced: ${added} added, ${updated} updated.`;
    } catch (e) {
      steamErr = String(e);
    } finally {
      steamSyncing = false;
    }
  }

  async function openGogLogin() {
    await openUrl(await gogLoginUrl());
  }

  /** No code = log in through the embedded window; a code = the manual path. */
  async function connectGog(code = "") {
    gogBusy = true;
    gogMsg = "";
    gogErr = "";
    try {
      await gogConnect(code || undefined);
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
      gogMsg = `Synced: ${added} added, ${updated} updated.`;
    } catch (e) {
      gogErr = String(e);
    } finally {
      gogBusy = false;
    }
  }

  async function openEpicLogin() {
    await openUrl(await epicLoginUrl());
  }

  async function connectEpic(code = "") {
    epicBusy = true;
    epicMsg = "";
    epicErr = "";
    try {
      await epicConnect(code || undefined);
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
      epicMsg = `Synced: ${added} added, ${updated} updated.`;
    } catch (e) {
      epicErr = String(e);
    } finally {
      epicBusy = false;
    }
  }

  async function importIgnNow() {
    ignBusy = true;
    ignMsg = "";
    ignErr = "";
    try {
      await commitFields();
      const { added, updated } = await syncIgnLibrary();
      ignMsg = `Imported: ${added} added, ${updated} updated.`;
    } catch (e) {
      ignErr = String(e);
    } finally {
      ignBusy = false;
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
      <h3>Library file</h3>
      {#if app.currentPath}
        <p class="note"><span class="mono">{app.currentPath}</span></p>
      {:else}
        <p class="note">No file loaded — use Save in the header to create one.</p>
      {/if}
      <div class="row">
        <button class="full" onclick={openDifferentFile}>Open a different file…</button>
        <button class="full" onclick={startNewLibrary}>New empty library</button>
      </div>
    </section>

    <section>
      <h3>Sources</h3>
      <p class="note">
        Toggle a source off to hide it everywhere in the app — its badges, links, sync section,
        and any game only that source provides. Nothing is deleted; toggling back on restores it.
      </p>
      <div class="toggles">
        {#each SOURCE_TOGGLES as s}
          <label class="toggle">
            <input type="checkbox" checked={sourceEnabled(s)} onchange={() => toggleSourceEnabled(s)} />
            {SOURCE_TOGGLE_LABELS[s]}
          </label>
        {/each}
      </div>
    </section>

    {#if sourceEnabled("steam")}
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
    {/if}

    {#if sourceEnabled("gog")}
    <section>
      <h3>GOG {#if gogConnected}<span class="badge">connected</span>{/if}</h3>
      <p class="note">
        Log in once in the window that opens; the app picks up the rest itself. Only the resulting
        token is stored, on this device.
      </p>
      <button class="full" onclick={() => connectGog()} disabled={gogBusy}>
        {gogConnected ? "Log in to GOG again" : "Log in to GOG"}
      </button>
      <details>
        <summary>Enter a code manually</summary>
        <p class="note">
          If the login window doesn't work, log in in your browser instead and copy the
          <span class="mono">code</span> value from the address bar afterwards (the page URL ends
          with <span class="mono">?…&code=XXXX</span>).
        </p>
        <button class="full" onclick={openGogLogin}>Open GOG login in browser</button>
        <div class="row">
          <input bind:value={gogCode} placeholder="Paste GOG code here" />
          <button onclick={() => connectGog(gogCode)} disabled={gogBusy || !gogCode}>Connect</button>
        </div>
      </details>
      <button class="full" onclick={syncGogNow} disabled={gogBusy || !gogConnected}>
        {gogBusy ? "Working…" : "Sync GOG library now"}
      </button>
      {#if gogMsg}<p class="ok">{gogMsg}</p>{/if}
      {#if gogErr}<p class="err">{gogErr}</p>{/if}
    </section>
    {/if}

    {#if sourceEnabled("epic")}
    <section>
      <h3>Epic {#if epicConnected}<span class="badge">connected</span>{/if}</h3>
      <p class="note">
        Log in once in the window that opens; the app picks up the rest itself. Only the resulting
        token is stored, on this device.
      </p>
      <button class="full" onclick={() => connectEpic()} disabled={epicBusy}>
        {epicConnected ? "Log in to Epic again" : "Log in to Epic"}
      </button>
      <details>
        <summary>Enter a code manually</summary>
        <p class="note">
          If the login window doesn't work, log in in your browser instead. Epic then shows a page
          of JSON containing an <span class="mono">authorizationCode</span> — copy that value here.
        </p>
        <button class="full" onclick={openEpicLogin}>Open Epic login in browser</button>
        <div class="row">
          <input bind:value={epicCode} placeholder="Paste Epic authorizationCode" />
          <button onclick={() => connectEpic(epicCode)} disabled={epicBusy || !epicCode}>
            Connect
          </button>
        </div>
      </details>
      <button class="full" onclick={syncEpicNow} disabled={epicBusy || !epicConnected}>
        {epicBusy ? "Working…" : "Sync Epic library now"}
      </button>
      {#if epicMsg}<p class="ok">{epicMsg}</p>{/if}
      {#if epicErr}<p class="err">{epicErr}</p>{/if}
    </section>
    {/if}

    {#if sourceEnabled("ign")}
    <section>
      <h3>IGN Playlist</h3>
      <p class="note">
        One-time migration of your curated statuses from IGN's Playlist app. Set your playlist
        to <span class="mono">Public</span> in IGN, then enter your nickname (the last part of
        <span class="mono">ign.com/playlist/yourname</span>) or paste the full profile URL. No
        login needed — nothing is stored except the nickname.
      </p>
      <label>
        IGN nickname or profile URL
        <input bind:value={ignNickname} placeholder="malkstor" />
      </label>
      <button class="full" onclick={importIgnNow} disabled={ignBusy || !ignNickname}>
        {ignBusy ? "Importing…" : "Import IGN playlist now"}
      </button>
      {#if ignMsg}<p class="ok">{ignMsg}</p>{/if}
      {#if ignErr}<p class="err">{ignErr}</p>{/if}
    </section>
    {/if}

    <section>
      <h3>About</h3>
      <p class="note">
        Game Library {#if version}v{version}{/if} — tracks what you're playing, plan to play, and
        have beaten across Steam, GOG, Epic and IGN. Your library is a single JSON file you choose,
        so it can live on a NAS or synced folder and be shared between devices.
      </p>
      <div class="row">
        <button class="full" onclick={() => openUrl("https://github.com/AsenTahchiyski/game-lib")}>
          GitHub ↗
        </button>
        <button class="full" onclick={checkUpdatesNow} disabled={checkingUpdate}>
          {checkingUpdate ? "Checking…" : "Check for updates"}
        </button>
      </div>
      {#if updateMsg}
        <p class="ok">
          {updateMsg}
          {#if app.updateAvailable}
            <button onclick={() => openUrl(releaseUrl())}>Download ↗</button>
          {/if}
        </p>
      {/if}
      {#if updateErr}<p class="err">Update check failed: {updateErr}</p>{/if}
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
    z-index: 1000; /* above the sticky table header and column/status menus */
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
  .toggles {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 16px;
  }
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: #e6e6e6;
    margin-bottom: 0;
  }
  .toggle input {
    display: inline;
    width: auto;
    margin: 0;
    accent-color: #5865f2;
  }
  .row input {
    margin-top: 0;
  }
  details {
    margin-top: 8px;
  }
  summary {
    font-size: 12px;
    color: #8b909a;
    cursor: pointer;
  }
  details .note {
    margin-top: 8px;
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
