import { describe, expect, test } from "bun:test";
import { EMPTY_FORM, exportName, isBlank, toQuery } from "./search";

describe("search form", () => {
  test("blank until a filter is set", () => {
    expect(isBlank(EMPTY_FORM)).toBe(true);
    expect(isBlank({ ...EMPTY_FORM, text: "  " })).toBe(true);
    expect(isBlank({ ...EMPTY_FORM, text: "log" })).toBe(false);
    expect(isBlank({ ...EMPTY_FORM, ageMonths: 6 })).toBe(false);
  });

  test("age becomes seconds of 30-day months", () => {
    expect(toQuery({ ...EMPTY_FORM, ageMonths: 12 }).olderThanSecs).toBe(12 * 30 * 86400);
    expect(toQuery(EMPTY_FORM).olderThanSecs).toBe(0);
  });

  test("folders are dropped when an age is asked", () => {
    expect(toQuery({ ...EMPTY_FORM, includeDirs: true }).includeDirs).toBe(true);
    expect(toQuery({ ...EMPTY_FORM, includeDirs: true, ageMonths: 6 }).includeDirs).toBe(false);
  });

  test("text is trimmed", () => {
    expect(toQuery({ ...EMPTY_FORM, text: " *.log " }).text).toBe("*.log");
  });

  test("export name carries the date and format", () => {
    expect(exportName("csv", new Date("2026-03-04T10:00:00Z"))).toBe("faxina-busca-2026-03-04.csv");
  });
});
