<script lang="ts">
  import { addManualGame, addSearchResults, inLibrary, sourceEnabled } from "./store.svelte";
  import { storeSearch, type SearchResult } from "./api";
  import { normalizeTitle } from "./sync";
  import { STATUSES, STATUS_LABELS, type Status, type StoreId } from "./types";
  import StoreIcon from "./StoreIcon.svelte";

  const STORES: StoreId[] = ["steam", "gog", "epic", "ign"];
  const STORE_NAMES: Record<StoreId, string> = { steam: "Steam", gog: "GOG", epic: "Epic", ign: "IGN" };
  // Epic's store search sits behind a Cloudflare challenge, so it can't be searched.
  const SEARCHABLE: StoreId[] = ["steam", "gog", "ign"];

  let { onclose }: { onclose: () => void } = $props();

  const searchStores = STORES.filter((s) => SEARCHABLE.includes(s) && sourceEnabled(s));

  let title = $state("");
  let status = $state<Status>("wishlist");
  let manual = $state(false);
  let store = $state<"" | StoreId>("");
  let storeId = $state("");

  let searching = $state(false);
  let searched = $state(false);
  let results = $state<SearchResult[]>([]);
  let errors = $state<string[]>([]);
  let selected = $state<string[]>([]); // result keys

  const key = (r: SearchResult) => `${r.store}:${r.id}`;
  const pickedResults = $derived(results.filter((r) => selected.includes(key(r))));

  // Why a hit can't be picked: it's already in the library, or it's the same
  // game (by normalized title) as a hit that's already picked.
  function blockedReason(r: SearchResult): string | null {
    if (inLibrary(r)) return "In library";
    if (selected.includes(key(r))) return null;
    const norm = normalizeTitle(r.title);
    if (pickedResults.some((p) => normalizeTitle(p.title) === norm)) return "Duplicate";
    return null;
  }

  function toggle(r: SearchResult) {
    const k = key(r);
    selected = selected.includes(k) ? selected.filter((x) => x !== k) : [...selected, k];
  }

  async function search() {
    const term = title.trim();
    if (!term || searching) return;
    searching = true;
    errors = [];
    selected = [];
    const settled = await Promise.allSettled(searchStores.map((s) => storeSearch(s, term)));
    const found: SearchResult[] = [];
    settled.forEach((res, i) => {
      if (res.status === "fulfilled") found.push(...res.value);
      else errors.push(`${STORE_NAMES[searchStores[i]]}: ${res.reason}`);
    });
    results = found;
    searching = false;
    searched = true;
  }

  function addPicked() {
    if (pickedResults.length === 0) return;
    addSearchResults(pickedResults, status);
    onclose();
  }

  function addManual() {
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
  <div class="modal" class:wide={!manual} role="dialog" aria-modal="true" tabindex="-1">
    <h2>Add game</h2>
    {#if !manual}
      <div class="search-row">
        <!-- svelte-ignore a11y_autofocus -->
        <input bind:value={title} placeholder="Search by title, e.g. Hollow Knight" autofocus
          onkeydown={(e) => e.key === "Enter" && search()} />
        <button onclick={search} disabled={!title.trim() || searching || searchStores.length === 0}>
          {searching ? "Searching…" : "Search"}
        </button>
      </div>
      <p class="hint">
        {#if searchStores.length}
          Searching {searchStores.map((s) => STORE_NAMES[s]).join(", ")}{sourceEnabled("epic")
            ? " — Epic has no public search"
            : ""}.
        {:else}
          No searchable store is enabled in Settings.
        {/if}
      </p>
      {#each errors as err}
        <p class="error">{err}</p>
      {/each}
      {#if searched}
        <div class="results">
          {#each results as r (key(r))}
            {@const reason = blockedReason(r)}
            <label class="result" class:blocked={reason}>
              <input type="checkbox" checked={selected.includes(key(r))} disabled={!!reason}
                onchange={() => toggle(r)} />
              {#if r.coverUrl}
                <img src={r.coverUrl} alt="" loading="lazy"
                  onerror={(e) => ((e.currentTarget as HTMLImageElement).style.visibility = "hidden")} />
              {:else}
                <span class="noimg"></span>
              {/if}
              <StoreIcon store={r.store} size={14} />
              <span class="rtitle">{r.title}</span>
              {#if reason}<span class="tag">{reason}</span>{/if}
            </label>
          {:else}
            <p class="hint">No results.</p>
          {/each}
        </div>
      {/if}
      <label>
        Status for added games
        <select bind:value={status}>
          {#each STATUSES as s}
            <option value={s}>{STATUS_LABELS[s]}</option>
          {/each}
        </select>
      </label>
      <div class="actions">
        <button class="link" onclick={() => (manual = true)}>Add manually instead</button>
        <button onclick={onclose}>Cancel</button>
        <button class="primary" onclick={addPicked} disabled={pickedResults.length === 0}>
          Add{pickedResults.length ? ` ${pickedResults.length}` : ""}
        </button>
      </div>
    {:else}
      <label>
        Title
        <!-- svelte-ignore a11y_autofocus -->
        <input bind:value={title} placeholder="e.g. Cyberpunk 2077" autofocus
          onkeydown={(e) => e.key === "Enter" && addManual()} />
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
          {#each STORES.filter(sourceEnabled) as s}
            <option value={s}>{STORE_NAMES[s]}</option>
          {/each}
        </select>
      </label>
      {#if store}
        <label>
          {store === "steam" ? "Steam App ID" : "Store ID"}
          <input bind:value={storeId} placeholder={store === "steam" ? "e.g. 1091500 — adds cover" : "store product id"}
            inputmode={store === "steam" ? "numeric" : "text"} />
        </label>
      {/if}
      <div class="actions">
        <button class="link" onclick={() => (manual = false)}>Back to search</button>
        <button onclick={onclose}>Cancel</button>
        <button class="primary" onclick={addManual} disabled={!title.trim()}>Add</button>
      </div>
    {/if}
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
    max-height: 90dvh;
    overflow-y: auto;
    box-sizing: border-box;
  }
  .modal.wide {
    width: 560px;
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
  .search-row {
    display: flex;
    gap: 8px;
  }
  .search-row input {
    margin-top: 0;
  }
  .hint {
    font-size: 12px;
    color: #8b909a;
    margin: 8px 0 12px;
  }
  .error {
    font-size: 12px;
    color: #ffb4b4;
    margin: 0 0 8px;
  }
  .results {
    max-height: 45dvh;
    overflow-y: auto;
    border: 1px solid #3a3e48;
    border-radius: 7px;
    margin-bottom: 12px;
  }
  .result {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0;
    padding: 6px 10px;
    color: #e6e6e6;
    font-size: 13px;
    cursor: pointer;
    border-bottom: 1px solid #2c2f37;
  }
  .result:last-child {
    border-bottom: none;
  }
  .result.blocked {
    opacity: 0.5;
    cursor: default;
  }
  .result input {
    width: auto;
    margin: 0;
    flex: none;
  }
  .result img,
  .noimg {
    width: 48px;
    height: 28px;
    object-fit: cover;
    border-radius: 3px;
    flex: none;
    background: #14161a;
  }
  .rtitle {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag {
    font-size: 11px;
    color: #8b909a;
    flex: none;
  }
  .results .hint {
    margin: 10px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }
  .actions button,
  .search-row button {
    background: #2c2f37;
    color: #e6e6e6;
    border: 1px solid #3a3e48;
    border-radius: 7px;
    padding: 8px 14px;
    font-size: 13px;
    cursor: pointer;
    flex: none;
  }
  .actions button.primary {
    background: #5865f2;
    border-color: #5865f2;
    color: #fff;
  }
  .actions button.link {
    margin-right: auto;
    background: none;
    border: none;
    padding: 8px 0;
    color: #8b909a;
    text-decoration: underline;
  }
  .actions button:disabled,
  .search-row button:disabled {
    opacity: 0.5;
  }
</style>
