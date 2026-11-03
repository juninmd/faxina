import { confirm, open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { api, type EmptyItem } from "../lib/api";
import { formatCount } from "../lib/format";
import type { Swallow } from "./types";

const LABEL = { folder: "pasta vazia", link: "link quebrado" } as const;
const PAGE = 50;

export function EmptyPanel({ swallow, defaultRoot }: { swallow: Swallow; defaultRoot: string }) {
  const [root, setRoot] = useState(defaultRoot);
  const [items, setItems] = useState<EmptyItem[] | null>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [shown, setShown] = useState(PAGE);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => setRoot((r) => r || defaultRoot), [defaultRoot]);

  const search = async () => {
    setError("");
    setItems(null);
    setBusy(true);
    try {
      setItems(await api.findEmpty(root));
      setPicked(new Set());
      setShown(PAGE);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const toggle = (path: string) =>
    setPicked((p) => {
      const n = new Set(p);
      if (!n.delete(path)) n.add(path);
      return n;
    });

  const remove = async () => {
    const chosen = (items ?? []).filter((i) => picked.has(i.path));
    const folders = chosen.filter((i) => i.kind === "folder").map((i) => i.path);
    const links = chosen.filter((i) => i.kind === "link").map((i) => i.path);
    const ok = await confirm(
      `${formatCount(folders.length)} pastas vazias vão para a Lixeira e ${formatCount(links.length)} links quebrados serão removidos (só o link, nunca o destino).`,
      { title: "Limpar itens vazios?", kind: "warning", okLabel: "Limpar", cancelLabel: "Cancelar" },
    );
    if (!ok) return;
    const deleted = new Set<string>();
    swallow(
      chosen.map((i) => i.path),
      async () => {
        const reports = [
          folders.length ? await api.deletePaths(folders, false) : null,
          links.length ? await api.deleteBrokenLinks(links) : null,
        ].filter((r) => r !== null);
        for (const r of reports) for (const d of r.deleted) deleted.add(d);
        return {
          freed: 0,
          failed: reports.reduce((s, r) => s + r.failed.length, 0),
          reason: reports.flatMap((r) => r.failed)[0]?.error,
        };
      },
      () => {
        setItems((is) => (is ?? []).filter((i) => !deleted.has(i.path)));
        setPicked(new Set());
      },
    );
  };

  return (
    <div className="flex h-full w-full flex-col gap-6 overflow-y-auto">
      <div>
        <p className="text-[11px] font-bold tracking-widest text-amber-400 uppercase">Organização do sistema</p>
        <h1 className="mt-1 text-3xl font-extrabold tracking-tight text-white sm:text-4xl">
          Pastas vazias e links quebrados
        </h1>
        <p className="mt-1 text-sm text-slate-400">
          Só aparece a pasta mais externa de cada cadeia vazia. Nada vem marcado: revise antes, algumas pastas vazias
          são espaços reservados por programas.
        </p>
      </div>

      <div className="flex flex-wrap items-center gap-3 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-4 shadow-md backdrop-blur-md">
        <button
          type="button"
          onClick={async () => {
            try {
              const dir = await open({ directory: true, defaultPath: root });
              if (typeof dir === "string") setRoot(dir);
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

        <button
          type="button"
          disabled={!root || busy}
          onClick={search}
          className="cursor-pointer rounded-xl bg-gradient-to-r from-amber-500 to-amber-400 px-5 py-2 text-xs font-bold text-slate-950 shadow-lg shadow-amber-500/20 hover:brightness-110 disabled:opacity-50"
        >
          {busy ? "Procurando…" : "Procurar"}
        </button>

        {!!items?.length && (
          <button
            type="button"
            onClick={() => setPicked(picked.size === items.length ? new Set() : new Set(items.map((i) => i.path)))}
            className="cursor-pointer rounded-xl border border-slate-700 bg-slate-800 px-3.5 py-2 text-xs font-semibold text-slate-200 transition-colors hover:bg-slate-750"
          >
            {picked.size === items.length ? "Desmarcar todos" : "Marcar todos"}
          </button>
        )}

        {!!picked.size && (
          <button
            type="button"
            onClick={remove}
            className="ml-auto inline-flex cursor-pointer items-center gap-2 rounded-xl bg-gradient-to-r from-rose-500 to-rose-600 px-5 py-2 text-xs font-bold text-white shadow-lg shadow-rose-500/25 transition-all hover:brightness-110 active:scale-[0.98]"
          >
            <span aria-hidden="true">🕳️</span>
            <span>Limpar {formatCount(picked.size)}</span>
          </button>
        )}
      </div>

      {error && (
        <div
          role="alert"
          className="rounded-xl border border-rose-500/30 bg-rose-500/10 p-3 text-xs font-semibold text-rose-300"
        >
          {error}
        </div>
      )}

      {items?.length === 0 && (
        <div
          role="status"
          className="flex flex-col items-center justify-center gap-3 rounded-3xl border border-slate-800/80 bg-slate-900/60 p-12 text-center shadow-lg backdrop-blur-md"
        >
          <span className="text-4xl" aria-hidden="true">
            🎉
          </span>
          <h2 className="text-sm font-semibold text-white">Nada vazio ou quebrado por aqui.</h2>
          <p className="text-xs text-slate-400">Nenhuma pasta vazia ou link quebrado foi detectado nesta área.</p>
        </div>
      )}

      {!!items?.length && (
        <p className="text-xs text-slate-400 font-medium" role="status">
          {formatCount(items.length)} itens encontrados
        </p>
      )}

      <ul className="flex flex-col gap-2">
        {items?.slice(0, shown).map((i) => (
          <li key={i.path}>
            <label
              data-path={i.path}
              data-color="#8b5cd6"
              className="group flex cursor-pointer items-center gap-3.5 rounded-xl border border-slate-800/70 bg-slate-900/50 p-3 shadow-xs transition-colors hover:border-slate-700 hover:bg-slate-850/60"
            >
              <input
                type="checkbox"
                checked={picked.has(i.path)}
                onChange={() => toggle(i.path)}
                className="h-4 w-4 cursor-pointer rounded border-slate-700 accent-amber-500"
              />
              <span
                className="min-w-0 flex-1 truncate font-mono text-[11px] text-slate-300 group-hover:text-white"
                title={i.path}
              >
                {i.path}
              </span>
              <span
                className={`rounded-full px-2 py-0.5 text-[10px] font-bold uppercase tracking-wider ${
                  i.kind === "folder"
                    ? "border border-amber-500/30 bg-amber-500/10 text-amber-400"
                    : "border border-rose-500/30 bg-rose-500/10 text-rose-400"
                }`}
              >
                {LABEL[i.kind]}
              </span>
            </label>
          </li>
        ))}
      </ul>

      {items && items.length > shown && (
        <button
          type="button"
          onClick={() => setShown((n) => n + PAGE)}
          className="self-center cursor-pointer rounded-xl border border-slate-700 bg-slate-800/80 px-5 py-2.5 text-xs font-semibold text-slate-200 transition-colors hover:bg-slate-750"
        >
          Mostrar mais {Math.min(PAGE, items.length - shown)} (faltam {formatCount(items.length - shown)})
        </button>
      )}
    </div>
  );
}
