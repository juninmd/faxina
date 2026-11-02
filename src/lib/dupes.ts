import type { DupGroup } from "./api";

/** Every copy except the oldest one: the usual "keep the original" pick. */
/** Notice string for duplicate search states (cancelled or truncated due to MAX_DUP_GROUPS). */
export function dupNotice(res: { cancelled: boolean; truncated: boolean }): string | null {
  if (res.cancelled) {
    return "Busca cancelada: resultado parcial";
  }
  if (res.truncated) {
    return "Mostrando os 500 grupos com mais espaço desperdiçado";
  }
  return null;
}

/** Which copy of a group survives when the rest is selected. */
export type KeeperRule = "oldest" | "newest" | "shortestPath" | "shortestName";

export const KEEPER_RULES: { value: KeeperRule; label: string }[] = [
  { value: "oldest", label: "Manter a mais antiga" },
  { value: "newest", label: "Manter a mais recente" },
  { value: "shortestPath", label: "Manter o caminho mais curto" },
  { value: "shortestName", label: "Manter o nome mais curto" },
];

const nameOf = (path: string) => path.split(/[\\/]/).pop() ?? path;

/** Path of the copy to keep. Ties fall back to list order, which is oldest first. */
export function keeperOf(group: DupGroup, rule: KeeperRule): string {
  const best = <T extends number>(score: (f: DupGroup["files"][number]) => T, sign = 1) =>
    group.files.reduce((a, f) => (sign * score(f) < sign * score(a) ? f : a)).path;
  switch (rule) {
    case "newest":
      return best((f) => f.modified, -1);
    case "shortestPath":
      return best((f) => f.path.length);
    case "shortestName":
      return best((f) => nameOf(f.path).length);
    default:
      return group.files[0].path;
  }
}

/** Every copy except the keeper of each group. */
export function defaultPick(groups: DupGroup[], rule: KeeperRule = "oldest"): Set<string> {
  return new Set(groups.flatMap((g) => groupPick(g, rule)));
}

/** The copies of one group that a "select copies" action ticks. */
export function groupPick(group: DupGroup, rule: KeeperRule = "oldest"): string[] {
  const keep = keeperOf(group, rule);
  return group.files.filter((f) => f.path !== keep).map((f) => f.path);
}

/** Drops only what was really deleted; failures stay listed because they are still on disk. */
export function pruneDeleted(groups: DupGroup[], deleted: Iterable<string>): DupGroup[] {
  const gone = new Set(deleted);
  return groups
    .map((g) => ({ ...g, files: g.files.filter((f) => !gone.has(f.path)) }))
    .filter((g) => g.files.length > 1);
}
