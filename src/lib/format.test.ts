import { describe, expect, test } from "bun:test";
import { baseName, formatAge, formatBytes, formatCount, parentOf } from "./format";

describe("formatBytes", () => {
  test("uses pt-BR decimals and binary units", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1536)).toBe("1,5 KB");
    expect(formatBytes(12.4 * 1024 ** 3)).toBe("12,4 GB");
  });

  test("promotes to the next unit instead of rounding up to 1.024", () => {
    expect(formatBytes(1024 * 1024 - 1)).toBe("1 MB");
    expect(formatBytes(1023.6 * 1024)).toBe("1 MB");
    expect(formatBytes(1024 ** 3 - 1)).toBe("1 GB");
    expect(formatBytes(1023 * 1024)).toBe("1.023 KB");
    expect(formatBytes(2 * 1024 ** 4)).toBe("2 TB");
  });

  test("the largest unit never promotes past TB", () => {
    expect(formatBytes(1024 ** 5)).toBe("1.024 TB");
  });

  test("never renders garbage for bad input", () => {
    expect(formatBytes(-1)).toBe("0 B");
    expect(formatBytes(Number.NaN)).toBe("0 B");
  });
});

describe("formatCount", () => {
  test("thousands and millions", () => {
    expect(formatCount(999)).toBe("999");
    expect(formatCount(1500)).toBe("1,5 mil");
    expect(formatCount(2_500_000)).toBe("2,5 mi");
  });

  test("999.950 is shown as 1 mi, not 1.000 mil", () => {
    expect(formatCount(999_950)).toBe("1 mi");
    expect(formatCount(999_949)).toBe("999,9 mil");
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
