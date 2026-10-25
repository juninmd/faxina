import { describe, expect, test } from "bun:test";
import type { DupGroup } from "./api";
import { defaultPick, dupNotice, pruneDeleted } from "./dupes";

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
