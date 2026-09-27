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
}

export interface CleanReport {
  freed: number;
  removed: number;
  skipped: number;
}

export interface DupGroup {
  hash: string;
  size: number;
  files: { path: string; modified: number }[];
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
  deletePaths: (paths: string[], permanent: boolean) => invoke<DeleteReport>("delete_paths", { paths, permanent }),
  junkScan: () => invoke<JunkItem[]>("junk_scan"),
  junkClean: (ids: string[]) => invoke<CleanReport>("junk_clean", { ids }),
  findDuplicates: (path: string, minSize: number) => invoke<DupGroup[]>("find_duplicates", { path, minSize }),
  cancelDuplicates: () => invoke<void>("cancel_duplicates"),
  diskInfo: (path: string) => invoke<DiskInfo | null>("disk_info", { path }),
  homeDir: () => invoke<string>("home_dir"),
};
