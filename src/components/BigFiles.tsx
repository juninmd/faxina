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

  const load = useCallback(async () => {
    setError("");
    try {
      setFiles(await api.largestFiles(1000));
      setPicked(new Set());
    } catch (e) {
      setError(String(e));
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
    <div className="mx-auto flex h-full w-full max-w-5xl flex-col gap-5 overflow-y-auto p-8">
      <header className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold">Arquivos grandes</h1>
          <p className="mt-1 text-sm text-[var(--color-muted)]">
            Os maiores arquivos da área analisada{root ? ` (${root})` : ""}. Ordenados por tamanho, não por idade.
          </p>
        </div>
        <label className="flex items-center gap-2 text-sm text-[var(--color-muted)]">
          Mostrar
          <select
            value={minSize}
            onChange={(e) => {
              setMinSize(Number(e.target.value));
              setShown(PAGE);
              // A hidden row must never stay selected: the button would delete something the
              // user can no longer see, with a byte count computed from the visible rows.
              setPicked(new Set());
            }}
            className="rounded-md border border-[var(--color-line)] bg-[var(--color-panel)] px-3 py-2 text-[var(--color-text)]"
          >
            {MIN_SIZES.map((s) => (
              <option key={s} value={s}>
                ≥ {formatBytes(s)}
              </option>
            ))}
          </select>
        </label>
      </header>

      <div className="flex flex-wrap items-center gap-3">
        <button
          type="button"
          onClick={load}
          className="rounded-md border border-[var(--color-line)] px-4 py-2 text-sm hover:bg-[var(--color-panel-2)]"
        >
          <span aria-hidden="true">🔄 </span>Atualizar lista
        </button>
        {!!picked.size && (
          <button
            type="button"
            onClick={remove}
            className="ml-auto rounded-md bg-[var(--color-danger)] px-4 py-2 text-sm font-semibold text-black"
          >
            <span aria-hidden="true">🕳️ </span>Mandar {formatCount(picked.size)} para a Lixeira ({formatBytes(total)})
          </button>
        )}
      </div>

      {error && <p className="text-sm text-[var(--color-danger)]">{error}</p>}

      {files === null && !error && (
        <p className="text-[var(--color-muted)]" role="status">
          Carregando a lista de arquivos…
        </p>
      )}
      {files?.length === 0 && (
        <div className="panel flex flex-col items-start gap-3 p-6">
          <p className="text-[var(--color-muted)]">
            {root
              ? "Esta análise não rastreou nenhum arquivo acima de 512 KB. Os menores ficam agrupados por pasta no mapa."
              : "Nada para listar ainda. A lista sai do mapa do disco, então analise uma pasta ou um disco primeiro."}
          </p>
          <button type="button" className="button primary" onClick={onAnalyze}>
            {root ? "Voltar ao mapa" : "Abrir o mapa do disco"}
          </button>
        </div>
      )}
      {!!files?.length && listed.length === 0 && (
        <p className="text-[var(--color-muted)]">Nenhum arquivo acima de {formatBytes(minSize)}.</p>
      )}

      {!!listed.length && (
        <p className="text-sm text-[var(--color-muted)]" role="status">
          {formatCount(listed.length)} arquivos · {formatBytes(listed.reduce((sum, f) => sum + f.size, 0))} no total
        </p>
      )}

      <ul className="flex flex-col gap-1.5">
        {listed.slice(0, shown).map((f) => (
          <li key={f.path}>
            <label
              data-path={f.path}
              data-color={kindColor(f.kind)}
              className="flex cursor-pointer items-center gap-3 rounded-lg bg-[var(--color-panel)] px-4 py-2.5 hover:bg-[var(--color-panel-2)]"
            >
              <input type="checkbox" checked={picked.has(f.path)} onChange={() => toggle(f.path)} />
              <span className="h-6 w-1 shrink-0 rounded" style={{ background: kindColor(f.kind) }} />
              <span className="min-w-0 flex-1">
                <span className="block truncate">{baseName(f.path)}</span>
                <span className="block truncate text-xs text-[var(--color-muted)]" title={f.path}>
                  {f.path}
                </span>
              </span>
              <span className="shrink-0 text-xs text-[var(--color-muted)]">{formatAge(f.modified)}</span>
              <span className="w-20 shrink-0 text-right tabular-nums">{formatBytes(f.size)}</span>
            </label>
          </li>
        ))}
      </ul>

      {listed.length > shown && (
        <button
          type="button"
          onClick={() => setShown((n) => n + PAGE)}
          className="self-center rounded-md border border-[var(--color-line)] px-4 py-2 text-sm hover:bg-[var(--color-panel-2)]"
        >
          Mostrar mais {Math.min(PAGE, listed.length - shown)} (faltam {formatCount(listed.length - shown)})
        </button>
      )}
    </div>
  );
}
