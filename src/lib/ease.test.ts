import { describe, expect, test } from "bun:test";
import { countValue, easeOutCubic } from "./ease";

describe("countValue", () => {
  test("starts at the origin and lands exactly on the target", () => {
    expect(countValue(10, 50, 0, 500)).toBe(10);
    expect(countValue(10, 50, 500, 500)).toBe(50);
    expect(countValue(10, 50, 900, 500)).toBe(50);
  });

  test("eases out: more than half way at half the time, counting down too", () => {
    expect(countValue(0, 100, 250, 500)).toBeGreaterThan(50);
    expect(countValue(100, 0, 250, 500)).toBeLessThan(50);
  });

  test("a zero duration jumps straight to the target", () => {
    expect(countValue(1, 9, 0, 0)).toBe(9);
  });

  test("clamps the easing input", () => {
    expect(easeOutCubic(-1)).toBe(0);
    expect(easeOutCubic(2)).toBe(1);
  });
});
