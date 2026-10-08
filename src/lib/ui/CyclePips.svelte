<script lang="ts">
  import { tint, type Phase } from "$lib/theme";

  // The run-up to the next long break. The final pip is wider and in the long
  // break colour, so the reward at the end of the set is visible from the start.
  let {
    position,
    length,
    phase,
    size = 6,
  }: { position: number; length: number; phase: Phase; size?: number } = $props();

  const count = $derived(Math.max(length, 1));

  const width = (index: number) => (index === count - 1 ? size * 2.4 : size);

  function fill(index: number): string {
    if (index < position) return tint(index === count - 1 ? "longBreak" : "focus");
    return "var(--ink-16)";
  }
</script>

<div class="pips" style:gap="{size * 0.7}px" style:--tint-focus={tint("focus")}>
  {#each { length: count } as _, index (index)}
    <span
      class="pip"
      class:current={index === position && phase === "focus"}
      style:width="{width(index)}px"
      style:height="{size}px"
      style:background={fill(index)}
    ></span>
  {/each}
</div>

<style>
  .pips {
    display: flex;
    align-items: center;
  }
  .pip {
    box-sizing: border-box;
    border-radius: 999px;
    /* Done and doing never read as the same state: the pip you're in gets a ring. */
    transition:
      background-color 400ms cubic-bezier(0.23, 1, 0.32, 1),
      width 400ms cubic-bezier(0.23, 1, 0.32, 1);
  }
  .pip.current {
    box-shadow: inset 0 0 0 1.2px var(--tint-focus);
  }
</style>
