import { useCallback, useRef, useState } from "react";

/** Tracks an element's content box with a ResizeObserver. */
export function useSize<T extends HTMLElement>() {
  const [size, setSize] = useState({ width: 0, height: 0 });
  const observer = useRef<ResizeObserver | null>(null);

  const ref = useCallback((el: T | null) => {
    observer.current?.disconnect();
    if (!el) return;
    observer.current = new ResizeObserver(([entry]) => {
      const { width, height } = entry.contentRect;
      setSize((s) =>
        s.width === Math.round(width) && s.height === Math.round(height)
          ? s
          : { width: Math.round(width), height: Math.round(height) },
      );
    });
    observer.current.observe(el);
  }, []);

  return [ref, size] as const;
}
