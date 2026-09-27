import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { api, type DiskInfo, type Suggestion, type ViewNode } from "../lib/api";
import { parentOf } from "../lib/format";

export interface ScanTick {
  files: number;
  bytes: number;
}

export function useScan() {
  const [root, setRoot] = useState("");
  const [total, setTotal] = useState(0);
  const [view, setView] = useState<ViewNode | null>(null);
  const [depth, setDepth] = useState(4);
  const [progress, setProgress] = useState<ScanTick | null>(null);
  const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
  const [disk, setDisk] = useState<DiskInfo | null>(null);
  const [error, setError] = useState("");
  const [elapsed, setElapsed] = useState(0);

  useEffect(() => {
    const un = listen<ScanTick>("scan-progress", (e) => setProgress((p) => (p ? e.payload : p)));
    return () => {
      un.then((f) => f());
    };
  }, []);

  const scan = useCallback(
    async (path: string) => {
      setError("");
      setProgress({ files: 0, bytes: 0 });
      const t0 = performance.now();
      try {
        const v = await api.startScan(path, depth);
        setElapsed((performance.now() - t0) / 1000);
        setRoot(path);
        setTotal(v.size);
        setView(v);
        setSuggestions(await api.suggestions());
        setDisk(await api.diskInfo(path));
      } catch (e) {
        setError(String(e));
      } finally {
        setProgress(null);
      }
    },
    [depth],
  );

  const open = useCallback(async (path: string) => setView(await api.getView(path, depth)), [depth]);

  const up = useCallback(() => {
    if (!view) return;
    const parent = parentOf(view.path, root);
    if (parent) open(parent);
  }, [view, root, open]);

  /** Re-reads the tree after a deletion; the focused folder may be gone, so fall back to root. */
  const refresh = useCallback(async () => {
    if (!root) return;
    const current = view?.path ?? root;
    const next = await api.getView(current, depth).catch(() => api.getView(root, depth));
    setView(next);
    setTotal((await api.getView(root, 0)).size);
    setSuggestions(await api.suggestions());
    setDisk(await api.diskInfo(root));
  }, [root, view, depth]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: re-layout only when the depth changes
  useEffect(() => {
    if (view) api.getView(view.path, depth).then(setView, (e) => setError(String(e)));
  }, [depth]);

  /** Back to the start screen to pick another disk or folder; the last root stays for Duplicatas. */
  const reset = useCallback(() => {
    setView(null);
    setError("");
  }, []);

  return {
    root,
    total,
    view,
    depth,
    setDepth,
    progress,
    suggestions,
    disk,
    error,
    elapsed,
    scan,
    open,
    up,
    refresh,
    reset,
  };
}
