import { useEffect, useRef } from "react";
import { shouldAnimate } from "../lib/motion";

export interface Ring {
  label: string;
  /** 0..1 share of the full turn. */
  share: number;
  color: string;
}

/**
 * The home screen's planet: the outer ring is the disk in use, inner rings are the cache groups
 * that can go, each scaled against the largest one so small groups stay visible.
 */
export default function Planet({ rings, label }: { rings: Ring[]; label: string }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const key = rings.map((r) => `${r.label}:${r.share.toFixed(3)}`).join("|");

  // biome-ignore lint/correctness/useExhaustiveDependencies: redraw only when the ring values change
  useEffect(() => {
    const el = canvas.current;
    const ctx = el?.getContext("2d");
    if (!el || !ctx) return;
    const animate = shouldAnimate();
    const dust = Array.from({ length: 160 }, () => ({
      a: Math.random() * Math.PI * 2,
      r: 0.43 + Math.random() * 0.06,
      s: 0.0004 + Math.random() * 0.0012,
      z: Math.random(),
    }));
    const t0 = performance.now();
    let raf = 0;
    const frame = (t: number) => {
      const dpr = window.devicePixelRatio || 1;
      const S = el.clientWidth;
      if (el.width !== Math.round(S * dpr)) el.width = el.height = Math.round(S * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, S, S);
      const c = S / 2;
      const k = animate ? Math.min(1, (t - t0) / 1200) : 1;
      const e = 1 - (1 - k) ** 3;
      for (const p of dust) {
        if (animate) p.a += p.s;
        ctx.fillStyle = `rgba(183,156,255,${0.12 + p.z * 0.35})`;
        ctx.fillRect(c + Math.cos(p.a) * S * p.r, c + Math.sin(p.a) * S * p.r * 0.92, 1.4, 1.4);
      }
      ctx.lineCap = "round";
      rings.forEach((r, i) => {
        const R = S * (0.37 - i * 0.045);
        if (R < S * 0.12) return;
        const width = i === 0 ? S * 0.034 : S * 0.022;
        const start = -Math.PI / 2 + i * 0.32;
        ctx.lineWidth = width;
        ctx.strokeStyle = "rgba(58,58,86,0.55)";
        ctx.beginPath();
        ctx.arc(c, c, R, 0, Math.PI * 2);
        ctx.stroke();
        const sweep = Math.max(0.02, Math.min(0.985, r.share)) * Math.PI * 2 * e;
        ctx.strokeStyle = r.color;
        ctx.shadowColor = r.color;
        ctx.shadowBlur = i === 0 ? 18 : 8;
        ctx.beginPath();
        ctx.arc(c, c, R, start, start + sweep);
        ctx.stroke();
        ctx.shadowBlur = 0;
      });
      if (animate && !document.hidden) raf = requestAnimationFrame(frame);
    };
    const wake = () => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(frame);
    };
    wake();
    document.addEventListener("visibilitychange", wake);
    window.addEventListener("resize", wake);
    return () => {
      cancelAnimationFrame(raf);
      document.removeEventListener("visibilitychange", wake);
      window.removeEventListener("resize", wake);
    };
  }, [key]);

  return <canvas ref={canvas} className="aspect-square w-full" role="img" aria-label={label} />;
}
