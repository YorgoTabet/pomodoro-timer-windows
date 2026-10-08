<script lang="ts">
  import type { Snippet } from "svelte";
  import { PHASE_TITLES, highlight, tint, type Phase } from "$lib/theme";

  // The progress ring with the countdown inside it. The time label comes in as a
  // snippet, like the Swift view's injected `timeLabel`.
  let {
    phase,
    progress,
    lineWidth = 8,
    isRunning = false,
    timeLabel,
  }: {
    phase: Phase;
    progress: number;
    lineWidth?: number;
    isRunning?: boolean;
    timeLabel: Snippet;
  } = $props();

  const box = 120;
  const radius = $derived((box - lineWidth) / 2);
  const circumference = $derived(2 * Math.PI * radius);
  const shown = $derived(Math.max(progress, 0.0001));
</script>

<div class="dial" class:running={isRunning} style:--tint={tint(phase)}>
  <!-- A soft phase-coloured bloom, so the ring has something under it. -->
  <div class="bloom" style:background="radial-gradient(circle, {tint(phase, 0.3)}, transparent 70%)"></div>

  <svg viewBox="0 0 {box} {box}" aria-hidden="true">
    <defs>
      <!-- The Swift ring sweeps an angular gradient; SVG has none, so this runs the
           same three colours down the ring in a straight line. -->
      <linearGradient id="dial-sweep" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color={tint(phase, 0.75)} />
        <stop offset="0.5" stop-color={tint(phase)} />
        <stop offset="1" stop-color={highlight(phase)} />
      </linearGradient>
    </defs>
    <circle cx={box / 2} cy={box / 2} r={radius} fill="none" stroke="var(--ink-10)" stroke-width={lineWidth} />
    <!-- Starts at 12 o'clock and sweeps clockwise, the way a kitchen timer does. -->
    <circle
      cx={box / 2}
      cy={box / 2}
      r={radius}
      fill="none"
      stroke="url(#dial-sweep)"
      stroke-width={lineWidth}
      stroke-linecap="round"
      stroke-dasharray="{shown * circumference} {circumference}"
      transform="rotate(-90 {box / 2} {box / 2})"
      style:filter="drop-shadow(0 0 4px {tint(phase, 0.5)})"
    />
  </svg>

  <div class="label">
    {@render timeLabel()}
    <span class="phase">{PHASE_TITLES[phase].toUpperCase()}</span>
  </div>
</div>

<style>
  .dial {
    position: relative;
    width: 120px;
    height: 120px;
    display: grid;
    place-items: center;
  }
  .bloom,
  svg,
  .label {
    grid-area: 1 / 1;
  }
  .bloom {
    width: 140px;
    height: 140px;
    border-radius: 50%;
    filter: blur(8px);
    opacity: 0.45;
    transform: scale(0.94);
    transition: opacity 450ms cubic-bezier(0.23, 1, 0.32, 1);
  }
  .running .bloom {
    opacity: 1;
    animation: breathe 2.6s ease-in-out infinite alternate;
  }
  @keyframes breathe {
    to {
      transform: scale(1.06);
    }
  }
  svg {
    width: 100%;
    height: 100%;
  }
  .label {
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .phase {
    font-size: 8px;
    font-weight: 700;
    letter-spacing: 0.9px;
    color: var(--tint);
    white-space: nowrap;
    transition: color 450ms cubic-bezier(0.23, 1, 0.32, 1);
  }
  @media (prefers-reduced-motion: reduce) {
    .running .bloom {
      animation: none;
    }
  }
</style>
