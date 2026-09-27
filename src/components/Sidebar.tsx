import type { DiskInfo, Suggestion, ViewNode } from "../lib/api";
import { formatAge, formatBytes, formatCount, splitBytes } from "../lib/format";
import { KINDS, kindColor } from "../lib/kinds";
import type { Mark, Marks } from "../lib/marks";

interface Props {
  selection: ViewNode;
  scanTotal: number;
  suggestions: Suggestion[];
  disk: DiskInfo | null;
  marked: Marks;
  onToggleMark: (m: Mark) => void;
  onReveal: (path: string) => void;
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-[11px] tracking-wider text-[var(--color-muted)] uppercase">{label}</div>
      <div className="mt-1 text-[15px]">{value}</div>
    </div>
  );
}

export function Sidebar({ selection, scanTotal, suggestions, disk, marked, onToggleMark, onReveal }: Props) {
  const [num, unit] = splitBytes(selection.size);
  const share = scanTotal ? (selection.size / scanTotal) * 100 : 0;
  const worth = suggestions.reduce((s, x) => s + x.size, 0);
  const maxSug = suggestions[0]?.size ?? 1;

  return (
    <aside className="flex w-[330px] shrink-0 flex-col gap-6 overflow-y-auto border-l border-[var(--color-line)] bg-[var(--color-panel)] p-5">
      <section aria-label="Seleção">
        <div className="text-[11px] tracking-wider text-[var(--color-muted)] uppercase">Seleção</div>
        <div className="mt-3 flex items-center gap-2">
          <span className="h-5 w-1 rounded" style={{ background: kindColor(selection.kind) }} />
          <span className="truncate text-lg font-medium" title={selection.path}>
            {selection.name}
          </span>
        </div>
        <div className="mt-3 text-5xl font-light tabular-nums">
          {num}
          <span className="ml-2 text-xl text-[var(--color-muted)]">{unit}</span>
        </div>
        <div className="mt-3 h-1.5 rounded bg-[var(--color-panel-2)]">
          <div className="h-full rounded bg-[var(--color-accent)]" style={{ width: `${Math.min(100, share)}%` }} />
        </div>
        <div className="mt-4 grid grid-cols-2 gap-4">
          <Stat label="Da análise" value={`${share.toFixed(share < 10 ? 1 : 0)}%`} />
          <Stat label="Arquivos" value={formatCount(selection.files)} />
          <Stat label="Última escrita" value={formatAge(selection.modified)} />
          <Stat label="Tipo" value={KINDS[selection.kind].label} />
        </div>
        {!selection.grouped && (
          <div className="mt-4 flex gap-2">
            <button
              type="button"
              onClick={() => onToggleMark(selection)}
              className="flex-1 rounded-md border border-[var(--color-line)] px-3 py-1.5 text-sm hover:bg-[var(--color-panel-2)]"
            >
              {marked.has(selection.path) ? "Desmarcar" : "Marcar para excluir"}
            </button>
            <button
              type="button"
              onClick={() => onReveal(selection.path)}
              className="rounded-md border border-[var(--color-line)] px-3 py-1.5 text-sm hover:bg-[var(--color-panel-2)]"
              title="Copiar caminho"
            >
              ⧉
            </button>
          </div>
        )}
      </section>

      <section aria-label="Vale uma olhada" className="border-t border-[var(--color-line)] pt-5">
        <div className="flex items-baseline justify-between">
          <span className="text-[11px] tracking-wider text-[var(--color-muted)] uppercase">Vale uma olhada</span>
          <span className="text-sm text-[var(--color-accent)] tabular-nums">{formatBytes(worth)}</span>
        </div>
        {suggestions.length === 0 && (
          <p className="mt-3 text-sm text-[var(--color-muted)]">Nada óbvio para limpar aqui. 🎉</p>
        )}
        <ul className="mt-3 flex flex-col gap-1">
          {suggestions.map((s) => (
            <li key={s.path}>
              <button
                type="button"
                data-path={s.path}
                data-color={kindColor(s.kind)}
                onClick={() => onToggleMark(s)}
                aria-pressed={marked.has(s.path)}
                className={`w-full rounded-md border-l-2 px-3 py-2 text-left hover:bg-[var(--color-panel-2)] ${marked.has(s.path) ? "bg-[var(--color-panel-2)]" : ""}`}
                style={{ borderColor: marked.has(s.path) ? "var(--color-danger)" : kindColor(s.kind) }}
              >
                <div className="flex justify-between gap-2 text-sm">
                  <span className="truncate" title={s.path}>
                    {s.name}
                  </span>
                  <span className="shrink-0 tabular-nums">{formatBytes(s.size)}</span>
                </div>
                <div className="mt-1 flex items-center justify-between gap-3">
                  <span className="truncate text-xs text-[var(--color-muted)]">{s.reason}</span>
                  <span className="h-1 w-24 shrink-0 rounded bg-[var(--color-panel-2)]">
                    <span
                      className="block h-full rounded"
                      style={{ width: `${(s.size / maxSug) * 100}%`, background: kindColor(s.kind) }}
                    />
                  </span>
                </div>
              </button>
            </li>
          ))}
        </ul>
      </section>

      {disk && (
        <section aria-label="Disco" className="mt-auto border-t border-[var(--color-line)] pt-5">
          <div className="text-[11px] tracking-wider text-[var(--color-muted)] uppercase">Disco {disk.mount}</div>
          <div className="mt-2 text-3xl font-light tabular-nums">
            {formatBytes(disk.free)} <span className="text-base text-[var(--color-muted)]">livres</span>
          </div>
          <div className="mt-2 h-1.5 rounded bg-[var(--color-panel-2)]">
            <div
              className="h-full rounded bg-[#6b7a99]"
              style={{ width: `${((disk.total - disk.free) / disk.total) * 100}%` }}
            />
          </div>
          <div className="mt-2 flex justify-between text-xs text-[var(--color-muted)]">
            <span>{formatBytes(disk.total - disk.free)} usados</span>
            <span>{formatBytes(disk.total)} total</span>
          </div>
        </section>
      )}
    </aside>
  );
}
