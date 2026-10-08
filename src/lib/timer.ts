import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { readable, type Readable } from "svelte/store";
import type { Phase } from "./theme";

export interface TimerState {
  phase: Phase;
  isRunning: boolean;
  remainingSeconds: number;
  totalSeconds: number;
  completedFocusSessions: number;
}

export interface Settings {
  focusMinutes: number;
  shortBreakMinutes: number;
  longBreakMinutes: number;
  pomodorosUntilLongBreak: number;
  autoStartBreaks: boolean;
  autoStartFocus: boolean;
  notificationsEnabled: boolean;
  chimeEnabled: boolean;
  chimeVolume: number;
  focusChime: string;
  breakChime: string;
  controlMusic: boolean;
  resumeMusicOnFocusStart: boolean;
  showFloatingBar: boolean;
  compactFloatingBar: boolean;
  launchAtLogin: boolean;
  character: string;
  floatingBarX: number | null;
  floatingBarY: number | null;
}

export const DEFAULT_STATE: TimerState = {
  phase: "focus",
  isRunning: false,
  remainingSeconds: 25 * 60,
  totalSeconds: 25 * 60,
  completedFocusSessions: 0,
};

export const DEFAULT_SETTINGS: Settings = {
  focusMinutes: 25,
  shortBreakMinutes: 5,
  longBreakMinutes: 15,
  pomodorosUntilLongBreak: 4,
  autoStartBreaks: true,
  autoStartFocus: false,
  notificationsEnabled: true,
  chimeEnabled: true,
  chimeVolume: 1,
  focusChime: "Hero",
  breakChime: "Blow",
  controlMusic: true,
  resumeMusicOnFocusStart: true,
  showFloatingBar: true,
  compactFloatingBar: false,
  launchAtLogin: false,
  character: "none",
  floatingBarX: null,
  floatingBarY: null,
};

/** A plain browser (the dev server, a screenshot) has no Tauri behind it. */
export const inTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/**
 * A store fed once by a command and then by every event of one name. The listener
 * is registered before the first read, so a change in between cannot be lost.
 */
function backend<T>(initial: T, command: string, event: string): Readable<T> {
  return readable(initial, (set) => {
    if (!inTauri()) return;
    let stopped = false;
    let unlisten: UnlistenFn | undefined;
    listen<T>(event, (e) => set(e.payload)).then((fn) => (stopped ? fn() : (unlisten = fn)));
    invoke<T>(command).then((value) => !stopped && set(value));
    return () => {
      stopped = true;
      unlisten?.();
    };
  });
}

export const timerState = backend<TimerState>(DEFAULT_STATE, "get_state", "timer-state");
export const settings = backend<Settings>(DEFAULT_SETTINGS, "get_settings", "settings-changed");

/** Commands the windows send. Each is a no-op outside Tauri. */
export const timerCommands = {
  toggle: () => inTauri() && invoke("toggle"),
  skip: () => inTauri() && invoke("skip"),
  reset: () => inTauri() && invoke("reset"),
};

/** Fires on `phase-elapsed`, which the bar uses to hold itself open. */
export function onPhaseElapsed(handler: () => void): () => void {
  if (!inTauri()) return () => {};
  let stopped = false;
  let unlisten: UnlistenFn | undefined;
  listen("phase-elapsed", handler).then((fn) => (stopped ? fn() : (unlisten = fn)));
  return () => {
    stopped = true;
    unlisten?.();
  };
}
