import { useEffect, useRef, useState } from "react";
import type { Rule } from "../domain";
import { formatBytes } from "../lib/format";
import { kindColor } from "../lib/kinds";
import type { Mark } from "../lib/marks";
import CountUp from "./CountUp";
import HoleCanvas from "./HoleCanvas";
import Icon from "./Icon";

const GB = 1024 ** 3;
const JUNK_COLOR = "#ffae4a";

/**
 * The Órbita: one tray for everything waiting to leave the disk, from any space. Map marks and
 * cache categories sit side by side; "Engolir" opens the single confirmation for all of them.
 */
export default function Orbit({
  marks,
  junk,
  eating,
  onUnmark,
  onUnpick,
  onClear,
  onSwallow,
}: {
  marks: Mark[];
  junk: Rule[];
  eating: boolean;
  onUnmark: (path: string) => void;
  onUnpick: (id: string) => void;
  onClear: () => void;
  onSwallow: () => void;
}) {
  const [open, setOpen] = useState(false);
  const count = marks.length + junk.length;
  const total = marks.reduce((s, m) => s + m.size, 0) + junk.reduce((s, r) => s + r.size, 0);
  const [pulse, setPulse] = useState(0);
  const last = useRef(count);
  useEffect(() => {
    if (count > last.current) setPulse((p) => p + 1);
    last.current = count;
  }, [count]);

  return (
    <>
      <button
        type="button"
        className="orbit-fab"
        onClick={() => setOpen(true)}
        data-orbit-target={open ? undefined : ""}
        aria-label={`Abrir Órbita: ${count} itens, ${formatBytes(total)}`}
      >
        <span className="orbit-fab-hole" aria-hidden="true" />
        <span className="font-mono text-sm font-semibold">{formatBytes(total)}</span>
        {count > 0 && <span className="rail-badge static">{count}</span>}
      </button>
      {open && (
        <button type="button" className="orbit-scrim" aria-label="Fechar Órbita" onClick={() => setOpen(false)} />
      )}
      <aside aria-label="Órbita" className={`orbit-tray ${open ? "is-open" : ""}`}>
        <div className="relative h-48 shrink-0 overflow-hidden border-b border-slate-800" data-orbit-target="">
          <HoleCanvas mass={Math.min(1, total / (60 * GB))} pulse={pulse} eating={eating} />
          <div className="absolute top-3 left-4">
            <h2 className="text-lg font-semibold text-white">Órbita</h2>
            <p className="text-xs text-slate-400">
              {count ? `${count} ${count === 1 ? "item" : "itens"} esperando` : "nada marcado ainda"}
            </p>
          </div>
          <button
            type="button"
            className="orbit-close absolute top-3 right-3 rounded-lg p-1.5 text-slate-400 hover:bg-slate-800 hover:text-white"
            aria-label="Fechar Órbita"
            onClick={() => setOpen(false)}
          >
            <Icon name="close" size={16} />
          </button>
          <div className="absolute right-4 bottom-2 font-display text-2xl font-extrabold tracking-tight text-white tabular-nums">
            <CountUp value={total} format={formatBytes} />
          </div>
        </div>

        <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto p-3">
          {!count && (
            <div className="rounded-2xl border border-dashed border-slate-700 p-6 text-center text-sm leading-relaxed text-slate-400">
              Marque pastas no <strong className="text-slate-200">Mapa</strong> ou caches em{" "}
              <strong className="text-slate-200">Limpar</strong>. Tudo espera aqui até você decidir.
            </div>
          )}
          {junk.length > 0 && (
            <Group title="Caches" total={junk.reduce((s, r) => s + r.size, 0)}>
              {junk.map((r) => (
                <Row
                  key={r.id}
                  dataPath={`junk:${r.id}`}
                  color={JUNK_COLOR}
                  name={r.name}
                  sub={r.app}
                  size={r.size}
                  onRemove={() => onUnpick(r.id)}
                />
              ))}
            </Group>
          )}
          {marks.length > 0 && (
            <Group title="Do mapa" total={marks.reduce((s, m) => s + m.size, 0)}>
              {marks.map((m) => (
                <Row
                  key={m.path}
                  dataPath={m.path}
                  color={kindColor(m.kind)}
                  name={m.name}
                  sub={m.path}
                  size={m.size}
                  onRemove={() => onUnmark(m.path)}
                />
              ))}
            </Group>
          )}
        </div>

        <div className="grid gap-2 border-t border-slate-800 p-3">
          <p className="flex items-center gap-2 text-[11px] text-slate-400">
            <Icon name="shield" size={14} />
            Lixeira por padrão. Você confirma o destino antes.
          </p>
          <div className="grid grid-cols-[auto_1fr] gap-2">
            <button type="button" className="btn" disabled={!count} onClick={onClear}>
              Esvaziar
            </button>
            <button type="button" className="btn btn-hot" disabled={!count} onClick={onSwallow}>
              Engolir {count > 0 && formatBytes(total)}
            </button>
          </div>
        </div>
      </aside>
    </>
  );
}

function Group({ title, total, children }: { title: string; total: number; children: React.ReactNode }) {
  return (
    <section className="flex flex-col gap-1.5">
      <h3 className="flex justify-between px-1 text-[10px] font-semibold tracking-widest text-slate-400 uppercase">
        <span>{title}</span>
        <span className="font-mono">{formatBytes(total)}</span>
      </h3>
      {children}
    </section>
  );
}

function Row({
  dataPath,
  color,
  name,
  sub,
  size,
  onRemove,
}: {
  dataPath: string;
  color: string;
  name: string;
  sub: string;
  size: number;
  onRemove: () => void;
}) {
  return (
    <div className="orbit-row" data-path={dataPath} data-color={color} title={sub}>
      <i style={{ background: color }} />
      <span className="min-w-0">
        <span className="block truncate text-slate-100">{name}</span>
        <span className="block truncate font-mono text-[10px] text-slate-500">{sub}</span>
      </span>
      <span className="font-mono text-[11px] text-slate-300">{formatBytes(size)}</span>
      <button type="button" aria-label={`Tirar ${name} da Órbita`} onClick={onRemove}>
        <Icon name="close" size={13} />
      </button>
    </div>
  );
}
