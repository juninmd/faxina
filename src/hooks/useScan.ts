import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
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
  const [scanning, setScanning] = useState("");
  // Each scan or folder open takes a ticket; only the latest one may write the view.
  const latest = useRef(0);

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
      setScanning(path);
      const t0 = performance.now();
      const mine = ++latest.current;
      try {
        const v = await api.startScan(path, depth);
        setElapsed((performance.now() - t0) / 1000);
        if (mine !== latest.current) return;
        setRoot(path);
        setTotal(v.size);
        setView(v);
        // Independent reads: awaiting them one after the other kept the result screen waiting.
        const [sug, info] = await Promise.all([api.suggestions(), api.diskInfo(path)]);
        if (mine !== latest.current) return;
        setSuggestions(sug);
        setDisk(info);
      } catch (e) {
        setError(String(e));
      } finally {
        setProgress(null);
      }
    },
    [depth],
  );

  const open = useCallback(
    async (path: string) => {
      const mine = ++latest.current;
      try {
        const v = await api.getView(path, depth);
        if (mine === latest.current) setView(v);
      } catch (e) {
        setError(String(e));
      }
    },
    [depth],
  );

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
    const [rootView, sug, info] = await Promise.all([api.getView(root, 0), api.suggestions(), api.diskInfo(root)]);
    setTotal(rootView.size);
    setSuggestions(sug);
    setDisk(info);
  }, [root, view, depth]);

  // A depth change gets its own ticket: it must not drop an in-flight scan, nor be dropped by one.
  const depthTicket = useRef(0);
  // biome-ignore lint/correctness/useExhaustiveDependencies: re-layout only when the depth changes
  useEffect(() => {
    if (!view) return;
    const ticket = ++depthTicket.current;
    api.getView(view.path, depth).then(
      (v) => ticket === depthTicket.current && setView(v),
      (e) => ticket === depthTicket.current && setError(String(e)),
    );
  }, [depth]);

  /** Back to the start screen to pick another disk or folder; the last root stays for Duplicatas. */
  const reset = useCallback(() => {
    setView(null);
    setError("");
  }, []);

  return {
    root,
    scanning,
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
