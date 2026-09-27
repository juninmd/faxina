import { describe, expect, test } from "bun:test";
import { buildManifest } from "./updater-manifest";

const names = [
  "Faxina_1.2.0_x64-setup.exe",
  "Faxina_1.2.0_x64_en-US.msi",
  "Faxina_1.2.0_aarch64.app.tar.gz",
  "Faxina_1.2.0_x64.app.tar.gz",
  "Faxina_1.2.0_aarch64.dmg",
  "Faxina_1.2.0_amd64.AppImage",
  "Faxina_1.2.0_amd64.deb",
  "Faxina-1.2.0-1.x86_64.rpm",
];
const assets = names.map((name) => ({ name, url: `https://dl/${name}` }));
const sigs = Object.fromEntries(names.filter((n) => !n.endsWith(".dmg")).map((n) => [n, `sig-of-${n}\n`]));

describe("buildManifest", () => {
  test("covers every OS, not just the last build to finish", () => {
    const m = buildManifest("1.2.0", "notes", "2026-09-27T00:00:00Z", assets, sigs);
    for (const k of ["windows-x86_64", "darwin-aarch64", "darwin-x86_64", "linux-x86_64"]) {
      expect(m.platforms[k]).toBeDefined();
    }
    expect(m.platforms["darwin-x86_64"].url).toBe("https://dl/Faxina_1.2.0_x64.app.tar.gz");
    expect(m.platforms["windows-x86_64"].url).toEndWith("_x64-setup.exe");
    expect(m.platforms["linux-x86_64"].signature).toBe("sig-of-Faxina_1.2.0_amd64.AppImage");
  });

  test("an unsigned bundle is never offered to the updater", () => {
    const { "Faxina_1.2.0_amd64.AppImage": _, ...partial } = sigs;
    const m = buildManifest("1.2.0", "", "", assets, partial);
    expect(m.platforms["linux-x86_64"]).toBeUndefined();
    expect(m.platforms["darwin-aarch64"]).toBeDefined();
  });
});
