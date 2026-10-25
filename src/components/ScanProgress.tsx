import { useEffect, useState } from "react";
import type { ScanTick } from "../hooks/useScan";
import { formatBytes, formatCount } from "../lib/format";
import CountUp from "./CountUp";

export function formatElapsed(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  return `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
}

/** Live numbers of a running scan: what it has seen, how fast, and for how long. */
export default function ScanProgress({
  progress,
  path,
  onCancel,
}: {
  progress: ScanTick;
  path: string;
  onCancel: () => void;
}) {
  const [secs, setSecs] = useState(0);
  // The backend stops at its next check, so the numbers keep moving for a moment after the click.
  const [asked, setAsked] = useState(false);
  useEffect(() => {
    const t0 = performance.now();
    const id = setInterval(() => setSecs((performance.now() - t0) / 1000), 250);
    return () => clearInterval(id);
  }, []);
  const rate = secs >= 1 ? progress.files / secs : 0;

  return (
    <div className="panel map-scan-state grain flex flex-col items-center justify-center gap-7 p-12 text-center">
      <div className="relative h-36 w-36" aria-hidden="true">
        <div className="absolute inset-0 animate-spin rounded-full border-4 border-transparent border-t-[var(--color-accent)] border-r-[var(--color-accent)]/40" />
        <div className="absolute inset-4 animate-[spin_2.4s_linear_infinite_reverse] rounded-full border-4 border-transparent border-b-[#8b5cd6]" />
        <div className="absolute inset-0 flex flex-col items-center justify-center">
          <span className="text-2xl font-semibold tabular-nums">
            <CountUp value={progress.bytes} format={formatBytes} />
          </span>
          <span className="text-xs text-[var(--color-muted)]">encontrados</span>
        </div>
      </div>
      <div className="flex max-w-xl flex-col items-center gap-1">
        <p className="text-sm text-[var(--color-muted)]" role="status">
          Mapeando disco…
        </p>
        <p className="max-w-full truncate font-medium text-white" title={path}>
          {path}
        </p>
      </div>
      <dl className="stagger grid w-full max-w-md grid-cols-3 gap-3">
        {[
          ["Arquivos", formatCount(progress.files)],
          ["Ritmo", rate ? `${formatCount(Math.round(rate))}/s` : "—"],
          ["Tempo", formatElapsed(secs)],
        ].map(([label, value]) => (
          <div key={label} className="rounded-lg border border-[var(--color-line)] bg-[var(--color-panel-2)] px-3 py-2">
            <dt className="text-[10px] tracking-wider text-[var(--color-muted)] uppercase">{label}</dt>
            <dd className="mt-1 text-lg font-medium tabular-nums">{value}</dd>
          </div>
        ))}
      </dl>
      <button
        type="button"
        disabled={asked}
        onClick={() => {
          setAsked(true);
          onCancel();
        }}
        className="button secondary"
      >
        {asked ? "Cancelando…" : "Cancelar mapeamento"}
      </button>
    </div>
  );
}
