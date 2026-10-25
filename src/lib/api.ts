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

export interface DeleteReport {
  freed: number;
  deleted: string[];
  failed: { path: string; error: string }[];
}

export interface JunkItem {
  id: string;
  label: string;
  group: string;
  size: number;
  files: number;
  paths: string[];
  /** Opt-in item: never ticked by default; this is the warning to show. */
  caution?: string | null;
}

export interface CleanReport {
  freed: number;
  removed: number;
  skipped: number;
  failed: number;
  reason?: string | null;
}

export interface DupGroup {
  hash: string;
  size: number;
  files: { path: string; modified: number }[];
}

export interface EmptyItem {
  path: string;
  kind: "folder" | "link";
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
  deletePaths: (paths: string[], permanent: boolean) => invoke<DeleteReport>("delete_paths", { paths, permanent }),
  junkScan: () => invoke<JunkItem[]>("junk_scan"),
  junkClean: (ids: string[], permanent: boolean) => invoke<CleanReport>("junk_clean", { ids, permanent }),
  findDuplicates: (path: string, minSize: number) => invoke<DupGroup[]>("find_duplicates", { path, minSize }),
  findEmpty: (path: string) => invoke<EmptyItem[]>("find_empty", { path }),
  deleteBrokenLinks: (paths: string[]) => invoke<DeleteReport>("delete_broken_links", { paths }),
  cancelDuplicates: () => invoke<void>("cancel_duplicates"),
  diskInfo: (path: string) => invoke<DiskInfo | null>("disk_info", { path }),
  listDisks: () => invoke<DiskInfo[]>("list_disks"),
  homeDir: () => invoke<string>("home_dir"),
};
