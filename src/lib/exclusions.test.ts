import { describe, expect, test } from "bun:test";
import { MAX_EXCLUDED_NAMES, parseExcludedNames } from "./exclusions";

describe("parseExcludedNames", () => {
  test("splits on commas and line breaks, trims and removes repeats", () => {
    expect(parseExcludedNames(" node_modules, Backups\n\nnode_modules ,  ").names).toEqual(["node_modules", "Backups"]);
  });

  test("an empty field means no exclusions", () => {
    expect(parseExcludedNames("")).toEqual({ names: [], error: null });
  });

  test("paths, wildcards, drives and dot names are refused", () => {
    for (const bad of ["a/b", "a\\b", "*.tmp", "C:", "..", "."]) {
      expect(parseExcludedNames(bad).error).not.toBeNull();
    }
  });

  test("too many names are refused", () => {
    const text = Array.from({ length: MAX_EXCLUDED_NAMES + 1 }, (_, i) => `n${i}`).join(",");
    expect(parseExcludedNames(text).error).toContain(String(MAX_EXCLUDED_NAMES));
  });
});
