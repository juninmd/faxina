/** How an item leaves the disk: each route keeps its own backend command and checks. */
export type OrbitRoute = "path" | "link" | "worktree";

/** Something a panel (Grandes, Duplicatas, Vazios, Repositórios) put in the Órbita. */
export interface OrbitItem {
  /** The path on disk; also the key, so the same file is never queued twice. */
  path: string;
  name: string;
  size: number;
  color: string;
  route: OrbitRoute;
  /** The panel it came from, shown as the group title in the tray. */
  source: string;
}

export interface OrbitPlan {
  /** Through `delete_paths` (guard + duplicate-group checks), honoring the chosen destination. */
  paths: string[];
  /** Through `delete_broken_links` (`guard::check_dangling_link`). */
  links: string[];
  /** Through `delete_worktrees`, which re-checks git and always uses the trash. */
  worktrees: string[];
}

/** Splits what waits in the Órbita by the command that is allowed to remove it. */
export function planOrbit(marks: Iterable<string>, items: Iterable<OrbitItem>): OrbitPlan {
  const paths = new Set(marks);
  const links: string[] = [];
  const worktrees: string[] = [];
  for (const it of items) {
    if (it.route === "link") links.push(it.path);
    else if (it.route === "worktree") worktrees.push(it.path);
    else paths.add(it.path);
  }
  return { paths: [...paths], links, worktrees };
}

/** Groups items by source in first-seen order, for the tray. */
export function groupBySource(items: Iterable<OrbitItem>): [string, OrbitItem[]][] {
  const groups = new Map<string, OrbitItem[]>();
  for (const it of items) {
    const list = groups.get(it.source);
    if (list) list.push(it);
    else groups.set(it.source, [it]);
  }
  return [...groups];
}
