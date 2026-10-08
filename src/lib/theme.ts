export type Phase = "focus" | "shortBreak" | "longBreak";

export const PHASE_TITLES: Record<Phase, string> = {
  focus: "Focus",
  shortBreak: "Short Break",
  longBreak: "Long Break",
};

type Rgb = [number, number, number];

// The same colours as the Mac app's Theme.swift, as 0-255 channels.
const TINT: Record<Phase, Rgb> = {
  focus: [232, 77, 66],
  shortBreak: [51, 168, 135],
  longBreak: [61, 130, 217],
};

// The lighter end of each phase's gradient.
const HIGHLIGHT: Record<Phase, Rgb> = {
  focus: [250, 140, 92],
  shortBreak: [107, 219, 168],
  longBreak: [120, 189, 250],
};

function css([r, g, b]: Rgb, alpha = 1): string {
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

export const tint = (phase: Phase, alpha = 1) => css(TINT[phase], alpha);
export const highlight = (phase: Phase, alpha = 1) => css(HIGHLIGHT[phase], alpha);

/** Wash behind a surface showing this phase: enough colour to read, not enough to cost contrast. */
export function backdrop(phase: Phase): string {
  return `linear-gradient(135deg, ${tint(phase, 0.34)}, ${highlight(phase, 0.1)}, ${tint(phase, 0.04)})`;
}

/** Whole minutes and seconds as mm:ss, never negative. */
export function displayTime(remainingSeconds: number): string {
  const clamped = Math.max(Math.trunc(remainingSeconds), 0);
  const mm = String(Math.floor(clamped / 60)).padStart(2, "0");
  const ss = String(clamped % 60).padStart(2, "0");
  return `${mm}:${ss}`;
}

/** How much of the phase has gone, 0 to 1. */
export function progress(remainingSeconds: number, totalSeconds: number): number {
  if (totalSeconds <= 0) return 0;
  return Math.min(Math.max((totalSeconds - remainingSeconds) / totalSeconds, 0), 1);
}
