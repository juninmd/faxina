import { expect, test } from "bun:test";
import { formatElapsed } from "../components/ScanProgress";

test("formats elapsed time as mm:ss", () => {
  expect(formatElapsed(0)).toBe("00:00");
  expect(formatElapsed(75.9)).toBe("01:15");
  expect(formatElapsed(-3)).toBe("00:00");
});
