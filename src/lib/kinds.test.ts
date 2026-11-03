import { describe, expect, test } from "bun:test";
import { KIND_ORDER, KINDS, type Kind, kindColor } from "./kinds";

describe("kinds", () => {
  test("KINDS defines valid label and color for every known kind", () => {
    const expectedKinds: Kind[] = ["cache", "build", "git", "code", "media", "documents", "archives", "apps", "other"];

    expect(KIND_ORDER).toEqual(expectedKinds);
    for (const k of expectedKinds) {
      expect(KINDS[k]).toBeDefined();
      expect(typeof KINDS[k].label).toBe("string");
      expect(KINDS[k].label.length).toBeGreaterThan(0);
      expect(KINDS[k].color).toMatch(/^#[0-9a-fA-F]{6}$/);
      expect(kindColor(k)).toBe(KINDS[k].color);
    }
  });
});
