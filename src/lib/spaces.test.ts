import { describe, expect, test } from "bun:test";
import type { Page } from "../domain";
import { filterCommands, PAGE_COMMANDS, SPACES, spaceOf } from "./spaces";

const ALL: Page[] = [
  "overview",
  "map",
  "bigfiles",
  "review",
  "report",
  "growth",
  "dupes",
  "empty",
  "worktrees",
  "coverage",
  "routine",
  "updates",
];

describe("spaces", () => {
  test("every page lives in exactly one space", () => {
    for (const p of ALL) {
      const homes = SPACES.filter((s) => s.tabs.some((t) => t.page === p));
      expect(homes.length).toBe(1);
    }
    expect(SPACES.flatMap((s) => s.tabs).length).toBe(ALL.length);
  });

  test("spaceOf finds the owning space", () => {
    expect(spaceOf("dupes").id).toBe("limpar");
    expect(spaceOf("growth").id).toBe("explorar");
    expect(spaceOf("updates").id).toBe("ajustes");
  });

  test("the palette reaches every page", () => {
    expect(PAGE_COMMANDS.map((c) => c.page).sort()).toEqual([...ALL].sort());
  });

  test("filter ignores accents, case and word order", () => {
    const hits = filterCommands(PAGE_COMMANDS, "VAZIOS limpar");
    expect(hits.map((c) => c.page)).toEqual(["empty"]);
    expect(filterCommands(PAGE_COMMANDS, "atualizacoes").map((c) => c.page)).toEqual(["updates"]);
    expect(filterCommands(PAGE_COMMANDS, "  ").length).toBe(PAGE_COMMANDS.length);
  });
});
