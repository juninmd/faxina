import { confirm, open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { api, type EmptyItem } from "../lib/api";
import { formatCount } from "../lib/format";
import type { Swallow } from "./types";

const LABEL = { folder: "pasta vazia", link: "link quebrado" } as const;

export function EmptyPanel({ swallow, defaultRoot }: { swallow: Swallow; defaultRoot: string }) {
  const [root, setRoot] = useState(defaultRoot);
  const [items, setItems] = useState<EmptyItem[] | null>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());
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
    <div className="mx-auto flex h-full w-full max-w-5xl flex-col gap-5 overflow-y-auto p-8">
      <header>
        <h1 className="text-2xl font-semibold">Pastas vazias e links quebrados</h1>
        <p className="mt-1 text-sm text-[var(--color-muted)]">
          Só aparece a pasta mais externa de cada cadeia vazia. Nada vem marcado: revise antes, algumas pastas vazias
          são espaços reservados por programas.
        </p>
      </header>
      <div className="flex flex-wrap items-center gap-3">
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
          className="max-w-md truncate rounded-md border border-[var(--color-line)] px-3 py-2 text-sm hover:bg-[var(--color-panel-2)]"
          title={root}
        >
          <span aria-hidden="true">📁 </span>
          {root || "Escolher pasta"}
        </button>
        <button
          type="button"
          disabled={!root || busy}
          onClick={search}
          className="rounded-md bg-[var(--color-accent)] px-4 py-2 text-sm font-semibold text-black disabled:opacity-50"
        >
          {busy ? "Procurando…" : "Procurar"}
        </button>
        {!!items?.length && (
          <button
            type="button"
            onClick={() => setPicked(picked.size === items.length ? new Set() : new Set(items.map((i) => i.path)))}
            className="rounded-md border border-[var(--color-line)] px-3 py-2 text-sm hover:bg-[var(--color-panel-2)]"
          >
            {picked.size === items.length ? "Desmarcar todos" : "Marcar todos"}
          </button>
        )}
        {!!picked.size && (
          <button
            type="button"
            onClick={remove}
            className="ml-auto rounded-md bg-[var(--color-danger)] px-4 py-2 text-sm font-semibold text-black"
          >
            <span aria-hidden="true">🕳️ </span>Limpar {formatCount(picked.size)}
          </button>
        )}
      </div>
      {error && <p className="text-sm text-[var(--color-danger)]">{error}</p>}
      {items?.length === 0 && <p className="text-[var(--color-muted)]">Nada vazio ou quebrado por aqui. 🎉</p>}
      <ul className="flex flex-col gap-1">
        {items?.map((i) => (
          <li key={i.path}>
            <label
              data-path={i.path}
              data-color="#8b5cd6"
              className="flex cursor-pointer items-center gap-3 rounded px-2 py-1 text-sm hover:bg-[var(--color-panel-2)]"
            >
              <input type="checkbox" checked={picked.has(i.path)} onChange={() => toggle(i.path)} />
              <span className="min-w-0 flex-1 truncate" title={i.path}>
                {i.path}
              </span>
              <span className="shrink-0 rounded bg-[var(--color-panel-2)] px-1.5 text-[10px] uppercase">
                {LABEL[i.kind]}
              </span>
            </label>
          </li>
        ))}
      </ul>
    </div>
  );
}
