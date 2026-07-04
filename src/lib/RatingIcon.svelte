<script lang="ts">
  // Steam-style review summary icon: thumbs-up / tilde / thumbs-down, using
  // Steam's own buckets (≥70% positive, 40–69% mixed, <40% negative). A 0
  // means "no reviews" (e.g. unreleased), not a real score — show nothing.
  let { rating, size = 14 }: { rating: number | undefined; size?: number } = $props();

  const kind = $derived(!rating ? null : rating >= 70 ? "up" : rating >= 40 ? "mixed" : "down");
</script>

{#if kind !== null && rating}
  <span class="rating-icon" title="Rating: {rating}/100">
    {#if kind === "up"}
      <svg width={size} height={size} viewBox="0 0 24 24" fill="#66c0f4" role="img" aria-label="Positive">
        <path
          d="M1 21h4V9H1v12zm22-11c0-1.1-.9-2-2-2h-6.31l.95-4.57.03-.32c0-.41-.17-.79-.44-1.06L14.17 1 7.58 7.59C7.22 7.95 7 8.45 7 9v10c0 1.1.9 2 2 2h9c.83 0 1.54-.5 1.84-1.22l3.02-7.05c.09-.23.14-.47.14-.72v-2z"
        />
      </svg>
    {:else if kind === "mixed"}
      <svg
        width={size}
        height={size}
        viewBox="0 0 24 24"
        fill="none"
        stroke="#b9a074"
        stroke-width="3"
        stroke-linecap="round"
        role="img"
        aria-label="Mixed"
      >
        <path d="M3 14c2.5-4.5 6.5-4.5 9 0s6.5 4.5 9 0" />
      </svg>
    {:else}
      <svg width={size} height={size} viewBox="0 0 24 24" fill="#f87171" role="img" aria-label="Negative">
        <path
          d="M15 3H6c-.83 0-1.54.5-1.84 1.22l-3.02 7.05c-.09.23-.14.47-.14.73v2c0 1.1.9 2 2 2h6.31l-.95 4.57-.03.32c0 .41.17.79.44 1.06L9.83 23l6.59-6.59c.36-.36.58-.86.58-1.41V5c0-1.1-.9-2-2-2zm4 0v12h4V3h-4z"
        />
      </svg>
    {/if}
  </span>
{/if}

<style>
  .rating-icon {
    display: inline-flex;
    align-items: center;
    vertical-align: middle;
  }
</style>
