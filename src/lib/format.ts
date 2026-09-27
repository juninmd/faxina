const UNITS = ["B", "KB", "MB", "GB", "TB"];

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), UNITS.length - 1);
  const v = bytes / 1024 ** i;
  const digits = v >= 100 || i === 0 ? 0 : v >= 10 ? 1 : 2;
  return `${v.toLocaleString("pt-BR", { maximumFractionDigits: digits })} ${UNITS[i]}`;
}

/** Splits "12,4 GB" into number and unit so the UI can style them apart. */
export function splitBytes(bytes: number): [string, string] {
  const [n, u] = formatBytes(bytes).split(" ");
  return [n, u];
}

export function formatCount(n: number): string {
  if (n >= 1e6) return `${(n / 1e6).toLocaleString("pt-BR", { maximumFractionDigits: 1 })} mi`;
  if (n >= 1e3) return `${(n / 1e3).toLocaleString("pt-BR", { maximumFractionDigits: 1 })} mil`;
  return n.toLocaleString("pt-BR");
}

export function formatAge(epochSecs: number, now = Date.now()): string {
  if (!epochSecs) return "—";
  const secs = Math.max(0, now / 1000 - epochSecs);
  const table: [number, string, string][] = [
    [31536000, "ano", "anos"],
    [2592000, "mês", "meses"],
    [86400, "dia", "dias"],
    [3600, "hora", "horas"],
    [60, "minuto", "minutos"],
  ];
  for (const [unit, one, many] of table) {
    const v = Math.floor(secs / unit);
    if (v >= 1) return `há ${v} ${v === 1 ? one : many}`;
  }
  return "agora";
}

export function baseName(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

export function parentOf(path: string, root: string): string | null {
  if (path === root) return null;
  const idx = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  if (idx <= 0) return root;
  const parent = path.slice(0, idx);
  return parent.length < root.length ? root : parent;
}
