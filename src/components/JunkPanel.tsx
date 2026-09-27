import { confirm } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useMemo, useState } from "react";
import { api, type JunkItem } from "../lib/api";
import { formatBytes, formatCount } from "../lib/format";
import type { Swallow } from "./types";

const GROUP_COLORS: Record<string, string> = {
  Sistema: "#4f7dd6",
  Navegadores: "#c9a13b",
  Apps: "#8b5cd6",
  Desenvolvimento: "#c47a45",
};

export function JunkPanel({ swallow }: { swallow: Swallow }) {
  const [items, setItems] = useState<JunkItem[] | null>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [note, setNote] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const list = await api.junkScan();
      setItems(list);
      setPicked(new Set(list.map((i) => i.id)));
    } catch (e) {
      setError(`Não foi possível listar os caches: ${e}`);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const groups = useMemo(() => {
    const map = new Map<string, JunkItem[]>();
    for (const i of items ?? []) map.set(i.group, [...(map.get(i.group) ?? []), i]);
    return [...map.entries()];
  }, [items]);
  const total = (items ?? []).filter((i) => picked.has(i.id)).reduce((s, i) => s + i.size, 0);

  const toggle = (id: string) =>
    setPicked((p) => {
      const n = new Set(p);
      n.has(id) ? n.delete(id) : n.add(id);
      return n;
    });

  const clean = async () => {
    const ids = [...picked];
    const ok = await confirm(
      `${formatBytes(total)} de cache serão apagados direto, sem passar pela Lixeira. Os apps recriam esses arquivos quando precisarem.`,
      { title: "Limpar caches?", kind: "warning", okLabel: "Limpar", cancelLabel: "Cancelar" },
    );
    if (!ok) return;
    swallow(
      ids.map((id) => `junk:${id}`),
      async () => {
        const r = await api.junkClean(ids);
        setNote(r.skipped ? `${formatCount(r.skipped)} itens em uso foram mantidos.` : "");
        return { freed: r.freed, failed: 0 };
      },
      load,
    );
  };

  return (
    <div className="mx-auto flex h-full w-full max-w-4xl flex-col gap-5 overflow-y-auto p-8">
      <header className="flex items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold">Limpeza rápida</h1>
          <p className="mt-1 text-sm text-[var(--color-muted)]">
            Caches e temporários que os apps recriam sozinhos. Nunca tocamos em cookies, senhas ou histórico.
          </p>
        </div>
        <button
          type="button"
          disabled={!total || loading}
          onClick={clean}
          className="shrink-0 rounded-lg bg-[var(--color-accent)] px-5 py-2.5 font-semibold text-black disabled:opacity-40"
        >
          <span aria-hidden="true">🧹 </span>Limpar {formatBytes(total)}
        </button>
      </header>
      {note && (
        <p className="text-sm text-[var(--color-muted)]" role="status">
          {note}
        </p>
      )}
      {error && (
        <p className="text-sm text-[var(--color-danger)]" role="alert">
          {error}{" "}
          <button type="button" onClick={load} className="underline">
            Tentar de novo
          </button>
        </p>
      )}
      {loading && !items && <p className="animate-pulse text-[var(--color-muted)]">Procurando caches…</p>}
      {items?.length === 0 && <p className="text-[var(--color-muted)]">Tudo limpo por aqui. ✨</p>}
      {groups.map(([group, list]) => (
        <section key={group} aria-label={group}>
          <h2 className="mb-2 text-[11px] tracking-wider text-[var(--color-muted)] uppercase">{group}</h2>
          <ul className="flex flex-col gap-1.5">
            {list.map((i) => (
              <li key={i.id}>
                <label
                  data-path={`junk:${i.id}`}
                  data-color={GROUP_COLORS[group] ?? "#e3b341"}
                  className="flex cursor-pointer items-center gap-3 rounded-lg bg-[var(--color-panel)] px-4 py-3 hover:bg-[var(--color-panel-2)]"
                >
                  <input type="checkbox" checked={picked.has(i.id)} onChange={() => toggle(i.id)} />
                  <span className="h-8 w-1 rounded" style={{ background: GROUP_COLORS[group] }} />
                  <span className="min-w-0 flex-1">
                    <span className="block">{i.label}</span>
                    <span className="block truncate text-xs text-[var(--color-muted)]" title={i.paths.join("\n")}>
                      {i.paths[0]}
                      {i.paths.length > 1 && ` +${i.paths.length - 1}`}
                    </span>
                  </span>
                  <span className="text-xs text-[var(--color-muted)]">{formatCount(i.files)} arquivos</span>
                  <span className="w-20 text-right tabular-nums">{formatBytes(i.size)}</span>
                </label>
              </li>
            ))}
          </ul>
        </section>
      ))}
    </div>
  );
}
