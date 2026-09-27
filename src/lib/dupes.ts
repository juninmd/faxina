import type { DupGroup } from "./api";

/** Every copy except the oldest one: the usual "keep the original" pick. */
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
