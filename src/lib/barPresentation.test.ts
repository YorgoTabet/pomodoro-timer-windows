import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { BarPresentationModel, barMode, type BarPolicy } from "./barPresentation";

const policy = (over: Partial<BarPolicy> = {}): BarPolicy => ({
  compactEnabled: true,
  isRunning: true,
  hovering: false,
  awaitingAcknowledgement: false,
  ...over,
});

describe("barMode", () => {
  it("is compact only when nothing vetoes it", () => {
    expect(barMode(policy())).toBe("compact");
  });
  it("stays expanded when the setting is off", () => {
    expect(barMode(policy({ compactEnabled: false }))).toBe("expanded");
  });
  it("stays expanded while paused", () => {
    expect(barMode(policy({ isRunning: false }))).toBe("expanded");
  });
  it("opens under the pointer", () => {
    expect(barMode(policy({ hovering: true }))).toBe("expanded");
  });
  it("holds open for an unseen phase change, outranking running", () => {
    expect(barMode(policy({ awaitingAcknowledgement: true }))).toBe("expanded");
  });
});

describe("BarPresentationModel", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  const running = () => {
    const m = new BarPresentationModel(() => {}, 30_000);
    m.setCompactEnabled(true);
    m.setRunning(true);
    return m;
  };

  it("collapses a running bar", () => {
    expect(running().mode).toBe("compact");
  });

  it("holds the bar open after a phase change until the pointer enters and leaves", () => {
    const m = running();
    m.beginPhaseChange();
    expect(m.mode).toBe("expanded");
    m.setHovering(true);
    m.setHovering(false);
    expect(m.mode).toBe("compact");
  });

  it("does not count a stray leave as having looked", () => {
    const m = running();
    m.beginPhaseChange();
    m.setHovering(false);
    expect(m.mode).toBe("expanded");
  });

  it("gives up waiting after the backstop", () => {
    const m = running();
    m.beginPhaseChange();
    vi.advanceTimersByTime(29_999);
    expect(m.mode).toBe("expanded");
    vi.advanceTimersByTime(1);
    expect(m.mode).toBe("compact");
  });

  it("restarts the backstop on a second phase change", () => {
    const m = running();
    m.beginPhaseChange();
    vi.advanceTimersByTime(20_000);
    m.beginPhaseChange();
    vi.advanceTimersByTime(20_000);
    expect(m.mode).toBe("expanded");
    vi.advanceTimersByTime(10_000);
    expect(m.mode).toBe("compact");
  });

  it("reports every change", () => {
    const seen: BarPolicy[] = [];
    const m = new BarPresentationModel((p) => seen.push(p));
    m.setRunning(true);
    m.setHovering(true);
    m.setHovering(true);
    expect(seen).toHaveLength(2);
  });
});
