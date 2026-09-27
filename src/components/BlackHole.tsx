import { useEffect, useRef, useState } from "react";
import { formatBytes } from "../lib/format";
import { shouldAnimate } from "../lib/motion";

export interface HoleSource {
  rect: DOMRect;
  color: string;
}

interface Props {
  sources: HoleSource[];
  /** Bytes freed; null while the deletion is still running (the hole keeps spinning). */
  freed: number | null;
  failed: number;
  reason?: string;
  onDone: () => void;
}

interface Particle {
  r: number;
  a: number;
  vr: number;
  delay: number;
  size: number;
  color: string;
  alive: boolean;
}

type Phase = "pull" | "collapse" | "boom" | "done";

const MAX_PARTICLES = 4200;

function spawn(sources: HoleSource[], cx: number, cy: number, w: number, h: number): Particle[] {
  const list = sources.length ? sources : [{ rect: new DOMRect(0, 0, w, h), color: "#e3b341" }];
  const area = list.reduce((s, x) => s + x.rect.width * x.rect.height, 0) || 1;
  const out: Particle[] = [];
  for (const { rect, color } of list) {
    // Hundreds of duplicate rows must not add up to tens of thousands of particles.
    const floor = Math.min(24, Math.floor(MAX_PARTICLES / list.length));
    const n = Math.max(floor, Math.round((MAX_PARTICLES * rect.width * rect.height) / area));
    for (let i = 0; i < n; i++) {
      const fx = Math.random();
      const x = rect.left + fx * rect.width;
      const y = rect.top + Math.random() * rect.height;
      out.push({
        r: Math.hypot(x - cx, y - cy),
        a: Math.atan2(y - cy, x - cx),
        vr: 0,
        // Left-to-right sweep: the tile crumbles like dust before it falls in.
        delay: fx * 700 + Math.random() * 350,
        size: 1 + Math.random() * 2.2,
        color,
        alive: true,
      });
    }
  }
  return out;
}

export function BlackHole({ sources, freed, failed, reason, onDone }: Props) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const freedRef = useRef(freed);
  freedRef.current = freed;
  const [phase, setPhase] = useState<Phase>("pull");
  const [shown, setShown] = useState(0);

  useEffect(() => {
    const el = canvas.current;
    const ctx = el?.getContext("2d");
    if (!el || !ctx) return;
    const dpr = window.devicePixelRatio || 1;
    const w = window.innerWidth;
    const h = window.innerHeight;
    el.width = w * dpr;
    el.height = h * dpr;
    ctx.scale(dpr, dpr);
    const cx = w / 2;
    const cy = h / 2;
    const fast = !shouldAnimate();
    const particles = fast ? [] : spawn(sources, cx, cy, w, h);
    const start = performance.now();
    let current: Phase = "pull";
    let phaseAt = start;
    let hole = 0;
    let frame = 0;
    const sparks: { x: number; y: number; vx: number; vy: number; life: number }[] = [];

    const go = (p: Phase) => {
      current = p;
      phaseAt = performance.now();
      setPhase(p);
    };

    const drawHole = (t: number, radius: number) => {
      if (radius <= 0.5) return;
      const glow = ctx.createRadialGradient(cx, cy, radius * 0.6, cx, cy, radius * 3.2);
      glow.addColorStop(0, "rgba(255,190,90,0.55)");
      glow.addColorStop(0.35, "rgba(160,90,255,0.22)");
      glow.addColorStop(1, "rgba(0,0,0,0)");
      ctx.fillStyle = glow;
      ctx.beginPath();
      ctx.arc(cx, cy, radius * 3.2, 0, Math.PI * 2);
      ctx.fill();
      for (let i = 0; i < 4; i++) {
        ctx.save();
        ctx.translate(cx, cy);
        ctx.rotate(t / (260 - i * 40) + i * 1.3);
        ctx.scale(1, 0.34 + i * 0.05);
        ctx.strokeStyle = `hsla(${30 + i * 18}, 100%, ${70 - i * 8}%, ${0.55 - i * 0.1})`;
        ctx.lineWidth = 3 - i * 0.5;
        ctx.beginPath();
        ctx.arc(0, 0, radius * (1.35 + i * 0.28), 0, Math.PI * 1.4);
        ctx.stroke();
        ctx.restore();
      }
      ctx.fillStyle = "#000";
      ctx.shadowColor = "rgba(255,170,60,0.9)";
      ctx.shadowBlur = 24;
      ctx.beginPath();
      ctx.arc(cx, cy, radius, 0, Math.PI * 2);
      ctx.fill();
      ctx.shadowBlur = 0;
    };

    const tick = (now: number) => {
      const t = now - start;
      const since = now - phaseAt;
      ctx.globalCompositeOperation = "source-over";
      ctx.fillStyle = "rgba(8,10,14,0.28)";
      ctx.fillRect(0, 0, w, h);

      if (current === "pull") {
        hole = Math.min(46, hole + 1.4);
        ctx.globalCompositeOperation = "lighter";
        let alive = 0;
        for (const p of particles) {
          if (!p.alive) continue;
          alive++;
          if (t < p.delay) {
            ctx.fillStyle = p.color;
            ctx.fillRect(cx + p.r * Math.cos(p.a), cy + p.r * Math.sin(p.a), p.size, p.size);
            continue;
          }
          p.vr = Math.min(p.vr + 0.035 + 9 / (p.r + 120), 11);
          p.r -= p.vr;
          p.a += 1.6 / Math.sqrt(Math.max(p.r, 10));
          if (p.r < hole * 0.7) {
            p.alive = false;
            continue;
          }
          const heat = Math.max(0, 1 - p.r / 220);
          ctx.fillStyle = heat > 0.55 ? "#fff3d6" : p.color;
          ctx.fillRect(cx + p.r * Math.cos(p.a), cy + p.r * Math.sin(p.a), p.size, p.size);
        }
        ctx.globalCompositeOperation = "source-over";
        drawHole(t, hole * (1 + 0.04 * Math.sin(t / 90)));
        const minTime = fast ? 150 : 1400;
        if (alive === 0 && t > minTime && freedRef.current !== null) go("collapse");
      } else if (current === "collapse") {
        const k = Math.min(1, since / 320);
        drawHole(t, hole * (1 - k * k));
        if (k >= 1) {
          for (let i = 0; i < (fast ? 0 : 140); i++) {
            const ang = Math.random() * Math.PI * 2;
            const sp = 2 + Math.random() * 9;
            sparks.push({ x: cx, y: cy, vx: Math.cos(ang) * sp, vy: Math.sin(ang) * sp, life: 1 });
          }
          go("boom");
        }
      } else if (current === "boom") {
        const k = Math.min(1, since / 900);
        ctx.strokeStyle = `rgba(255,214,140,${0.8 * (1 - k)})`;
        ctx.lineWidth = 6 * (1 - k) + 1;
        ctx.beginPath();
        ctx.arc(cx, cy, k * Math.max(w, h) * 0.75, 0, Math.PI * 2);
        ctx.stroke();
        ctx.globalCompositeOperation = "lighter";
        for (const s of sparks) {
          s.x += s.vx;
          s.y += s.vy;
          s.vx *= 0.96;
          s.vy *= 0.96;
          s.life -= 0.012;
          ctx.fillStyle = `rgba(255,${180 + Math.random() * 60},120,${Math.max(0, s.life)})`;
          ctx.fillRect(s.x, s.y, 2, 2);
        }
        if (since > 2600) go("done");
      }
      if (current !== "done") frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [sources]);

  // Counter rolls up once the hole collapses.
  useEffect(() => {
    if (phase !== "boom" || freed === null) return;
    const t0 = performance.now();
    let raf = 0;
    const step = (now: number) => {
      const k = Math.min(1, (now - t0) / 1100);
      setShown(freed * (1 - (1 - k) ** 3));
      if (k < 1) raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  }, [phase, freed]);

  useEffect(() => {
    if (phase === "done") onDone();
  }, [phase, onDone]);

  return (
    <div className="fixed inset-0 z-50" role="status" aria-live="polite">
      <canvas ref={canvas} className="absolute inset-0 h-full w-full" />
      {phase === "pull" && (
        <p className="absolute inset-x-0 bottom-16 text-center text-sm tracking-widest text-white/60 uppercase">
          {freed === null ? "Engolindo arquivos…" : "Quase lá…"}
        </p>
      )}
      {(phase === "boom" || phase === "done") && (
        <div className="pop-in absolute inset-0 flex flex-col items-center justify-center">
          <span className="text-6xl font-bold tabular-nums text-[var(--color-accent)] drop-shadow-[0_0_24px_rgba(227,179,65,0.6)]">
            {formatBytes(shown)}
          </span>
          <span className="mt-2 text-lg text-white/80">liberados ✨</span>
          {failed > 0 && (
            <span className="mt-3 max-w-lg text-center text-sm text-[var(--color-danger)]">
              {failed} item(ns) não puderam ser removidos{reason && `: ${reason}`}
            </span>
          )}
        </div>
      )}
    </div>
  );
}
