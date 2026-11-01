import { useCallback, useEffect, useMemo, useState } from "react";
import { api, type BigFile, type DeleteReport } from "../lib/api";
import { baseName, formatAge, formatBytes, formatCount } from "../lib/format";
import { kindColor } from "../lib/kinds";
import type { Swallow } from "./types";

const MIN_SIZES = [100 * 1024 * 1024, 500 * 1024 * 1024, 1024 * 1024 * 1024];
const PAGE = 100;

/**
 * Files the scan tracked individually, biggest first. The scan folds tiny files into per-folder
 * buckets, so this is exactly the "what is eating my disk" list.
 */
export function BigFiles({
  swallow,
  root,
  onAnalyze,
  onDeleted,
}: {
  swallow: Swallow;
  root: string;
  onAnalyze: () => void;
  onDeleted: (r: DeleteReport) => void;
}) {
  const [files, setFiles] = useState<BigFile[] | null>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [minSize, setMinSize] = useState(MIN_SIZES[0]);
  const [shown, setShown] = useState(PAGE);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    setError("");
    setBusy(true);
    try {
      setFiles(await api.largestFiles(1000));
      setPicked(new Set());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const listed = useMemo(() => (files ?? []).filter((f) => f.size >= minSize), [files, minSize]);
  const total = listed.filter((f) => picked.has(f.path)).reduce((sum, f) => sum + f.size, 0);

  const toggle = (path: string) =>
    setPicked((p) => {
      const n = new Set(p);
      n.has(path) ? n.delete(path) : n.add(path);
      return n;
    });

  const remove = () => {
    const paths = [...picked];
    swallow(
      paths,
      async () => {
        const r = await api.deletePaths(paths, false);
        const gone = new Set(r.deleted);
        setFiles((f) => (f ?? []).filter((x) => !gone.has(x.path)));
        setPicked(new Set());
        onDeleted(r);
        return { freed: r.freed, failed: r.failed.length, reason: r.failed[0]?.error };
      },
      load,
    );
  };

  return (
    <div className="flex h-full w-full flex-col gap-6 overflow-y-auto">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="text-[11px] font-bold tracking-widest text-amber-400 uppercase">Varredura de volume</p>
          <h1 className="mt-1 text-3xl font-extrabold tracking-tight text-white sm:text-4xl">Arquivos grandes</h1>
          <p className="mt-1 text-sm text-slate-400">
            Os maiores arquivos da área analisada{root ? ` (${root})` : ""}. Ordenados por tamanho, não por idade.
          </p>
        </div>
        <label className="flex items-center gap-2 text-xs font-semibold text-slate-300">
          <span>Mostrar</span>
          <select
            value={minSize}
            onChange={(e) => {
              setMinSize(Number(e.target.value));
              setShown(PAGE);
              setPicked(new Set());
            }}
            className="cursor-pointer rounded-xl border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-slate-200 outline-none focus:border-amber-500"
          >
            {MIN_SIZES.map((s) => (
              <option key={s} value={s}>
                ≥ {formatBytes(s)}
              </option>
            ))}
          </select>
        </label>
      </div>

      <div className="flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-4 shadow-md backdrop-blur-md">
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={load}
            disabled={busy}
            className="inline-flex cursor-pointer items-center gap-2 rounded-xl border border-slate-700 bg-slate-800 px-4 py-2 text-xs font-semibold text-slate-200 transition-colors hover:bg-slate-750 disabled:opacity-50"
          >
            <span aria-hidden="true">🔄</span>
            <span>{busy ? "Atualizando…" : "Atualizar lista"}</span>
          </button>
          {!!listed.length && (
            <span className="text-xs text-slate-400 font-medium">
              {formatCount(listed.length)} arquivos · {formatBytes(listed.reduce((sum, f) => sum + f.size, 0))} no total
            </span>
          )}
        </div>

        {!!picked.size && (
          <button
            type="button"
            onClick={remove}
            className="inline-flex cursor-pointer items-center gap-2 rounded-xl bg-gradient-to-r from-rose-500 to-rose-600 px-5 py-2 text-xs font-bold text-white shadow-lg shadow-rose-500/25 transition-all hover:brightness-110 active:scale-[0.98]"
          >
            <span aria-hidden="true">🕳️</span>
            <span>
              Mandar {formatCount(picked.size)} para a Lixeira ({formatBytes(total)})
            </span>
          </button>
        )}
      </div>

      {error && <p className="text-xs font-semibold text-rose-400">{error}</p>}

      {files === null && !error && (
        <p className="text-xs text-slate-400" role="status">
          Carregando a lista de arquivos…
        </p>
      )}

      {files?.length === 0 && (
        <div className="flex flex-col items-center justify-center gap-4 rounded-3xl border border-slate-800/80 bg-slate-900/60 p-12 text-center shadow-lg backdrop-blur-md">
          <span className="text-4xl" aria-hidden="true">
            📁
          </span>
          <p className="max-w-md text-sm text-slate-400">
            {root
              ? "Esta análise não rastreou nenhum arquivo acima de 512 KB. Os menores ficam agrupados por pasta no mapa."
              : "Nada para listar ainda. A lista sai do mapa do disco, então analise uma pasta ou um disco primeiro."}
          </p>
          <button
            type="button"
            className="cursor-pointer rounded-xl bg-gradient-to-r from-amber-500 to-amber-400 px-6 py-2.5 text-xs font-bold text-slate-950 shadow-lg shadow-amber-500/20 hover:brightness-110"
            onClick={onAnalyze}
          >
            {root ? "Voltar ao mapa" : "Abrir o mapa do disco"}
          </button>
        </div>
      )}

      {!!files?.length && listed.length === 0 && (
        <p className="text-xs text-slate-400">Nenhum arquivo acima de {formatBytes(minSize)}.</p>
      )}

      <ul className="flex flex-col gap-2">
        {listed.slice(0, shown).map((f) => (
          <li key={f.path}>
            <label
              data-path={f.path}
              data-color={kindColor(f.kind)}
              className="group flex cursor-pointer items-center gap-3.5 rounded-2xl border border-slate-800/70 bg-slate-900/50 p-3.5 shadow-xs transition-all hover:border-slate-700 hover:bg-slate-850/60"
            >
              <input
                type="checkbox"
                checked={picked.has(f.path)}
                onChange={() => toggle(f.path)}
                className="h-4 w-4 cursor-pointer rounded border-slate-700 accent-amber-500"
              />
              <span className="h-7 w-1 shrink-0 rounded-full" style={{ background: kindColor(f.kind) }} />
              <div className="min-w-0 flex-1">
                <span className="block truncate text-sm font-semibold text-white group-hover:text-amber-300">
                  {baseName(f.path)}
                </span>
                <span className="block truncate font-mono text-[11px] text-slate-400" title={f.path}>
                  {f.path}
                </span>
              </div>
              <span className="shrink-0 text-xs text-slate-400">{formatAge(f.modified)}</span>
              <span className="shrink-0 font-mono text-sm font-bold text-amber-400">{formatBytes(f.size)}</span>
            </label>
          </li>
        ))}
      </ul>

      {listed.length > shown && (
        <button
          type="button"
          onClick={() => setShown((n) => n + PAGE)}
          className="self-center cursor-pointer rounded-xl border border-slate-700 bg-slate-800/80 px-5 py-2.5 text-xs font-semibold text-slate-200 transition-colors hover:bg-slate-750"
        >
          Mostrar mais {Math.min(PAGE, listed.length - shown)} (faltam {formatCount(listed.length - shown)})
        </button>
      )}
    </div>
  );
}
