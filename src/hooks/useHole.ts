import { useCallback, useRef, useState } from "react";
import type { HoleSource } from "../components/BlackHole";

interface HoleState {
  sources: HoleSource[];
  freed: number | null;
  failed: number;
}

/** Grabs the on-screen elements tagged with these keys and fades them as the hole takes over. */
export function collectSources(keys: Iterable<string>): HoleSource[] {
  const wanted = new Set(keys);
  const out: HoleSource[] = [];
  for (const el of document.querySelectorAll<HTMLElement>("[data-path]")) {
    const key = el.dataset.path;
    if (!key || !wanted.has(key)) continue;
    const rect = el.getBoundingClientRect();
    if (rect.width < 1 || rect.height < 1) continue;
    out.push({ rect, color: el.dataset.color ?? "#e3b341" });
    el.animate(
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
  }
  return out;
}

export function useHole() {
  const [hole, setHole] = useState<HoleState | null>(null);
  const after = useRef<() => void>(() => {});

  const swallow = useCallback(
    async (keys: Iterable<string>, action: () => Promise<{ freed: number; failed: number }>, done: () => void) => {
      after.current = done;
      setHole({ sources: collectSources(keys), freed: null, failed: 0 });
      try {
        const { freed, failed } = await action();
        setHole((h) => h && { ...h, freed, failed });
      } catch (e) {
        console.error(e);
        setHole((h) => h && { ...h, freed: 0, failed: 1 });
      }
    },
    [],
  );

  const finish = useCallback(() => {
    setHole(null);
    after.current();
  }, []);

  return { hole, swallow, finish };
}
