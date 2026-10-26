import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

// Mojibake: UTF-8 bytes of a character decoded as Latin-1, e.g. "ç" -> "Ã§".
const MOJIBAKE = /Ã[\u0080-¿]/;

test("tauri.conf.json não contém mojibake nas strings de metadados", () => {
  const raw = readFileSync(join(import.meta.dir, "..", "src-tauri", "tauri.conf.json"), "utf8");
  const conf = JSON.parse(raw);
  expect(conf.bundle.shortDescription).not.toMatch(MOJIBAKE);
  expect(conf.bundle.shortDescription).toContain("espaço");
});
