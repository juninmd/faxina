import { save } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useState } from "react";
import { api, type DeleteReport, type ExportFormat, type SearchHit } from "../lib/api";
import { baseName, formatAge, formatBytes, formatCount } from "../lib/format";
import { KIND_ORDER, KINDS, kindColor } from "../lib/kinds";
import type { OrbitItem } from "../lib/orbit";
import {
  AGE_STEPS,
  EMPTY_FORM,
  EXPORT_FORMATS,
  exportName,
  isBlank,
  type SearchForm,
  SIZE_STEPS,
  toQuery,
} from "../lib/search";
import OrbitButton from "./OrbitButton";
import type { Swallow } from "./types";

const SELECT =
  "cursor-pointer rounded-xl border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-slate-200 outline-none focus:border-amber-500";
const PAGE = 100;

/**
 * Instant search over the scanned map: name (with `*` and `?`), size, kind and "untouched for N
 * months". Only files the scan tracked individually show up; the results can be exported or sent
 * to the Órbita like any other list.
 */
export function Search({
  swallow,
  root,
  onAnalyze,
  onDeleted,
  onOrbit,
}: {
  swallow: Swallow;
  root: string;
  onAnalyze: () => void;
  onDeleted: (r: DeleteReport) => void;
  onOrbit?: (items: OrbitItem[]) => void;
}) {
  const [form, setForm] = useState<SearchForm>(EMPTY_FORM);
  const [hits, setHits] = useState<SearchHit[] | null>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [shown, setShown] = useState(PAGE);
  const [error, setError] = useState("");
  const [note, setNote] = useState("");

  const blank = isBlank(form);
  const query = useMemo(() => toQuery(form), [form]);

  // Debounced: the backend answers from memory, so typing feels live without a request per key.
  useEffect(() => {
    if (!root || blank) {
      setHits(null);
      setError("");
      return;
    }
    let live = true;
    const t = setTimeout(async () => {
      try {
        const r = await api.searchFiles(query);
        if (!live) return;
        setHits(r);
        setPicked(new Set());
        setShown(PAGE);
        setError("");
      } catch (e) {
        if (!live) return;
        // The old list answered other filters: keeping it ticked would remove files this query never matched.
        setHits(null);
        setPicked(new Set());
        setError(String(e));
      }
    }, 150);
    return () => {
      live = false;
      clearTimeout(t);
    };
  }, [query, blank, root]);

  const patch = (p: Partial<SearchForm>) => setForm((f) => ({ ...f, ...p }));
  const list = hits ?? [];
  const total = list.reduce((n, h) => n + h.size, 0);
  const pickedSize = list.filter((h) => picked.has(h.path)).reduce((n, h) => n + h.size, 0);

  const toggle = (path: string) =>
    setPicked((p) => {
      const n = new Set(p);
      n.has(path) ? n.delete(path) : n.add(path);
      return n;
    });

  const exportAs = async (format: ExportFormat) => {
    setNote("");
    try {
      const path = await save({
        defaultPath: exportName(format),
        filters: [{ name: format.toUpperCase(), extensions: [format] }],
      });
      if (!path) return;
      const n = await api.exportReport(query, format, path);
      setNote(`${formatCount(n)} itens salvos em ${path}`);
    } catch (e) {
      setError(String(e));
    }
  };

  const toOrbit = () => {
    onOrbit?.(
      list
        .filter((h) => picked.has(h.path))
        .map((h) => ({
          path: h.path,
          name: h.name,
          size: h.size,
          color: kindColor(h.kind),
          route: "path",
          source: "Buscar",
        })),
    );
    setPicked(new Set());
  };

  const remove = () => {
    const paths = [...picked];
    swallow(
      paths,
      async () => {
        const r = await api.deletePaths(paths, false);
        const gone = new Set(r.deleted);
        setHits((h) => (h ?? []).filter((x) => !gone.has(x.path)));
        setPicked(new Set());
        onDeleted(r);
        return { freed: r.freed, failed: r.failed.length, reason: r.failed[0]?.error };
      },
      () => {},
    );
  };

  return (
    <div className="flex h-full w-full flex-col gap-6 overflow-y-auto">
      <div>
        <p className="text-[11px] font-bold tracking-widest text-amber-400 uppercase">Busca instantânea</p>
        <h1 className="mt-1 text-3xl font-extrabold tracking-tight text-white sm:text-4xl">Buscar arquivos</h1>
        <p className="mt-1 text-sm text-slate-400">
          Procure no mapa{root ? ` (${root})` : ""} por nome, tamanho, tipo ou tempo parado. Use * e ? como curingas,
          por exemplo <span className="font-mono text-slate-300">*.mkv</span>. Arquivos pequenos ficam agrupados por
          pasta e não aparecem aqui.
        </p>
      </div>

      {!root ? (
        <div className="flex flex-col items-center gap-4 rounded-3xl border border-slate-800/80 bg-slate-900/60 p-12 text-center">
          <p className="max-w-md text-sm text-slate-400">A busca usa o mapa do disco. Analise uma pasta primeiro.</p>
          <button
            type="button"
            className="cursor-pointer rounded-xl bg-gradient-to-r from-amber-500 to-amber-400 px-6 py-2.5 text-xs font-bold text-slate-950"
            onClick={onAnalyze}
          >
            Abrir o mapa do disco
          </button>
        </div>
      ) : (
        <>
          <div className="flex flex-wrap items-center gap-3 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-4">
            <input
              type="search"
              value={form.text}
              onChange={(e) => patch({ text: e.target.value })}
              placeholder="Nome ou padrão (*.log, backup*)"
              aria-label="Nome do arquivo"
              className="min-w-56 flex-1 rounded-xl border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-slate-100 outline-none focus:border-amber-500"
            />
            <select
              className={SELECT}
              aria-label="Tamanho mínimo"
              value={form.minSize}
              onChange={(e) => patch({ minSize: Number(e.target.value) })}
            >
              {SIZE_STEPS.map((s) => (
                <option key={s.value} value={s.value}>
                  {s.label}
                </option>
              ))}
            </select>
            <select
              className={SELECT}
              aria-label="Tempo sem alteração"
              value={form.ageMonths}
              onChange={(e) => patch({ ageMonths: Number(e.target.value) })}
            >
              {AGE_STEPS.map((s) => (
                <option key={s.months} value={s.months}>
                  {s.label}
                </option>
              ))}
            </select>
            <select
              className={SELECT}
              aria-label="Tipo"
              value={form.kind ?? ""}
              onChange={(e) => patch({ kind: (e.target.value || null) as SearchForm["kind"] })}
            >
              <option value="">Todos os tipos</option>
              {KIND_ORDER.map((k) => (
                <option key={k} value={k}>
                  {KINDS[k].label}
                </option>
              ))}
            </select>
            <label className="flex items-center gap-2 text-xs font-semibold text-slate-300">
              <input
                type="checkbox"
                checked={form.includeDirs && form.ageMonths === 0}
                disabled={form.ageMonths > 0}
                onChange={(e) => patch({ includeDirs: e.target.checked })}
                className="h-4 w-4 accent-amber-500"
              />
              Incluir pastas
            </label>
          </div>

          {blank && (
            <p className="text-xs text-slate-400" role="status">
              Digite um nome ou escolha um filtro para começar.
            </p>
          )}

          {!blank && hits && (
            <div className="flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-4">
              <span className="text-xs font-medium text-slate-400" role="status">
                {formatCount(list.length)} resultados · {formatBytes(total)}
              </span>
              <div className="flex flex-wrap items-center gap-2">
                <span className="text-xs font-semibold text-slate-400">Exportar</span>
                {EXPORT_FORMATS.map((f) => (
                  <button
                    key={f.value}
                    type="button"
                    disabled={!list.length}
                    onClick={() => exportAs(f.value)}
                    className="cursor-pointer rounded-lg border border-slate-700 bg-slate-800 px-3 py-1.5 text-[11px] font-semibold text-slate-200 hover:bg-slate-750 disabled:opacity-40"
                  >
                    {f.label}
                  </button>
                ))}
                {!!picked.size && onOrbit && <OrbitButton count={picked.size} onClick={toOrbit} />}
                {!!picked.size && (
                  <button
                    type="button"
                    onClick={remove}
                    className="cursor-pointer rounded-xl bg-gradient-to-r from-rose-500 to-rose-600 px-4 py-2 text-xs font-bold text-white"
                  >
                    Mandar {formatCount(picked.size)} para a Lixeira ({formatBytes(pickedSize)})
                  </button>
                )}
              </div>
            </div>
          )}

          {note && (
            <p className="text-xs font-semibold text-emerald-300" role="status">
              {note}
            </p>
          )}
          {error && (
            <div
              role="alert"
              className="rounded-xl border border-rose-500/30 bg-rose-500/10 p-3 text-xs font-semibold text-rose-300"
            >
              {error}
            </div>
          )}

          {!blank && hits?.length === 0 && <p className="text-xs text-slate-400">Nada encontrado com esses filtros.</p>}

          <ul className="flex flex-col gap-2">
            {list.slice(0, shown).map((h) => (
              <li key={h.path}>
                <label
                  data-path={h.path}
                  data-color={kindColor(h.kind)}
                  className="group flex cursor-pointer items-center gap-3.5 rounded-2xl border border-slate-800/70 bg-slate-900/50 p-3.5 transition-all hover:border-slate-700"
                >
                  <input
                    type="checkbox"
                    checked={picked.has(h.path)}
                    onChange={() => toggle(h.path)}
                    className="h-4 w-4 cursor-pointer accent-amber-500"
                  />
                  <span className="h-7 w-1 shrink-0 rounded-full" style={{ background: kindColor(h.kind) }} />
                  <div className="min-w-0 flex-1">
                    <span className="block truncate text-sm font-semibold text-white group-hover:text-amber-300">
                      {h.isDir ? "📁 " : ""}
                      {baseName(h.path)}
                    </span>
                    <span className="block truncate font-mono text-[11px] text-slate-400" title={h.path}>
                      {h.path}
                    </span>
                  </div>
                  <span className="shrink-0 text-xs text-slate-400">{formatAge(h.modified)}</span>
                  <span className="shrink-0 font-mono text-sm font-bold text-amber-400">{formatBytes(h.size)}</span>
                </label>
              </li>
            ))}
          </ul>

          {list.length > shown && (
            <button
              type="button"
              onClick={() => setShown((n) => n + PAGE)}
              className="self-center cursor-pointer rounded-xl border border-slate-700 bg-slate-800/80 px-5 py-2.5 text-xs font-semibold text-slate-200"
            >
              Mostrar mais {Math.min(PAGE, list.length - shown)} (faltam {formatCount(list.length - shown)})
            </button>
          )}
        </>
      )}
    </div>
  );
}
