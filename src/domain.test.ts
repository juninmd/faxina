import { describe, expect, test } from "bun:test";
import {
  BASE_RULES,
  createReport,
  DEFAULT_SETTINGS,
  mergeRulesWithScan,
  type Rule,
  readHistory,
  readSettings,
  recommended,
  saveHistory,
  saveSettings,
  totalSize,
} from "./domain";
import type { JunkItem } from "./lib/api";

const store = new Map<string, string>();
const storageMock: Storage = {
  getItem: (k: string) => store.get(k) ?? null,
  setItem: (k: string, v: string) => store.set(k, String(v)),
  removeItem: (k: string) => store.delete(k),
  clear: () => store.clear(),
  key: (i: number) => Array.from(store.keys())[i] ?? null,
  get length() {
    return store.size;
  },
};
Object.defineProperty(globalThis, "localStorage", {
  value: storageMock,
  writable: true,
});

describe("domain logic", () => {
  test("BASE_RULES contains valid rules and offline is blocked", () => {
    expect(BASE_RULES.length).toBeGreaterThan(10);
    const offline = BASE_RULES.find((r) => r.id === "offline");
    expect(offline).toBeDefined();
    expect(offline?.blocked).toBe(true);
  });

  test("mergeRulesWithScan updates size, files and path", () => {
    const items: JunkItem[] = [
      {
        id: "chrome",
        label: "Google Chrome",
        group: "Navegadores",
        size: 1024 * 1024 * 50,
        files: 120,
        paths: ["C:\\AppData\\Chrome\\Cache"],
      },
      {
        id: "custom_app",
        label: "Custom App",
        group: "Aplicativos",
        size: 1024 * 500,
        files: 15,
        paths: ["C:\\AppData\\CustomApp\\Cache"],
      },
    ];

    const merged = mergeRulesWithScan(items);
    const chrome = merged.find((r) => r.id === "chrome");
    expect(chrome?.size).toBe(1024 * 1024 * 50);
    expect(chrome?.files).toBe(120);
    expect(chrome?.path).toBe("C:\\AppData\\Chrome\\Cache");

    const custom = merged.find((r) => r.id === "custom_app");
    expect(custom).toBeDefined();
    expect(custom?.size).toBe(1024 * 500);
  });

  test("recommended filters out 0-size, blocked, busy, caution, and excluded items", () => {
    const testRules: Rule[] = [
      {
        id: "r1",
        name: "Normal 1",
        app: "App1",
        group: "Sistema",
        size: 1000,
        files: 10,
        path: "/p1",
        consequence: "Safe",
      },
      {
        id: "r2",
        name: "Empty",
        app: "App2",
        group: "Sistema",
        size: 0,
        files: 0,
        path: "/p2",
        consequence: "Safe",
      },
      {
        id: "r3",
        name: "Caution",
        app: "App3",
        group: "Sistema",
        size: 500,
        files: 5,
        path: "/p3",
        consequence: "Safe",
        caution: "Slow reboot",
      },
      {
        id: "r4",
        name: "Busy",
        app: "App4",
        group: "Sistema",
        size: 800,
        files: 8,
        path: "/p4",
        consequence: "Safe",
        busy: true,
      },
      {
        id: "r5",
        name: "Blocked",
        app: "App5",
        group: "Sistema",
        size: 600,
        files: 6,
        path: "/p5",
        consequence: "Safe",
        blocked: true,
      },
      {
        id: "r6",
        name: "Excluded",
        app: "App6",
        group: "Sistema",
        size: 400,
        files: 4,
        path: "/p6",
        consequence: "Safe",
      },
    ];

    const settings = {
      ...DEFAULT_SETTINGS,
      excluded: ["r6"],
    };

    const rec = recommended(testRules, settings);
    expect(rec.map((r) => r.id)).toEqual(["r1"]);
    expect(totalSize(rec)).toBe(1000);
  });

  test("createReport generates valid Report for trash and permanent", () => {
    const testRules: Rule[] = [
      {
        id: "r1",
        name: "Rule 1",
        app: "App1",
        group: "Sistema",
        size: 1000,
        files: 5,
        path: "/p1",
        consequence: "Safe",
      },
    ];

    const repTrash = createReport(testRules, "trash", 1000, 0, 5);
    expect(repTrash.destination).toBe("trash");
    expect(repTrash.trashed).toBe(1000);
    expect(repTrash.permanent).toBe(0);
    expect(repTrash.removed).toBe(1000);
    expect(repTrash.files).toBe(5);

    const repPerm = createReport(testRules, "permanent", 1000, 0, 5);
    expect(repPerm.destination).toBe("permanent");
    expect(repPerm.trashed).toBe(0);
    expect(repPerm.permanent).toBe(1000);
    expect(repPerm.removed).toBe(1000);
  });

  test("settings and history fallback gracefully when localStorage has invalid data", () => {
    localStorage.clear();
    expect(readSettings()).toEqual(DEFAULT_SETTINGS);
    expect(readHistory()).toEqual([]);

    localStorage.setItem("faxina-settings-v1", "invalid json");
    expect(readSettings()).toEqual(DEFAULT_SETTINGS);

    localStorage.setItem("faxina-history-v1", JSON.stringify([{ id: "1", invalid: true }]));
    expect(readHistory()).toEqual([]);
  });

  test("saveSettings and saveHistory persist correctly", () => {
    const s = {
      excluded: ["test_id"],
      schedule: true,
      frequency: "Mensal",
      time: "20:30",
      reminder: false,
    };
    expect(saveSettings(s)).toBe(true);
    expect(readSettings()).toEqual(s);

    const h = [
      {
        id: "rep-1",
        date: new Date().toISOString(),
        selected: 500,
        removed: 500,
        trashed: 0,
        permanent: 500,
        observed: 500,
        skipped: 0,
        files: 3,
        destination: "permanent" as const,
      },
    ];
    expect(saveHistory(h)).toBe(true);
    expect(readHistory()).toEqual(h);
  });
});
