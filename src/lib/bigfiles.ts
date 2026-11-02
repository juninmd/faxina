import type { BigFile, Kind } from "./api";
import { baseName } from "./format";

export type SortKey = "size" | "age" | "name";

export const SORT_OPTIONS: { value: SortKey; label: string }[] = [
  { value: "size", label: "Maior primeiro" },
  { value: "age", label: "Mais antigo primeiro" },
  { value: "name", label: "Nome (A–Z)" },
];

/** How many files each kind holds, so the filter chips can show their counts. */
export function kindCounts(files: BigFile[]): Map<Kind, number> {
  const counts = new Map<Kind, number>();
  for (const f of files) counts.set(f.kind, (counts.get(f.kind) ?? 0) + 1);
  return counts;
}

/** A sorted copy; `size` and `age` break ties by path so the order is stable between renders. */
export function sortFiles(files: BigFile[], key: SortKey): BigFile[] {
  const byPath = (a: BigFile, b: BigFile) => a.path.localeCompare(b.path);
  const order: Record<SortKey, (a: BigFile, b: BigFile) => number> = {
    size: (a, b) => b.size - a.size || byPath(a, b),
    age: (a, b) => a.modified - b.modified || byPath(a, b),
    name: (a, b) => baseName(a.path).localeCompare(baseName(b.path)) || byPath(a, b),
  };
  return [...files].sort(order[key]);
}
