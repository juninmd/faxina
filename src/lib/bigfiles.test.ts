import { describe, expect, test } from "bun:test";
import type { BigFile } from "./api";
import { kindCounts, sortFiles } from "./bigfiles";

const f = (path: string, size: number, modified: number, kind: BigFile["kind"] = "other"): BigFile => ({
  path,
  size,
  modified,
  kind,
});

const files = [
  f("/a/zeta.iso", 5, 300, "archives"),
  f("/a/Alpha.mp4", 9, 100, "media"),
  f("/b/beta.mp4", 9, 200, "media"),
];

describe("sortFiles", () => {
  test("size is biggest first with a stable path tiebreak", () => {
    expect(sortFiles(files, "size").map((x) => x.path)).toEqual(["/a/Alpha.mp4", "/b/beta.mp4", "/a/zeta.iso"]);
  });

  test("age puts the oldest modification first", () => {
    expect(sortFiles(files, "age").map((x) => x.modified)).toEqual([100, 200, 300]);
  });

  test("name ignores the folder", () => {
    expect(sortFiles(files, "name").map((x) => x.path)).toEqual(["/a/Alpha.mp4", "/b/beta.mp4", "/a/zeta.iso"]);
  });

  test("does not mutate its input", () => {
    const copy = [...files];
    sortFiles(files, "age");
    expect(files).toEqual(copy);
  });
});

test("kindCounts tallies every kind present", () => {
  expect(kindCounts(files).get("media")).toBe(2);
  expect(kindCounts(files).get("archives")).toBe(1);
  expect(kindCounts(files).get("code")).toBeUndefined();
});
