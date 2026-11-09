import { confirm, open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { api, type Worktree } from "../lib/api";
import { formatAge, formatBytes, formatCount } from "../lib/format";
import type { Swallow } from "./types";

const PAGE = 50;

/**
 * Linked git worktrees under a folder. The backend only marks a checkout as solta when git says
 * every commit is on a remote-tracking ref and nothing local was changed; the rest stays visible,
 * disabled and explained, so the user knows what to push before cleaning.
 */
export function WorktreesPanel({ swallow, defaultRoot }: { swallow: Swallow; defaultRoot: string }) {
  const [root, setRoot] = useState(defaultRoot);
  const [items, setItems] = useState<Worktree[] | null>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [shown, setShown] = useState(PAGE);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => setRoot((r) => r || defaultRoot), [defaultRoot]);

  const pickable = (items ?? []).filter((w) => w.synced);
  const chosen = (items ?? []).filter((w) => picked.has(w.path));
  const chosenSize = chosen.reduce((s, w) => s + w.size, 0);
  const found = (items ?? []).reduce((s, w) => s + w.size, 0);

  const reset = () => {
    setItems(null);
    setPicked(new Set());
    setError("");
  };

  const search = async () => {
    setError("");
    setItems(null);
    setBusy(true);
    try {
      const list = await api.findWorktrees(root);
      setItems(list);
      setPicked(new Set(list.filter((w) => w.synced).map((w) => w.path)));
      setShown(PAGE);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const toggle = (path: string) =>
    setPicked((p) => {
      const next = new Set(p);
      if (!next.delete(path)) next.add(path);
      return next;
    });

  const toggleAll = () => setPicked(picked.size === pickable.length ? new Set() : new Set(pickable.map((w) => w.path)));

  const remove = async () => {
    const ok = await confirm(
      `${formatCount(chosen.length)} worktrees (${formatBytes(chosenSize)}) vão para a Lixeira. Os commits e as branches continuam no repositório principal.`,
      { title: "Remover worktrees?", kind: "warning", okLabel: "Mandar para a Lixeira", cancelLabel: "Cancelar" },
    );
    if (!ok) return;
    let removed: string[] = [];
    swallow(
      chosen.map((w) => w.path),
      async () => {
        const report = await api.deleteWorktrees(
          chosen.map((w) => w.path),
          false,
        );
        removed = report.removed;
        return { freed: report.freed, failed: report.failed.length, reason: report.failed[0]?.error };
      },
      () => {
        setItems((list) => (list ?? []).filter((w) => !removed.includes(w.path)));
        setPicked(new Set());
      },
    );
  };

  return (
    <div className="flex h-full w-full flex-col gap-6 overflow-y-auto">
      <div>
        <p className="text-[11px] font-bold tracking-widest text-amber-400 uppercase">Higiene do repositório</p>
        <h1 className="mt-1 text-3xl font-extrabold tracking-tight text-white sm:text-4xl">Worktrees do Git</h1>
        <p className="mt-1 text-sm text-slate-400">
          Checkouts criados com <code className="font-mono text-slate-300">git worktree add</code>. Só entram na seleção
          os que já têm todos os commits no remoto e nenhuma alteração local; o resto aparece bloqueado com o motivo.
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
                reset();
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

        <button
          type="button"
          disabled={!root || busy}
          onClick={search}
          className="cursor-pointer rounded-xl bg-gradient-to-r from-amber-500 to-amber-400 px-5 py-2 text-xs font-bold text-slate-950 shadow-lg shadow-amber-500/20 hover:brightness-110 disabled:opacity-50"
        >
          {busy ? "Verificando com o git…" : "Procurar worktrees"}
        </button>

        {!!pickable.length && (
          <button
            type="button"
            onClick={toggleAll}
            className="cursor-pointer rounded-xl border border-slate-700 bg-slate-800 px-3.5 py-2 text-xs font-semibold text-slate-200 transition-colors hover:bg-slate-750"
          >
            {picked.size === pickable.length ? "Desmarcar sincronizadas" : "Marcar sincronizadas"}
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
              Remover {formatCount(picked.size)} ({formatBytes(chosenSize)})
            </span>
          </button>
        )}
      </div>

      <p className="flex items-start gap-2 rounded-xl border border-slate-800/70 bg-slate-950/40 p-3 text-xs leading-relaxed text-slate-400">
        <span aria-hidden="true">🔎</span>
        <span>
          A conferência usa o <code className="font-mono text-slate-300">git</code> instalado no PATH e os refs remotos
          do último fetch — nada é enviado à rede. Pastas ignoradas (build, dependências, <code>.env</code>) vão junto
          com o checkout; a Lixeira continua sendo o caminho de volta.
        </span>
      </p>

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
          <h2 className="text-sm font-semibold text-white">Nenhuma worktree vinculada nesta pasta.</h2>
          <p className="max-w-md text-xs text-slate-400">
            O checkout principal do repositório nunca aparece aqui: só os criados com{" "}
            <code className="font-mono">git worktree add</code>, dentro da pasta escolhida.
          </p>
        </div>
      )}

      {!!items?.length && (
        <p className="text-xs font-medium text-slate-400" role="status">
          {formatCount(items.length)} worktrees · {formatBytes(found)} no total · {formatCount(pickable.length)} já
          sincronizadas
        </p>
      )}

      <ul className="flex flex-col gap-2">
        {items?.slice(0, shown).map((w) => {
          const disabled = !w.synced;
          return (
            <li key={w.path}>
              <label
                data-path={w.path}
                data-color={disabled ? "#8b5cd6" : "#e3b341"}
                className={`group flex cursor-pointer items-start gap-3.5 rounded-xl border p-3 shadow-xs transition-colors ${
                  disabled
                    ? "cursor-not-allowed border-slate-800/60 bg-slate-950/30 opacity-70"
                    : "border-slate-800/70 bg-slate-900/50 hover:border-slate-700 hover:bg-slate-850/60"
                }`}
              >
                <input
                  type="checkbox"
                  checked={picked.has(w.path)}
                  disabled={disabled}
                  onChange={() => toggle(w.path)}
                  className="mt-0.5 h-4 w-4 cursor-pointer rounded border-slate-700 accent-amber-500 disabled:cursor-not-allowed"
                />
                <span className="flex min-w-0 flex-1 flex-col gap-1.5">
                  <code className="truncate font-mono text-[11px] text-slate-300 group-hover:text-white" title={w.path}>
                    {w.path}
                  </code>
                  <span className="flex flex-wrap items-center gap-2 text-[11px] text-slate-400">
                    {w.branch && (
                      <span className="rounded-full border border-slate-700 bg-slate-800/80 px-2 py-0.5 font-mono text-[10px] text-slate-200">
                        {w.branch}
                      </span>
                    )}
                    {w.head && <span className="font-mono text-slate-400">{w.head}</span>}
                    <span>{formatBytes(w.size)}</span>
                    <span>{formatCount(w.files)} arquivos</span>
                    <span>último commit {formatAge(w.modified)}</span>
                  </span>
                  {w.synced ? (
                    <span className="w-fit rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 text-[10px] font-bold tracking-wider text-emerald-400 uppercase">
                      tudo no remoto · pode remover
                    </span>
                  ) : (
                    <span className="w-fit text-[11px] font-medium break-words text-amber-400">
                      Mantida: {w.reason ?? "trabalho local"}
                    </span>
                  )}
                </span>
              </label>
            </li>
          );
        })}
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
