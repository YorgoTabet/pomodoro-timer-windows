<script lang="ts">
  // The handful of SF Symbols the bar uses, redrawn on a 24 point grid. The Mac
  // app draws these with system symbols, which the web view does not have.
  export type GlyphName = "play" | "pause" | "skip" | "reset" | "timer" | "cup" | "walk";

  let { name, size = 12 }: { name: GlyphName; size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" aria-hidden="true">
  {#if name === "play"}
    <path d="M7 4.5v15l12.5-7.5z" fill="currentColor" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" />
  {:else if name === "pause"}
    <rect x="5.5" y="4.5" width="4.5" height="15" rx="1.4" fill="currentColor" />
    <rect x="14" y="4.5" width="4.5" height="15" rx="1.4" fill="currentColor" />
  {:else if name === "skip"}
    <path d="M4.5 5v14l10.5-7z" fill="currentColor" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" />
    <rect x="17" y="5" width="3" height="14" rx="1.2" fill="currentColor" />
  {:else if name === "reset"}
    <path d="M4.5 12a7.5 7.5 0 1 0 2.3-5.4" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" />
    <path d="M4 3.5v5h5" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" />
  {:else if name === "timer"}
    <circle cx="12" cy="13.5" r="7.5" stroke="currentColor" stroke-width="2.4" />
    <path d="M12 13.5V9.5M9.5 3h5" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" />
  {:else if name === "cup"}
    <path d="M4.5 9.5h11.5v4.5a5 5 0 0 1-5 5h-1.5a5 5 0 0 1-5-5z" fill="currentColor" />
    <path d="M16 10.5h1.2a2.8 2.8 0 0 1 0 5.6H16M3.5 21.5h14" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" />
  {:else if name === "walk"}
    <circle cx="13.5" cy="4.5" r="2.2" fill="currentColor" />
    <path
      d="M12.5 9 9.5 12l1 4M12.5 9l2.5 3 3 .5M12.5 9l1 5.5-3 3-1 4M13.5 14.5l3 3 .8 4"
      stroke="currentColor"
      stroke-width="2.3"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  {/if}
</svg>
