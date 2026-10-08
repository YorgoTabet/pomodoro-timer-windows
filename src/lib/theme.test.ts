import { describe, expect, it } from "vitest";
import { displayTime, progress } from "./theme";

describe("displayTime", () => {
  it("pads minutes and seconds", () => {
    expect(displayTime(1453)).toBe("24:13");
    expect(displayTime(65)).toBe("01:05");
    expect(displayTime(0)).toBe("00:00");
  });
  it("never goes negative", () => {
    expect(displayTime(-3)).toBe("00:00");
  });
  it("shows an hour as minutes", () => {
    expect(displayTime(3600)).toBe("60:00");
  });
});

describe("progress", () => {
  it("is the elapsed share of the phase", () => {
    expect(progress(1500, 1500)).toBe(0);
    expect(progress(750, 1500)).toBe(0.5);
    expect(progress(0, 1500)).toBe(1);
  });
  it("stays in 0 to 1 and survives a zero total", () => {
    expect(progress(-5, 1500)).toBe(1);
    expect(progress(2000, 1500)).toBe(0);
    expect(progress(0, 0)).toBe(0);
  });
});
