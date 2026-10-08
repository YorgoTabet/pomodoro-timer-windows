<script lang="ts">
  import { onMount } from "svelte";
  import { LogicalSize } from "@tauri-apps/api/dpi";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { BarPresentationModel, DEFAULT_POLICY, barMode, type BarPolicy } from "$lib/barPresentation";
  import { METRICS, MORPH_MS, compactWidth, secondaryX, tuck, width } from "$lib/barMetrics";
  import {
    DEFAULT_SETTINGS,
    DEFAULT_STATE,
    inTauri,
    onPhaseElapsed,
    settings,
    timerCommands,
    timerState,
    type Settings,
    type TimerState,
  } from "$lib/timer";
  import { PHASE_TITLES, backdrop, displayTime, highlight, progress as progressOf, tint, type Phase } from "$lib/theme";
  import Glyph, { type GlyphName } from "$lib/ui/Glyph.svelte";

  const M = METRICS;
  const PHASES: Phase[] = ["focus", "shortBreak", "longBreak"];
  const PHASE_GLYPH: Record<Phase, GlyphName> = { focus: "timer", shortBreak: "cup", longBreak: "walk" };

  // A plain browser has no timer behind it, so the dev server can be pointed at a
  // state with query parameters. Never read inside the app.
  const mock = !inTauri() && import.meta.env.DEV ? new URLSearchParams(location.search) : null;
  const mockState: TimerState = {
    ...DEFAULT_STATE,
    phase: (mock?.get("phase") as Phase) ?? DEFAULT_STATE.phase,
    isRunning: mock?.get("running") === "1",
    remainingSeconds: Math.round(DEFAULT_STATE.totalSeconds * (1 - Number(mock?.get("progress") ?? 0))),
  };
  const mockSettings: Settings = { ...DEFAULT_SETTINGS, compactFloatingBar: mock?.get("compact") === "1" };

  const timer = $derived(mock ? mockState : $timerState);
  const prefs = $derived(mock ? mockSettings : $settings);

  let policy = $state<BarPolicy>({ ...DEFAULT_POLICY });
  const model = new BarPresentationModel((next) => (policy = next));
  const compact = $derived(barMode(policy) === "compact");
  const hovering = $derived(policy.hovering);
  // Off once compact mode is on: there the opening is the response to the pointer.
  const flourish = $derived(!policy.compactEnabled);

  $effect(() => model.setRunning(timer.isRunning));
  $effect(() => model.setCompactEnabled(prefs.compactFloatingBar));

  const ringProgress = $derived(progressOf(timer.remainingSeconds, timer.totalSeconds));
  const time = $derived(displayTime(timer.remainingSeconds));
  const title = $derived(PHASE_TITLES[timer.phase]);

  let pill: HTMLElement;

  onMount(() => {
    const stopPhase = onPhaseElapsed(() => model.beginPhaseChange());
    return () => {
      stopPhase();
      model.dispose();
    };
  });

  // The window is only as wide as the pill's current form, so the empty strip past
  // the play button does not swallow clicks meant for whatever is behind it. It
  // widens the moment expansion begins and narrows once the collapse has finished,
  // so no click lands in the gap.
  let narrowTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    clearTimeout(narrowTimer);
    if (!inTauri()) return;
    const target = compact ? compactWidth : width;
    const resize = () => getCurrentWindow().setSize(new LogicalSize(target, M.height));
    if (!compact) {
      resize();
      return;
    }
    narrowTimer = setTimeout(() => {
      resize();
      // The edge withdrew past a stationary cursor, so the browser may not say so.
      model.setHovering(pill.matches(":hover"));
    }, MORPH_MS);
    return () => clearTimeout(narrowTimer);
  });

  // Dragging starts anywhere on the pill except on a button.
  function startDrag(event: MouseEvent) {
    if (event.button !== 0 || (event.target as Element).closest("button") || !inTauri()) return;
    getCurrentWindow().startDragging();
  }

  const wash = (phase: Phase) => (phase === timer.phase ? (timer.isRunning ? 0.85 : 0.45) : 0);
</script>

<svelte:head>
  <title>Pomodoro</title>
  <style>
    html,
    body {
      background: transparent;
      overflow: hidden;
      user-select: none;
      -webkit-user-select: none;
      cursor: default;
    }
  </style>
</svelte:head>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="pill"
  class:flourish
  class:rest={flourish && !hovering}
  bind:this={pill}
  style:width="{compact ? compactWidth : width}px"
  style:--tint={tint(timer.phase)}
  style:--highlight={highlight(timer.phase)}
  onmousedown={startDrag}
  onpointerenter={() => model.setHovering(true)}
  onpointerleave={() => model.setHovering(false)}
>
  {#each PHASES as phase (phase)}
    <div class="wash" style:background={backdrop(phase)} style:opacity={wash(phase)}></div>
  {/each}

  <div class="row" style:width="{width}px">
    <div class="ring" role="img" aria-label="{title}, {time} remaining">
      <svg viewBox="0 0 28 28" aria-hidden="true">
        <defs>
          <linearGradient id="ring-grad" x1="0" y1="0" x2="0" y2="28" gradientUnits="userSpaceOnUse">
            <stop offset="0" style:stop-color={highlight(timer.phase)} />
            <stop offset="1" style:stop-color={tint(timer.phase)} />
          </linearGradient>
        </defs>
        <circle cx="14" cy="14" r="12.75" fill="none" stroke="var(--ink-14)" stroke-width="2.5" />
        <!-- Not eased on progress: a step of 1/1500th of the ring is under a pixel. -->
        <circle
          cx="14"
          cy="14"
          r="12.75"
          fill="none"
          stroke="url(#ring-grad)"
          stroke-width="2.5"
          stroke-linecap="round"
          pathLength="1000"
          stroke-dasharray="{ringProgress * 1000} 1000"
          transform="rotate(-90 14 14)"
          opacity={ringProgress > 0.005 ? 1 : 0}
        />
      </svg>
      <span class="glyph" class:breathing={timer.isRunning}><Glyph name={PHASE_GLYPH[timer.phase]} size={10} /></span>
    </div>

    <div class="readout" style:margin-left="{M.ringGap}px">
      <span class="time" class:idle={!timer.isRunning}>{time}</span>
      <span class="label" class:idle={!timer.isRunning}>{timer.isRunning ? title : "Paused"}</span>
    </div>

    <button
      class="control primary"
      style:margin-left="{M.readoutGap}px"
      aria-label={timer.isRunning ? "Pause" : "Start"}
      onclick={() => timerCommands.toggle()}
    >
      <Glyph name={timer.isRunning ? "pause" : "play"} size={M.primary * 0.38 + 2} />
    </button>

    <!-- Always mounted and moved by a transform, so a flick that turns around
         mid-open reverses instead of replaying from the far end. -->
    {#each [{ name: "skip", label: "Skip", index: 0, run: timerCommands.skip }, { name: "reset", label: "Reset", index: 1, run: timerCommands.reset }] as c (c.name)}
      <button
        class="control secondary"
        class:parked={compact}
        style:margin-left="{M.controlGap}px"
        style:--tuck="{tuck(c.index)}px"
        style:z-index={1 - c.index}
        aria-label={c.label}
        aria-hidden={compact}
        tabindex={compact ? -1 : 0}
        onclick={c.run}
      >
        <Glyph name={c.name as GlyphName} size={M.secondary * 0.38 + 2} />
      </button>
    {/each}
  </div>
</div>

<style>
  /* Same spring as the Mac app's .smooth(0.42): critically damped, no overshoot. */
  .pill {
    --morph: 420ms cubic-bezier(0.22, 1, 0.36, 1);
    position: relative;
    box-sizing: border-box;
    height: 46px;
    border-radius: 15px;
    overflow: hidden;
    background: var(--glass);
    box-shadow: inset 0 0 0 0.5px var(--glass-edge);
    backdrop-filter: blur(24px) saturate(1.6);
    transition:
      width var(--morph),
      opacity 220ms cubic-bezier(0.22, 1, 0.36, 1),
      transform 220ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .pill.rest {
    opacity: 0.92;
    transform: scale(0.99);
  }
  .wash {
    position: absolute;
    inset: 0;
    transition: opacity 500ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  /* Held at full width in both forms and revealed by the growing box, so nothing is
     ever laid out twice and the left edge never moves. */
  .row {
    position: relative;
    box-sizing: border-box;
    height: 46px;
    padding: 0 10px 0 11px;
    display: flex;
    align-items: center;
  }

  .ring {
    position: relative;
    flex: none;
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    color: var(--tint);
  }
  .ring svg {
    position: absolute;
    inset: 0;
    width: 28px;
    height: 28px;
  }
  .ring stop {
    transition: stop-color 400ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .glyph {
    display: grid;
    opacity: 0.75;
    transition: color 400ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .glyph.breathing {
    opacity: 0.55;
    animation: breathe 1.4s ease-in-out infinite alternate;
  }
  @keyframes breathe {
    to {
      opacity: 1;
    }
  }

  .readout {
    flex: none;
    width: 74px;
    display: flex;
    flex-direction: column;
    padding-bottom: 3px;
  }
  .time {
    font: 500 18px/1.15 var(--font-rounded);
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.3px;
    white-space: nowrap;
    color: var(--ink);
  }
  .label {
    font-size: 8px;
    font-weight: 700;
    line-height: 1.3;
    letter-spacing: 0.6px;
    white-space: nowrap;
    color: var(--tint);
    transition: color 200ms ease;
  }
  .time.idle,
  .label.idle {
    color: var(--ink-secondary);
  }

  .control {
    flex: none;
    display: grid;
    place-items: center;
    padding: 0;
    border: 0;
    border-radius: 50%;
    font: inherit;
    cursor: pointer;
    transition:
      transform 130ms cubic-bezier(0.23, 1, 0.32, 1),
      opacity 130ms cubic-bezier(0.23, 1, 0.32, 1);
  }
  .control:active {
    transform: scale(0.94);
    opacity: 0.7;
  }
  .control:focus-visible {
    outline: 2px solid var(--tint);
    outline-offset: 1px;
  }

  /* Opaque tint disc: glass on glass is nearly invisible. */
  .primary {
    position: relative;
    z-index: 2;
    width: 28px;
    height: 28px;
    color: #fff;
    background: linear-gradient(to bottom, color-mix(in srgb, var(--tint) 95%, transparent), var(--tint));
    box-shadow: 0 1px 4px color-mix(in srgb, var(--tint) 45%, transparent);
  }

  /* A recessed well with a hairline, not a second pane of glass. */
  .secondary {
    position: relative;
    width: 26px;
    height: 26px;
    color: var(--ink-secondary);
    background: var(--ink-07);
    box-shadow: inset 0 0 0 0.5px var(--ink-10);
    /* The press scale composes with the slide, so the slide is the base. */
    transform: translateX(0);
    transition:
      transform var(--morph),
      opacity var(--morph);
  }
  .secondary:active {
    transform: translateX(0) scale(0.94);
    transition: transform 130ms cubic-bezier(0.23, 1, 0.32, 1);
  }
  .secondary.parked {
    transform: translateX(var(--tuck));
    opacity: 0;
    pointer-events: none;
  }

  @media (prefers-reduced-motion: reduce) {
    /* The box swaps and only the contents cross-fade. */
    .pill {
      --morph: 10ms linear;
    }
    .secondary {
      transition: opacity 200ms ease-in-out;
    }
    .secondary.parked {
      transform: none;
    }
    .glyph.breathing {
      animation: none;
    }
  }
</style>
