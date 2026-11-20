import { invoke } from "@tauri-apps/api/core";

export type Kind = "cache" | "build" | "git" | "code" | "media" | "documents" | "archives" | "apps" | "other";

export interface ViewNode {
  name: string;
  path: string;
  size: number;
  files: number;
  modified: number;
  kind: Kind;
  isDir: boolean;
  grouped: boolean;
  reclaimable: boolean;
  children: ViewNode[];
}

export interface Suggestion {
  path: string;
  name: string;
  size: number;
  kind: Kind;
  reason: string;
}

export interface BigFile {
  path: string;
  name: string;
  size: number;
  modified: number;
  kind: Kind;
}

export interface SearchQuery {
  text: string;
  minSize: number;
  /** Seconds since the last change; 0 turns the age filter off. */
  olderThanSecs: number;
  kind: Kind | null;
  includeDirs: boolean;
  limit: number;
}

export interface SearchHit extends BigFile {
  isDir: boolean;
}

export type ExportFormat = "csv" | "json" | "html";

export interface DeleteReport {
  freed: number;
  deleted: string[];
  failed: { path: string; error: string }[];
}

export interface JunkLocation {
  path: string;
  size: number;
  files: number;
}

/** A location the user left out of one cache in the review: `[id, path]`. */
export type JunkSkip = [string, string];

export interface JunkItem {
  id: string;
  label: string;
  group: string;
  size: number;
  files: number;
  paths: string[];
  /** Size per path, so single locations can be left out. */
  locations?: JunkLocation[];
  /** Opt-in item: never ticked by default; this is the warning to show. */
  caution?: string | null;
  /** Programs that keep these files open right now; the cleanup is refused until they close. */
  running?: string[];
}

export interface CleanReport {
  freed: number;
  removed: number;
  skipped: number;
  failed: number;
  reason?: string | null;
}

export interface RecycleBinInfo {
  bytes: number;
  items: number;
}

export interface DupGroup {
  hash: string;
  size: number;
  files: { path: string; modified: number }[];
}

export interface DupResult {
  groups: DupGroup[];
  cancelled: boolean;
  truncated: boolean;
}

export interface EmptyItem {
  path: string;
  kind: "folder" | "link";
}

export interface Worktree {
  path: string;
  /** Linked worktree, or a main repository checkout with its own `.git`. */
  kind: "worktree" | "repository";
  branch: string;
  head: string;
  /** Last commit, seconds since the epoch. */
  modified: number;
  size: number;
  files: number;
  /** Ignored entries that leave with the checkout (build output, dependencies, `.env`). */
  ignored: number;
  /** Everything is on the remote: safe to remove. */
  synced: boolean;
  /** Why the worktree is blocked from removal; null when synced. */
  reason?: string | null;
}

export interface WorktreeReport {
  freed: number;
  removed: string[];
  failed: { path: string; error: string }[];
  /** Cleanup that did not stop the removal; the checkout is gone either way. */
  warnings: string[];
}

export interface FolderGrowth {
  path: string;
  before: number;
  /** False when the folder was too small to be saved last time: `before` is then an upper bound. */
  beforeKnown: boolean;
  after: number;
  growth: number;
}

export interface Growth {
  root: string;
  previousAt: number;
  /** Total now minus total then; negative when the folder shrank. */
  delta: number;
  grown: FolderGrowth[];
}

export interface DiskInfo {
  mount: string;
  total: number;
  free: number;
}

export const api = {
  startScan: (path: string, depth: number) => invoke<ViewNode>("start_scan", { path, depth }),
  cancelScan: () => invoke<void>("cancel_scan"),
  getView: (path: string, depth: number) => invoke<ViewNode>("get_view", { path, depth }),
  suggestions: () => invoke<Suggestion[]>("get_suggestions"),
  staleDeps: (months: number) => invoke<Suggestion[]>("get_stale_deps", { months }),
  growth: () => invoke<Growth | null>("get_growth"),
  largestFiles: (limit: number) => invoke<BigFile[]>("largest_files", { limit }),
  searchFiles: (query: SearchQuery) => invoke<SearchHit[]>("search_files", { query }),
  exportReport: (query: SearchQuery, format: ExportFormat, path: string) =>
    invoke<number>("export_report", { query, format, path }),
  deletePaths: (paths: string[], permanent: boolean) => invoke<DeleteReport>("delete_paths", { paths, permanent }),
  junkScan: () => invoke<{ scanId: number; items: JunkItem[] }>("junk_scan"),
  junkClean: (ids: string[], skip: JunkSkip[], permanent: boolean, scanId: number) =>
    invoke<CleanReport>("junk_clean", { ids, skip, permanent, scanId }),
  junkPreview: (ids: string[], skip: JunkSkip[], scanId: number) =>
    invoke<CleanReport>("junk_preview", { ids, skip, scanId }),
  findDuplicates: (path: string, minSize: number, excludeNames: string[] = []) =>
    invoke<DupResult>("find_duplicates", { path, minSize, excludeNames }),
  findEmpty: (path: string) => invoke<EmptyItem[]>("find_empty", { path }),
  deleteBrokenLinks: (paths: string[]) => invoke<DeleteReport>("delete_broken_links", { paths }),
  findWorktrees: (path: string) => invoke<Worktree[]>("find_worktrees", { path }),
  deleteWorktrees: (paths: string[]) => invoke<WorktreeReport>("delete_worktrees", { paths }),
  cancelDuplicates: () => invoke<void>("cancel_duplicates"),
  diskInfo: (path: string) => invoke<DiskInfo | null>("disk_info", { path }),
  listDisks: () => invoke<DiskInfo[]>("list_disks"),
  homeDir: () => invoke<string>("home_dir"),
  recycleBinInfo: () => invoke<RecycleBinInfo>("recycle_bin_info"),
  emptyRecycleBin: (expectedItems: number, expectedBytes: number) =>
    invoke<RecycleBinInfo>("empty_recycle_bin", { expectedItems, expectedBytes }),
};

export interface WorktreeTick {
  phase: "walk" | "check";
  done: number;
  total: number;
  found: number;
  current: string;
}
