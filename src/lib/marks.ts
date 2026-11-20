import type { Kind } from "./api";

export interface Mark {
  path: string;
  name: string;
  size: number;
  kind: Kind;
}

export type Marks = ReadonlyMap<string, Mark>;

function isChildOf(child: string, parent: string): boolean {
  return child.startsWith(`${parent}\\`) || child.startsWith(`${parent}/`);
}

/**
 * Toggles a mark without double-counting nested paths: marking a parent drops its marked
 * descendants, and marking a child of an already marked parent is a no-op.
 */
export function toggleMarkMap(prev: Marks, m: Mark): Map<string, Mark> {
  const next = new Map(prev);
  if (next.has(m.path)) {
    next.delete(m.path);
    return next;
  }
  for (const p of next.keys()) {
    if (isChildOf(m.path, p)) return next;
  }
  for (const p of next.keys()) {
    if (isChildOf(p, m.path)) next.delete(p);
  }
  next.set(m.path, { path: m.path, name: m.name, size: m.size, kind: m.kind });
  return next;
}
