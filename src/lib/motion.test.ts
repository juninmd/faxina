import { describe, expect, test } from "bun:test";
import { getMotionPref, MOTION_LABELS, setMotionPref, shouldAnimate } from "./motion";

describe("motion", () => {
  test("MOTION_LABELS has localized pt-BR values", () => {
    expect(MOTION_LABELS.system).toBe("sistema");
    expect(MOTION_LABELS.always).toBe("sempre");
    expect(MOTION_LABELS.never).toBe("nunca");
  });

  test("getMotionPref and setMotionPref roundtrip with localStorage", () => {
    try {
      setMotionPref("always");
      expect(getMotionPref()).toBe("always");

      setMotionPref("never");
      expect(getMotionPref()).toBe("never");

      setMotionPref("system");
      expect(getMotionPref()).toBe("system");

      localStorage.setItem("faxina.motion", "invalid_val");
      expect(getMotionPref()).toBe("system");
    } finally {
      localStorage.removeItem("faxina.motion");
    }
  });

  test("shouldAnimate respects explicit preference", () => {
    expect(shouldAnimate("always")).toBe(true);
    expect(shouldAnimate("never")).toBe(false);
  });
});
