// Builds a fake "home folder" with caches, build output, duplicates and stale files.
// Usage: bun scripts/demo-fixture.ts <target-dir>
import { randomBytes } from "node:crypto";
import { closeSync, existsSync, ftruncateSync, mkdirSync, openSync, utimesSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

const root = process.argv[2];
if (!root) {
  console.error("usage: bun scripts/demo-fixture.ts <target-dir>");
  process.exit(1);
}
if (existsSync(root)) {
  console.error(`refusing to write into existing path: ${root}`);
  process.exit(1);
}

const MB = 1024 * 1024;
const YEAR = 365 * 24 * 3600 * 1000;

/** Extends the file without writing data, so gigabytes cost nothing to create. */
function big(rel: string, sizeMb: number, ageMs = 0) {
  const p = join(root, rel);
  mkdirSync(dirname(p), { recursive: true });
  const fd = openSync(p, "w");
  ftruncateSync(fd, Math.round(sizeMb * MB));
  closeSync(fd);
  if (ageMs) {
    const t = new Date(Date.now() - ageMs);
    utimesSync(p, t, t);
  }
}

function text(rel: string, body: string) {
  const p = join(root, rel);
  mkdirSync(dirname(p), { recursive: true });
  writeFileSync(p, body);
}

function real(rel: string, data: Buffer) {
  const p = join(root, rel);
  mkdirSync(dirname(p), { recursive: true });
  writeFileSync(p, data);
}

// Web project with dependencies and framework cache.
text("Projetos/loja-web/package.json", '{"name":"loja-web"}');
for (const [pkg, mb] of [
  ["next", 180],
  ["@swc", 140],
  ["typescript", 45],
  ["react-dom", 12],
  ["lodash", 8],
  ["esbuild", 22],
] as const) {
  big(`Projetos/loja-web/node_modules/${pkg}/dist/index.js`, mb);
}
big("Projetos/loja-web/.next/cache/webpack/client.pack", 320);
big("Projetos/loja-web/src/app.tsx", 0.6);

// Rust project with a fat target/ and git history.
text("Projetos/motor-rust/Cargo.toml", "[package]\nname='motor'");
big("Projetos/motor-rust/target/debug/deps/libmotor.rlib", 640);
big("Projetos/motor-rust/target/debug/incremental/motor.bin", 380);
big("Projetos/motor-rust/target/release/motor.exe", 90);
big("Projetos/motor-rust/.git/objects/pack/pack-1.pack", 240);
big("Projetos/motor-rust/src/main.rs", 0.7);

// Python project cache.
text("Projetos/analise/pyproject.toml", "[project]\nname='analise'");
big("Projetos/analise/.venv/lib/site-packages/numpy/core.so", 110);
big("Projetos/analise/__pycache__/model.cpython-313.pyc", 60);

// Photos with a phone backup full of real duplicates (same bytes).
for (let i = 1; i <= 8; i++) {
  const photo = randomBytes(2 * MB + i * 1000);
  real(`Fotos/2025/viagem-${i}.jpg`, photo);
  if (i <= 5) real(`Fotos/backup-celular/IMG_${1000 + i}.jpg`, photo);
}
const raw = randomBytes(4 * MB);
real("Fotos/2024/natal.png", raw);
real("Downloads/natal (1).png", raw);

// Big and stale media, archives and installers.
big("Vídeos/aniversario-2023.mp4", 1400, 2 * YEAR);
big("Vídeos/show.mkv", 900, 0.2 * YEAR);
big("Música/album.flac", 385);
big("Downloads/ubuntu-24.04.iso", 700, 1.5 * YEAR);
big("Downloads/instalador.exe", 160, YEAR);
big("Downloads/fotos-antigas.zip", 520, YEAR);
big("Documentos/relatorio-anual.pdf", 24);
big("Documentos/planilha.xlsx", 6);
text("Documentos/notas.md", "# notas\n");

// Tool caches.
big(".cache/pip/http/wheels.bin", 210);
big(".cache/thumbnails/large.db", 95);
big("AppData/Local/Temp/setup.log", 44, YEAR);

console.log(`demo fixture ready at ${root}`);
