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

export function defaultPick(groups: DupGroup[]): Set<string> {
  return new Set(groups.flatMap((g) => g.files.slice(1).map((f) => f.path)));
}

/** Drops only what was really deleted; failures stay listed because they are still on disk. */
export function pruneDeleted(groups: DupGroup[], deleted: Iterable<string>): DupGroup[] {
  const gone = new Set(deleted);
  return groups
    .map((g) => ({ ...g, files: g.files.filter((f) => !gone.has(f.path)) }))
    .filter((g) => g.files.length > 1);
}
