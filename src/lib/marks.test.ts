import { describe, expect, test } from "bun:test";
import { type Mark, toggleMarkMap } from "./marks";

const mark = (path: string, size = 100): Mark => ({
  path,
  name: path.split(/[\\/]/).pop() ?? path,
  size,
  kind: "build",
});

describe("toggleMarkMap", () => {
  test("adds and removes a mark", () => {
    const added = toggleMarkMap(new Map(), mark("C:\\proj\\dist"));
    expect([...added.keys()]).toEqual(["C:\\proj\\dist"]);
    const removed = toggleMarkMap(added, mark("C:\\proj\\dist"));
    expect(removed.size).toBe(0);
  });

  test("marking a parent drops its marked descendants", () => {
    let m = toggleMarkMap(new Map(), mark("/r/app/node_modules", 80));
    m = toggleMarkMap(m, mark("/r/app/dist", 20));
    m = toggleMarkMap(m, mark("/r/other", 10));
    m = toggleMarkMap(m, mark("/r/app", 100));
    expect([...m.keys()].sort()).toEqual(["/r/app", "/r/other"]);
  });

  test("marking a child of an already marked parent does not double-count", () => {
    const parent = toggleMarkMap(new Map(), mark("C:\\proj\\app", 500));
    const after = toggleMarkMap(parent, mark("C:\\proj\\app\\node_modules", 400));
    expect([...after.keys()]).toEqual(["C:\\proj\\app"]);
  });

  test("look-alike sibling prefixes are independent", () => {
    let m = toggleMarkMap(new Map(), mark("/r/app", 100));
    m = toggleMarkMap(m, mark("/r/app-v2", 200));
    expect([...m.keys()].sort()).toEqual(["/r/app", "/r/app-v2"]);
  });
});
