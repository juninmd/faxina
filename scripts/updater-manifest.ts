// Builds the Tauri updater's latest.json from a release's assets in one place, so parallel
// platform builds can't overwrite each other's entries.
// Usage (CI): bun scripts/updater-manifest.ts <release-id> <owner/repo> > latest.json
// Takes the id, not the tag: drafts are invisible to the releases/tags endpoint.

export interface Asset {
  name: string;
  url: string;
}

/** Updater platform key -> asset name suffix. The first key per OS is the default target. */
const TARGETS: [string, RegExp][] = [
  ["windows-x86_64", /_x64-setup\.exe$/],
  ["windows-x86_64-nsis", /_x64-setup\.exe$/],
  ["windows-x86_64-msi", /_x64_[\w-]+\.msi$/],
  ["darwin-aarch64", /_aarch64\.app\.tar\.gz$/],
  ["darwin-aarch64-app", /_aarch64\.app\.tar\.gz$/],
  ["darwin-x86_64", /_x64\.app\.tar\.gz$/],
  ["darwin-x86_64-app", /_x64\.app\.tar\.gz$/],
  ["linux-x86_64", /_amd64\.AppImage$/],
  ["linux-x86_64-appimage", /_amd64\.AppImage$/],
  ["linux-x86_64-deb", /_amd64\.deb$/],
  ["linux-x86_64-rpm", /\.x86_64\.rpm$/],
];

export function buildManifest(
  version: string,
  notes: string,
  pubDate: string,
  assets: Asset[],
  signatures: Record<string, string>,
) {
  const platforms: Record<string, { signature: string; url: string }> = {};
  for (const [key, pattern] of TARGETS) {
    const asset = assets.find((a) => pattern.test(a.name));
    const signature = asset && signatures[asset.name]?.trim();
    if (asset && signature) platforms[key] = { signature, url: asset.url };
  }
  return { version, notes, pub_date: pubDate, platforms };
}

async function gh(args: string[]): Promise<string> {
  const proc = Bun.spawn(["gh", ...args], { stdout: "pipe", stderr: "inherit" });
  const out = await new Response(proc.stdout).text();
  if ((await proc.exited) !== 0) throw new Error(`gh ${args.join(" ")} failed`);
  return out;
}

if (import.meta.main) {
  const [id, repo] = process.argv.slice(2);
  if (!id || !repo) throw new Error("usage: updater-manifest.ts <release-id> <owner/repo>");
  const release = JSON.parse(await gh(["api", `repos/${repo}/releases/${id}`])) as {
    tag_name: string;
    body: string;
    assets: { name: string; url: string }[];
  };
  // Draft download URLs point at "untagged-…" and break once published; the tag URL is stable.
  const base = `https://github.com/${repo}/releases/download/${release.tag_name}`;
  const assets = release.assets.map((a) => ({ name: a.name, url: `${base}/${encodeURIComponent(a.name)}` }));
  const signatures: Record<string, string> = {};
  for (const sig of release.assets.filter((a) => a.name.endsWith(".sig"))) {
    signatures[sig.name.replace(/\.sig$/, "")] = await gh(["api", "-H", "Accept: application/octet-stream", sig.url]);
  }
  const manifest = buildManifest(
    release.tag_name.replace(/^v/, ""),
    release.body ?? "",
    new Date().toISOString(),
    assets,
    signatures,
  );
  const missing = ["windows-x86_64", "darwin-aarch64", "darwin-x86_64", "linux-x86_64"].filter(
    (k) => !manifest.platforms[k],
  );
  if (missing.length) throw new Error(`missing platforms: ${missing.join(", ")}`);
  process.stdout.write(`${JSON.stringify(manifest, null, 2)}\n`);
}
