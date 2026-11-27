import { describe, expect, test } from "bun:test";
import type { Worktree, WorktreeAllowance } from "./api";
import { canPick, optInFor } from "./worktrees";

const item = (path: string, allowance: WorktreeAllowance): Worktree => ({
  path,
  kind: "worktree",
  branch: "feature/x",
  head: "abc12345",
  modified: 0,
  size: 1,
  files: 1,
  ignored: 0,
  allowance,
  reason: allowance === "synced" ? null : "motivo",
  ahead: 0,
  changes: 0,
});

describe("canPick", () => {
  test("never is never offered", () => {
    expect(canPick(item("/w", "never"), false)).toBe(false);
    expect(canPick(item("/w", "never"), true)).toBe(false);
  });

  test("definitive waits for the damage switch", () => {
    expect(canPick(item("/w", "definitive"), false)).toBe(false);
    expect(canPick(item("/w", "definitive"), true)).toBe(true);
  });

  test("synced and recoverable need no switch", () => {
    expect(canPick(item("/w", "synced"), false)).toBe(true);
    expect(canPick(item("/w", "recoverable"), false)).toBe(true);
  });
});

describe("optInFor", () => {
  test("the batch travels at the least level that covers it", () => {
    expect(optInFor([item("/a", "synced"), item("/b", "synced")])).toBe("synced");
    expect(optInFor([item("/a", "synced"), item("/b", "recoverable")])).toBe("local");
    expect(optInFor([item("/a", "recoverable"), item("/b", "definitive")])).toBe("definitive");
  });

  test("an empty selection grants nothing", () => {
    expect(optInFor([])).toBe("synced");
  });
});
