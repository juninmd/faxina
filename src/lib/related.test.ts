import { describe, expect, test } from "bun:test";
import { related } from "../components/Treemap";

describe("related", () => {
  test("keeps the hovered tile, its parents and its children lit", () => {
    expect(related("/a/b", "/a/b")).toBe(true);
    expect(related("/a", "/a/b")).toBe(true);
    expect(related("/a/b/c", "/a/b")).toBe(true);
  });

  test("dims siblings and look-alike prefixes", () => {
    expect(related("/a/c", "/a/b")).toBe(false);
    expect(related("/a/bb", "/a/b")).toBe(false);
    expect(related("C:\\x\\y2", "C:\\x\\y")).toBe(false);
    expect(related("C:\\x\\y\\z", "C:\\x\\y")).toBe(true);
  });
});
