import { describe, expect, test } from "bun:test";
import { platformName } from "./platform";

describe("platformName", () => {
  test("identifies Windows user agents", () => {
    const originalUa = navigator.userAgent;
    try {
      Object.defineProperty(navigator, "userAgent", {
        value: "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
        configurable: true,
      });
      expect(platformName()).toBe("Windows");
    } finally {
      Object.defineProperty(navigator, "userAgent", {
        value: originalUa,
        configurable: true,
      });
    }
  });

  test("identifies macOS user agents", () => {
    const originalUa = navigator.userAgent;
    try {
      Object.defineProperty(navigator, "userAgent", {
        value: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
        configurable: true,
      });
      expect(platformName()).toBe("macOS");
    } finally {
      Object.defineProperty(navigator, "userAgent", {
        value: originalUa,
        configurable: true,
      });
    }
  });

  test("identifies Linux user agents", () => {
    const originalUa = navigator.userAgent;
    try {
      Object.defineProperty(navigator, "userAgent", {
        value: "Mozilla/5.0 (X11; Linux x86_64)",
        configurable: true,
      });
      expect(platformName()).toBe("Linux");
    } finally {
      Object.defineProperty(navigator, "userAgent", {
        value: originalUa,
        configurable: true,
      });
    }
  });

  test("falls back to 'Seu sistema' for unknown platforms", () => {
    const originalUa = navigator.userAgent;
    try {
      Object.defineProperty(navigator, "userAgent", {
        value: "CustomAgent/1.0",
        configurable: true,
      });
      expect(platformName()).toBe("Seu sistema");
    } finally {
      Object.defineProperty(navigator, "userAgent", {
        value: originalUa,
        configurable: true,
      });
    }
  });
});
