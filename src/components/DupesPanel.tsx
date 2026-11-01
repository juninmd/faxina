import { listen } from "@tauri-apps/api/event";
import { confirm, open } from "@tauri-apps/plugin-dialog";
import { useEffect, useRef, useState } from "react";
import { api, type DupGroup } from "../lib/api";
import { defaultPick, dupNotice, pruneDeleted } from "../lib/dupes";
import { baseName, formatAge, formatBytes, formatCount } from "../lib/format";
import type { Swallow } from "./types";

/** Progress sample from the backend: the stage now running and how far it is. */
interface DupTick {
  stage: number;
  done: number;
  total: number;
}

const MIN_SIZES = [
  { label: "≥ 100 KB", value: 100 * 1024 },
  { label: "≥ 1 MB", value: 1024 * 1024 },
  { label: "≥ 10 MB", value: 10 * 1024 * 1024 },
  { label: "≥ 100 MB", value: 100 * 1024 * 1024 },
];

/** Groups rendered per page: thousands of rows at once freeze the webview. */
const PAGE = 50;

export function DupesPanel({ swallow, defaultRoot }: { swallow: Swallow; defaultRoot: string }) {
  const [root, setRoot] = useState(defaultRoot);
  const [minSize, setMinSize] = useState(MIN_SIZES[1].value);
  const [groups, setGroups] = useState<DupGroup[] | null>(null);
  const [dupMeta, setDupMeta] = useState<{ cancelled: boolean; truncated: boolean }>({
    cancelled: false,
    truncated: false,
  });
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [tick, setTick] = useState<DupTick | null>(null);
  const [error, setError] = useState("");
  const [shown, setShown] = useState(PAGE);

  useEffect(() => setRoot((r) => r || defaultRoot), [defaultRoot]);
  useEffect(() => {
    const un = listen<DupTick>("dupes-progress", (e) => setTick(e.payload));
    return () => {
      un.then((f) => f());
    };
  }, []);

  // Results belong to the folder and size they were searched with; a change makes them stale.
  // A search still running when that happens must not write its results either.
  const epoch = useRef(0);
  const resetResults = () => {
    epoch.current += 1;
    // A search still running is no longer wanted: stop it and clear its progress, or the button sticks.
    if (tick) api.cancelDuplicates();
    setTick(null);
    setGroups(null);
    setPicked(new Set());
    setError("");
  };

  const search = async () => {
    const mine = epoch.current;
    setError("");
    setGroups(null);
    setShown(PAGE);
    setTick({ stage: 0, done: 0, total: 0 });
    try {
      const found = await api.findDuplicates(root, minSize);
      if (mine !== epoch.current) return;
      setGroups(found.groups);
      setDupMeta({ cancelled: found.cancelled, truncated: found.truncated });
      setPicked(defaultPick(found.groups));
    } catch (e) {
      if (mine === epoch.current) setError(String(e));
    } finally {
      if (mine === epoch.current) setTick(null);
    }
  };

  const toggle = (group: DupGroup, path: string) =>
    setPicked((p) => {
      const n = new Set(p);
      if (n.has(path)) n.delete(path);
      // Never let every copy of a file be selected: one must survive.
      else if (group.files.filter((f) => !n.has(f.path)).length > 1) n.add(path);
      return n;
    });

  const wasted = (groups ?? []).reduce((s, g) => s + g.files.filter((f) => picked.has(f.path)).length * g.size, 0);

  const remove = async () => {
    const paths = [...picked];
    const ok = await confirm(
      `${formatCount(paths.length)} arquivos (${formatBytes(wasted)}) vão para a Lixeira. Uma cópia de cada grupo é mantida.`,
      {
        title: "Mandar duplicatas para a Lixeira?",
        kind: "warning",
        okLabel: "Mandar para a Lixeira",
        cancelLabel: "Cancelar",
      },
    );
    if (!ok) return;
    let deleted: string[] = [];
    swallow(
      paths,
      async () => {
        const r = await api.deletePaths(paths, false);
        deleted = r.deleted;
        return { freed: r.freed, failed: r.failed.length, reason: r.failed[0]?.error };
      },
      () => {
        setGroups((gs) => pruneDeleted(gs ?? [], deleted));
        setPicked(new Set());
      },
    );
  };

  return (
    <div className="flex h-full w-full flex-col gap-6 overflow-y-auto">
      <div>
        <p className="text-[11px] font-bold tracking-widest text-amber-400 uppercase">Varredura de redundância</p>
        <h1 className="mt-1 text-3xl font-extrabold tracking-tight text-white sm:text-4xl">Arquivos duplicados</h1>
        <p className="mt-1 text-sm text-slate-400">
          Comparação por conteúdo (BLAKE3), não por nome. A cópia mais antiga de cada grupo é mantida por padrão.
        </p>
      </div>

      <div className="flex flex-wrap items-center gap-3 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-4 shadow-md backdrop-blur-md">
        <button
          type="button"
          onClick={async () => {
            try {
              const dir = await open({ directory: true, defaultPath: root });
              if (typeof dir === "string") {
                setRoot(dir);
                resetResults();
              }
            } catch (e) {
              setError(String(e));
            }
          }}
          className="flex max-w-md cursor-pointer items-center gap-2 truncate rounded-xl border border-slate-700 bg-slate-800 px-3.5 py-2 text-xs font-semibold text-slate-200 transition-colors hover:bg-slate-750"
          title={root}
        >
          <span aria-hidden="true">📁</span>
          <span className="truncate">{root || "Escolher pasta"}</span>
        </button>

        <select
          value={minSize}
          onChange={(e) => {
            setMinSize(Number(e.target.value));
            resetResults();
          }}
          className="cursor-pointer rounded-xl border border-slate-700 bg-slate-800 px-3 py-2 text-xs font-semibold text-slate-200 outline-none focus:border-amber-500"
          aria-label="Tamanho mínimo"
        >
          {MIN_SIZES.map((s) => (
            <option key={s.value} value={s.value}>
              {s.label}
            </option>
          ))}
        </select>

        {tick === null ? (
          <button
            type="button"
            disabled={!root}
            onClick={search}
            className="cursor-pointer rounded-xl bg-gradient-to-r from-amber-500 to-amber-400 px-5 py-2 text-xs font-bold text-slate-950 shadow-lg shadow-amber-500/20 hover:brightness-110 disabled:opacity-50"
          >
            Procurar
          </button>
        ) : (
          <button
            type="button"
            onClick={() => api.cancelDuplicates()}
            className="cursor-pointer rounded-xl border border-slate-700 bg-slate-800 px-4 py-2 text-xs font-semibold text-slate-200 hover:bg-slate-750"
          >
            Cancelar (
            {tick.total ? `etapa ${tick.stage}: ${formatCount(tick.done)} de ${formatCount(tick.total)}` : "preparando"}
            )
          </button>
        )}

        {!!picked.size && (
          <button
            type="button"
            onClick={remove}
            className="ml-auto inline-flex cursor-pointer items-center gap-2 rounded-xl bg-gradient-to-r from-rose-500 to-rose-600 px-5 py-2 text-xs font-bold text-white shadow-lg shadow-rose-500/25 transition-all hover:brightness-110 active:scale-[0.98]"
          >
            <span aria-hidden="true">🕳️</span>
            <span>
              Mandar {formatCount(picked.size)} para a Lixeira ({formatBytes(wasted)})
            </span>
          </button>
        )}
      </div>

      {error && <p className="text-xs font-semibold text-rose-400">{error}</p>}
      {dupNotice(dupMeta) && (
        <p className="rounded-xl border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-200" role="status">
          {dupNotice(dupMeta)}
        </p>
      )}

      {groups?.length === 0 && (
        <div className="flex flex-col items-center justify-center gap-3 rounded-3xl border border-slate-800/80 bg-slate-900/60 p-12 text-center shadow-lg backdrop-blur-md">
          <span className="text-4xl" aria-hidden="true">
            🎉
          </span>
          <p className="text-sm font-semibold text-white">Nenhuma duplicata encontrada.</p>
        </div>
      )}

      {!!groups?.length && (
        <p className="text-xs text-slate-400 font-medium" role="status">
          {formatCount(groups.length)} grupos · a seleção vale para todos, inclusive os que ainda não apareceram abaixo.
        </p>
      )}

      <ul className="flex flex-col gap-4">
        {groups?.slice(0, shown).map((g) => (
          <li
            key={g.hash}
            className="flex flex-col gap-2 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-4 shadow-sm backdrop-blur-md"
          >
            <div className="flex items-center justify-between pb-2 border-b border-slate-800/60 text-xs">
              <span className="truncate font-semibold text-white">{baseName(g.files[0].path)}</span>
              <span className="shrink-0 font-mono text-amber-400">
                {g.files.length} cópias × {formatBytes(g.size)}
              </span>
            </div>
            <div className="flex flex-col gap-1.5">
              {g.files.map((f, i) => (
                <label
                  key={f.path}
                  data-path={f.path}
                  data-color="#8b5cd6"
                  className="flex cursor-pointer items-center gap-3 rounded-xl border border-slate-800/50 bg-slate-950/40 px-3 py-2 text-xs transition-colors hover:border-slate-700 hover:bg-slate-850/50"
                >
                  <input
                    type="checkbox"
                    checked={picked.has(f.path)}
                    onChange={() => toggle(g, f.path)}
                    className="h-4 w-4 cursor-pointer rounded border-slate-700 accent-amber-500"
                  />
                  <span
                    className={`min-w-0 flex-1 truncate font-mono text-[11px] ${
                      picked.has(f.path) ? "text-rose-400 line-through opacity-70" : "text-slate-300"
                    }`}
                    title={f.path}
                  >
                    {f.path}
                  </span>
                  {i === 0 && (
                    <span className="rounded-full border border-blue-500/30 bg-blue-500/10 px-2 py-0.5 text-[9px] font-bold text-blue-400 uppercase tracking-wider">
                      original
                    </span>
                  )}
                  <span className="shrink-0 text-[11px] text-slate-400">{formatAge(f.modified)}</span>
                </label>
              ))}
            </div>
          </li>
        ))}
      </ul>

      {groups && groups.length > shown && (
        <button
          type="button"
          onClick={() => setShown((n) => n + PAGE)}
          className="self-center cursor-pointer rounded-xl border border-slate-700 bg-slate-800/80 px-5 py-2.5 text-xs font-semibold text-slate-200 transition-colors hover:bg-slate-750"
        >
          Mostrar mais {Math.min(PAGE, groups.length - shown)} (faltam {formatCount(groups.length - shown)})
        </button>
      )}
    </div>
  );
}
