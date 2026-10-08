import { describe, expect, it } from "vitest";
import { compactWidth, primaryX, secondaryX, tuck, width } from "./barMetrics";

describe("bar metrics", () => {
  it("add up to the Swift pill", () => {
    expect(primaryX).toBe(134);
    expect(width).toBe(236);
    expect(compactWidth).toBe(172);
  });
  it("park each secondary control under the play button", () => {
    expect(secondaryX(0) + tuck(0)).toBe(primaryX);
    expect(secondaryX(1) + tuck(1)).toBe(primaryX);
  });
});
