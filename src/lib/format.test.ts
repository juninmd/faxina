import { describe, expect, test } from "bun:test";
import { baseName, formatAge, formatBytes, parentOf } from "./format";

describe("formatBytes", () => {
  test("uses pt-BR decimals and binary units", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1536)).toBe("1,5 KB");
    expect(formatBytes(12.4 * 1024 ** 3)).toBe("12,4 GB");
  });

  test("never renders garbage for bad input", () => {
    expect(formatBytes(-1)).toBe("0 B");
    expect(formatBytes(Number.NaN)).toBe("0 B");
  });
});

describe("formatAge", () => {
  const now = Date.UTC(2026, 8, 27);
  test("singular and plural", () => {
    expect(formatAge(now / 1000 - 3600, now)).toBe("há 1 hora");
    expect(formatAge(now / 1000 - 3 * 86400, now)).toBe("há 3 dias");
  });
  test("unknown timestamps show a dash", () => {
    expect(formatAge(0, now)).toBe("—");
  });
});

describe("paths", () => {
  test("parent never climbs above the scan root", () => {
    expect(parentOf("C:\\Users\\me\\a\\b", "C:\\Users\\me")).toBe("C:\\Users\\me\\a");
    expect(parentOf("C:\\Users\\me\\a", "C:\\Users\\me")).toBe("C:\\Users\\me");
    expect(parentOf("C:\\Users\\me", "C:\\Users\\me")).toBeNull();
    expect(parentOf("/home/u/x", "/home/u")).toBe("/home/u");
  });

  test("baseName handles both separators", () => {
    expect(baseName("C:\\a\\foto.jpg")).toBe("foto.jpg");
    expect(baseName("/a/b/")).toBe("b");
  });
});
