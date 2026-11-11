import { useEffect, useRef } from "react";
import { shouldAnimate } from "../lib/motion";

interface Star {
  a: number;
  r: number;
}

/**
 * The Órbita's resting black hole. Its core grows with what waits in the tray, it flinches when
 * something new arrives and it pulls the disk inward while a deletion runs.
 */
export default function HoleCanvas({ mass, pulse, eating }: { mass: number; pulse: number; eating: boolean }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const state = useRef({ mass, kick: 0, eat: 0, eating });
  // Without animation the canvas draws one frame; a change must ask for the next one.
  const redraw = useRef(() => {});

  useEffect(() => {
    state.current.mass = mass;
    state.current.eating = eating;
    redraw.current();
  }, [mass, eating]);

  useEffect(() => {
    if (pulse > 0) state.current.kick = 1;
    redraw.current();
  }, [pulse]);

  useEffect(() => {
    const el = canvas.current;
    const ctx = el?.getContext("2d");
    if (!el || !ctx) return;
    const stars: Star[] = Array.from({ length: 200 }, () => ({
      a: Math.random() * Math.PI * 2,
      r: 18 + Math.random() * 120,
    }));
    const animate = shouldAnimate();
    let raf = 0;

    const frame = () => {
      const s = state.current;
      const dpr = window.devicePixelRatio || 1;
      const W = el.clientWidth;
      const H = el.clientHeight;
      if (el.width !== Math.round(W * dpr)) {
        el.width = Math.round(W * dpr);
        el.height = Math.round(H * dpr);
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, W, H);
      s.kick *= 0.92;
      s.eat += ((s.eating ? 1 : 0) - s.eat) * 0.06;
      const cx = W / 2;
      const cy = H / 2 + 6;
      const core = 16 + s.mass * 12 + s.kick * 6 + s.eat * 12;

      for (const p of stars) {
        if (animate) {
          p.a += (0.003 + 50 / (p.r * p.r)) * (1 + s.eat * 5);
          if (s.eat > 0.05) {
            p.r -= s.eat * 2.2;
            if (p.r < core) p.r = 60 + Math.random() * 80;
          }
        }
        const x = cx + Math.cos(p.a) * p.r * 1.55;
        const y = cy + Math.sin(p.a) * p.r * 0.42;
        const near = Math.max(0, 1 - (p.r - core) / 100);
        ctx.fillStyle = `rgba(255,${Math.round(150 + 80 * (1 - near))},${Math.round(70 + 120 * (1 - near))},${0.2 + near * 0.7})`;
        ctx.fillRect(x, y, 1.5, 1.5);
      }
      const glow = ctx.createRadialGradient(cx, cy, core * 0.8, cx, cy, core * 2.7);
      glow.addColorStop(0, "rgba(255,106,61,0.9)");
      glow.addColorStop(0.35, "rgba(255,174,74,0.32)");
      glow.addColorStop(1, "rgba(183,156,255,0)");
      ctx.fillStyle = glow;
      ctx.beginPath();
      ctx.arc(cx, cy, core * 2.7, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = "#000";
      ctx.beginPath();
      ctx.arc(cx, cy, core, 0, Math.PI * 2);
      ctx.fill();
      ctx.strokeStyle = "rgba(255,220,170,0.8)";
      ctx.lineWidth = 1.2;
      ctx.beginPath();
      ctx.arc(cx, cy, core + 1, 0, Math.PI * 2);
      ctx.stroke();
      if (animate && !document.hidden) raf = requestAnimationFrame(frame);
    };
    const wake = () => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(frame);
    };
    redraw.current = wake;
    wake();
    document.addEventListener("visibilitychange", wake);
    window.addEventListener("resize", wake);
    return () => {
      cancelAnimationFrame(raf);
      document.removeEventListener("visibilitychange", wake);
      window.removeEventListener("resize", wake);
    };
  }, []);

  return <canvas ref={canvas} className="pointer-events-none absolute inset-0 h-full w-full" />;
}
