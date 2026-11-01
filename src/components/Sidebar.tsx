import { useEffect, useState } from "react";
import { api, type DiskInfo, type Suggestion, type ViewNode } from "../lib/api";
import { assistantPrompt, explainFolder } from "../lib/folders";
import { formatAge, formatBytes, formatCount, splitBytes } from "../lib/format";
import { KINDS, kindColor } from "../lib/kinds";
import type { Mark, Marks } from "../lib/marks";
import { platformName } from "../lib/platform";

interface Props {
  selection: ViewNode;
  scanTotal: number;
  suggestions: Suggestion[];
  disk: DiskInfo | null;
  marked: Marks;
  onToggleMark: (m: Mark) => void;
  onReveal: (path: string) => Promise<void>;
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl border border-slate-800/70 bg-slate-900/50 p-2.5 shadow-xs">
      <div className="text-[10px] font-bold tracking-wider text-slate-400 uppercase">{label}</div>
      <div className="mt-0.5 text-xs font-semibold text-slate-200">{value}</div>
    </div>
  );
}

const PAGE = 20;

export function Sidebar({ selection, scanTotal, suggestions, disk, marked, onToggleMark, onReveal }: Props) {
  const [copied, setCopied] = useState("");
  const [promptCopied, setPromptCopied] = useState("");
  const [promptFailed, setPromptFailed] = useState("");
  const [pathFailed, setPathFailed] = useState("");
  const [months, setMonths] = useState(6);
  const [stale, setStale] = useState<Suggestion[]>([]);
  const [staleLoaded, setStaleLoaded] = useState(false);
  const [staleFailed, setStaleFailed] = useState(false);
  const [sugShown, setSugShown] = useState(PAGE);
  const [staleShown, setStaleShown] = useState(PAGE);
  // `suggestions` is re-read after every scan and deletion, so it doubles as the refresh signal.
  // biome-ignore lint/correctness/useExhaustiveDependencies: refetch when the scan data changes
  useEffect(() => {
    // A reply for a months value the user has already changed away from must not win.
    let current = true;
    setStaleLoaded(false);
    setStaleFailed(false);
    api.staleDeps(months).then(
      (s) => {
        if (!current) return;
        setStale(s);
        setStaleLoaded(true);
      },
      () => {
        if (!current) return;
        setStale([]);
        setStaleFailed(true);
        setStaleLoaded(true);
      },
    );
    return () => {
      current = false;
    };
  }, [months, suggestions]);
  const [num, unit] = splitBytes(selection.size);
  const share = scanTotal ? (selection.size / scanTotal) * 100 : 0;
  const worth = suggestions.reduce((s, x) => s + x.size, 0);
  const info = explainFolder(selection.path);
  const maxSug = suggestions[0]?.size ?? 1;

  return (
    <aside className="flex w-[330px] shrink-0 flex-col gap-6 overflow-y-auto border-l border-slate-800/80 bg-slate-950/90 p-5 backdrop-blur-xl">
      <section aria-label="Seleção">
        <div className="text-[10px] font-bold tracking-widest text-amber-400 uppercase">Seleção</div>
        <div className="mt-2.5 flex items-center gap-2">
          <span className="h-5 w-1.5 rounded-full" style={{ background: kindColor(selection.kind) }} />
          <span className="truncate text-base font-bold text-white" title={selection.path}>
            {selection.name}
          </span>
        </div>
        <div className="mt-2 font-mono text-4xl font-extrabold tracking-tight text-white">
          {num}
          <span className="ml-2 text-lg text-slate-400">{unit}</span>
        </div>
        <div className="mt-2.5 h-2 w-full overflow-hidden rounded-full bg-slate-800">
          <div
            className="h-full rounded-full bg-gradient-to-r from-amber-500 to-amber-400"
            style={{ width: `${Math.min(100, share)}%` }}
          />
        </div>
        <div className="mt-4 grid grid-cols-2 gap-2.5">
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
              className="flex-1 cursor-pointer rounded-xl border border-slate-700 bg-slate-800/90 px-3 py-2 text-xs font-semibold text-slate-200 transition-colors hover:bg-slate-750"
            >
              {marked.has(selection.path) ? "Desmarcar" : "Marcar para excluir"}
            </button>
            <button
              type="button"
              onClick={async () => {
                try {
                  await onReveal(selection.path);
                } catch {
                  setPathFailed(selection.path);
                  setTimeout(() => setPathFailed(""), 2000);
                  return;
                }
                setCopied(selection.path);
                setTimeout(() => setCopied(""), 1500);
              }}
              className="cursor-pointer rounded-xl border border-slate-700 bg-slate-800/90 px-3 py-2 text-xs font-semibold text-slate-200 transition-colors hover:bg-slate-750"
              title="Copiar caminho"
              aria-label="Copiar caminho"
            >
              <span aria-hidden="true">
                {copied === selection.path ? "✓" : pathFailed === selection.path ? "✕" : "⧉"}
              </span>
            </button>
            <span className="sr-only" role="status">
              {pathFailed === selection.path ? "Não foi possível copiar o caminho" : copied && "Caminho copiado"}
            </span>
          </div>
        )}
        {selection.isDir && !selection.grouped && selection.children.length > 0 && (
          <button
            type="button"
            onClick={async () => {
              try {
                await navigator.clipboard.writeText(assistantPrompt(selection, platformName()));
                setPromptCopied(selection.path);
                setTimeout(() => setPromptCopied(""), 2000);
              } catch {
                setPromptFailed(selection.path);
                setTimeout(() => setPromptFailed(""), 2000);
              }
            }}
            className="mt-2.5 w-full cursor-pointer rounded-xl border border-slate-700 bg-slate-850 px-3 py-2 text-xs font-semibold text-slate-300 transition-colors hover:bg-slate-800"
            title="Copia o nome, o tamanho e os maiores itens da pasta para colar em um assistente de IA"
          >
            {promptCopied === selection.path
              ? "Texto copiado ✓"
              : promptFailed === selection.path
                ? "Não foi possível copiar"
                : "Copiar para um assistente"}
          </button>
        )}
      </section>

      {info && (
        <section aria-label="O que é isto" className="border-t border-[var(--color-line)] pt-5">
          <div className="text-[11px] tracking-wider text-[var(--color-muted)] uppercase">O que é isto?</div>
          <p className="mt-3 text-sm">{info.whatItIs}</p>
          <p className="mt-2 text-sm text-[var(--color-muted)]">{info.ifRemoved}</p>
        </section>
      )}

      <section aria-label="Vale uma olhada" className="border-t border-[var(--color-line)] pt-5">
        <div className="flex items-baseline justify-between">
          <span className="text-[11px] tracking-wider text-[var(--color-muted)] uppercase">Vale uma olhada</span>
          <span className="text-sm text-[var(--color-accent)] tabular-nums">{formatBytes(worth)}</span>
        </div>
        {suggestions.length === 0 && (
          <p className="mt-3 text-sm text-[var(--color-muted)]">Nada óbvio para limpar aqui. 🎉</p>
        )}
        <ul className="mt-3 flex flex-col gap-1">
          {suggestions.slice(0, sugShown).map((s) => (
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
        {suggestions.length > sugShown && (
          <button
            type="button"
            onClick={() => setSugShown((n) => n + PAGE)}
            className="mt-2 w-full rounded-md border border-[var(--color-line)] px-3 py-1.5 text-sm hover:bg-[var(--color-panel-2)]"
          >
            Mostrar mais {Math.min(PAGE, suggestions.length - sugShown)} (faltam{" "}
            {formatCount(suggestions.length - sugShown)})
          </button>
        )}
      </section>

      <section aria-label="Projetos parados" className="border-t border-[var(--color-line)] pt-5">
        <div className="flex items-baseline justify-between gap-2">
          <span className="text-[11px] tracking-wider text-[var(--color-muted)] uppercase">Projetos parados</span>
          <select
            aria-label="Sem mexer há"
            value={months}
            onChange={(e) => setMonths(Number(e.target.value))}
            className="rounded bg-[var(--color-panel-2)] px-2 py-0.5 text-xs"
          >
            {[1, 3, 6, 12, 24].map((m) => (
              <option key={m} value={m}>
                há {m} {m === 1 ? "mês" : "meses"}+
              </option>
            ))}
          </select>
        </div>
        {!staleLoaded ? (
          <p className="mt-3 text-sm text-[var(--color-muted)]">Procurando projetos parados…</p>
        ) : staleFailed ? (
          <p className="mt-3 text-sm text-[var(--color-muted)]">Não foi possível verificar os projetos parados.</p>
        ) : stale.length === 0 ? (
          <p className="mt-3 text-sm text-[var(--color-muted)]">Nenhum projeto parado nesse período.</p>
        ) : (
          <>
            <div className="mt-2 text-sm text-[var(--color-accent)] tabular-nums">
              {formatBytes(stale.reduce((s, x) => s + x.size, 0))} em {stale.length}
            </div>
            <ul className="mt-2 flex flex-col gap-1">
              {stale.slice(0, staleShown).map((s) => (
                <li key={s.path}>
                  <button
                    type="button"
                    data-path={s.path}
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
                    <div className="mt-1 truncate text-xs text-[var(--color-muted)]">{s.reason}</div>
                  </button>
                </li>
              ))}
            </ul>
            {stale.length > staleShown && (
              <button
                type="button"
                onClick={() => setStaleShown((n) => n + PAGE)}
                className="mt-2 w-full rounded-md border border-[var(--color-line)] px-3 py-1.5 text-sm hover:bg-[var(--color-panel-2)]"
              >
                Mostrar mais {Math.min(PAGE, stale.length - staleShown)} (faltam{" "}
                {formatCount(stale.length - staleShown)})
              </button>
            )}
          </>
        )}
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
