import { describe, expect, test } from "bun:test";
import type { DupGroup } from "./api";
import { defaultPick, dupNotice, groupPick, keeperOf, pruneDeleted } from "./dupes";

const group = (hash: string, ...paths: string[]): DupGroup => ({
  hash,
  size: 10,
  files: paths.map((path) => ({ path, modified: 0 })),
});

describe("pruneDeleted", () => {
  test("a copy that failed to delete stays listed: it is still on disk", () => {
    const gs = [group("a", "/o", "/c1", "/c2")];
    const left = pruneDeleted(gs, ["/c1"]);
    expect(left[0].files.map((f) => f.path)).toEqual(["/o", "/c2"]);
  });

  test("a group with a single survivor is no longer a duplicate", () => {
    expect(pruneDeleted([group("a", "/o", "/c1"), group("b", "/x", "/y")], ["/c1"])).toEqual([group("b", "/x", "/y")]);
  });
});

test("defaultPick keeps the first (oldest) file of every group", () => {
  expect([...defaultPick([group("a", "/o", "/c1", "/c2")])]).toEqual(["/c1", "/c2"]);
});

describe("dupNotice", () => {
  test("returns cancelled message when cancelled is true", () => {
    expect(dupNotice({ cancelled: true, truncated: false })).toBe("Busca cancelada: resultado parcial");
    expect(dupNotice({ cancelled: true, truncated: true })).toBe("Busca cancelada: resultado parcial");
  });

  test("returns truncated message when truncated is true and cancelled is false", () => {
    expect(dupNotice({ cancelled: false, truncated: true })).toBe(
      "Mostrando os 500 grupos com mais espaço desperdiçado",
    );
  });

  test("returns null when neither cancelled nor truncated", () => {
    expect(dupNotice({ cancelled: false, truncated: false })).toBeNull();
  });
});

describe("keeper rules", () => {
  const g: DupGroup = {
    hash: "k",
    size: 10,
    files: [
      { path: "/a/photos/original-name.png", modified: 100 },
      { path: "/a/b.png", modified: 300 },
      { path: "/a/photos/deep/q", modified: 200 },
    ],
  };

  test("oldest keeps the first listed file", () => {
    expect(keeperOf(g, "oldest")).toBe("/a/photos/original-name.png");
  });

  test("newest keeps the highest mtime", () => {
    expect(keeperOf(g, "newest")).toBe("/a/b.png");
  });

  test("shortest path and shortest name pick different copies", () => {
    expect(keeperOf(g, "shortestPath")).toBe("/a/b.png");
    expect(keeperOf(g, "shortestName")).toBe("/a/photos/deep/q");
  });

  test("a group pick never includes the keeper, so one copy always survives", () => {
    for (const rule of ["oldest", "newest", "shortestPath", "shortestName"] as const) {
      const picked = groupPick(g, rule);
      expect(picked).toHaveLength(2);
      expect(picked).not.toContain(keeperOf(g, rule));
    }
  });

  test("defaultPick follows the rule for every group", () => {
    expect([...defaultPick([g], "newest")].sort()).toEqual(["/a/photos/deep/q", "/a/photos/original-name.png"]);
  });
});
