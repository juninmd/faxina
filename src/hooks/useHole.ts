import { useCallback, useRef, useState } from "react";
import type { HoleSource } from "../components/BlackHole";

interface HoleState {
  sources: HoleSource[];
  freed: number | null;
  failed: number;
  reason?: string;
}

/** Grabs the on-screen elements tagged with these keys and fades them as the hole takes over. */
export function collectSources(keys: Iterable<string>, fades: Animation[] = []): HoleSource[] {
  const wanted = new Set(keys);
  const out: HoleSource[] = [];
  for (const el of document.querySelectorAll<HTMLElement>("[data-path]")) {
    const key = el.dataset.path;
    if (!key || !wanted.has(key)) continue;
    const rect = el.getBoundingClientRect();
    if (rect.width < 1 || rect.height < 1) continue;
    out.push({ rect, color: el.dataset.color ?? "#e3b341" });
    const fade = el.animate(
      [
        { opacity: 1, filter: "none" },
        { opacity: 0, filter: "blur(6px)" },
      ],
      {
        duration: 700,
        easing: "ease-in",
        fill: "forwards",
      },
    );
    fades.push(fade);
  }
  return out;
}

export function useHole() {
  const [hole, setHole] = useState<HoleState | null>(null);
  const after = useRef<() => void | Promise<void>>(() => {});
  // React reuses DOM nodes by key: a row that survives (failed delete, re-listed cache)
  // would stay invisible forever if the forwards-filled fade were never cancelled.
  const fades = useRef<Animation[]>([]);

  const swallow = useCallback(
    async (
      keys: Iterable<string>,
      action: () => Promise<{ freed: number; failed: number; reason?: string }>,
      done: () => void | Promise<void>,
    ) => {
      after.current = done;
      fades.current = [];
      setHole({ sources: collectSources(keys, fades.current), freed: null, failed: 0 });
      try {
        const { freed, failed, reason } = await action();
        setHole((h) => h && { ...h, freed, failed, reason });
      } catch (e) {
        console.error(e);
        setHole((h) => h && { ...h, freed: 0, failed: 1, reason: String(e) });
      }
    },
    [],
  );

  const finish = useCallback(async () => {
    setHole(null);
    const pending = fades.current;
    fades.current = [];
    try {
      await after.current();
    } finally {
      // Wait for React to drop the deleted rows before un-hiding whatever is left.
      requestAnimationFrame(() => {
        for (const f of pending) f.cancel();
      });
    }
  }, []);

  return { hole, swallow, finish };
}
