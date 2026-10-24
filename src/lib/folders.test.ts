import { describe, expect, test } from "bun:test";
import type { ViewNode } from "./api";
import { assistantPrompt, explainFolder } from "./folders";

const node = (name: string, size: number, isDir = true, grouped = false): ViewNode => ({
  name,
  path: `C:\\Users\\a\\${name}`,
  size,
  files: 3,
  modified: 0,
  kind: "other",
  isDir,
  grouped,
  reclaimable: false,
  children: [],
});

describe("explainFolder", () => {
  test("matches by folder name on both separators", () => {
    expect(explainFolder("C:\\Users\\a\\Downloads")?.whatItIs).toContain("baixados");
    expect(explainFolder("/home/a/node_modules")?.whatItIs).toContain("JavaScript");
  });

  test("local/roaming only inside AppData", () => {
    expect(explainFolder("C:\\Users\\a\\AppData\\Local")).not.toBeNull();
    expect(explainFolder("C:\\Projetos\\Local")).toBeNull();
  });

  test("unknown folders and prototype names give nothing", () => {
    expect(explainFolder("C:\\Users\\a\\Meu Projeto")).toBeNull();
    expect(explainFolder("/tmp/constructor")).toBeNull();
    expect(explainFolder("")).toBeNull();
  });
});

describe("assistantPrompt", () => {
  test("lists the biggest entries first and skips folded buckets", () => {
    const folder = {
      ...node("Pasta", 3000),
      children: [node("a", 10), node("b", 2000), node("pequenos", 900, false, true)],
    };
    const text = assistantPrompt(folder, "Windows");
    expect(text).toContain("Pasta: C:\\Users\\a\\Pasta");
    expect(text.indexOf("- b/")).toBeLessThan(text.indexOf("- a/"));
    expect(text).not.toContain("pequenos");
  });
});
