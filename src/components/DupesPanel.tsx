import { listen } from "@tauri-apps/api/event";
import { confirm, open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { api, type DupGroup } from "../lib/api";
import { defaultPick, pruneDeleted } from "../lib/dupes";
import { baseName, formatAge, formatBytes, formatCount } from "../lib/format";
import type { Swallow } from "./types";

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
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [hashed, setHashed] = useState<number | null>(null);
  const [error, setError] = useState("");
  const [shown, setShown] = useState(PAGE);

  useEffect(() => setRoot((r) => r || defaultRoot), [defaultRoot]);
  useEffect(() => {
    const un = listen<number>("dupes-progress", (e) => setHashed(e.payload));
    return () => {
      un.then((f) => f());
    };
  }, []);

  const search = async () => {
    setError("");
    setGroups(null);
    setShown(PAGE);
    setHashed(0);
    try {
      const found = await api.findDuplicates(root, minSize);
      setGroups(found);
      setPicked(defaultPick(found));
    } catch (e) {
      setError(String(e));
    } finally {
      setHashed(null);
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
    <div className="mx-auto flex h-full w-full max-w-5xl flex-col gap-5 overflow-y-auto p-8">
      <header>
        <h1 className="text-2xl font-semibold">Arquivos duplicados</h1>
        <p className="mt-1 text-sm text-[var(--color-muted)]">
          Comparação por conteúdo (BLAKE3), não por nome. A cópia mais antiga de cada grupo é mantida por padrão.
        </p>
      </header>
      <div className="flex flex-wrap items-center gap-3">
        <button
          type="button"
          onClick={async () => {
            const dir = await open({ directory: true, defaultPath: root });
            if (typeof dir === "string") setRoot(dir);
          }}
          className="max-w-md truncate rounded-md border border-[var(--color-line)] px-3 py-2 text-sm hover:bg-[var(--color-panel-2)]"
          title={root}
        >
          <span aria-hidden="true">📁 </span>
          {root || "Escolher pasta"}
        </button>
        <select
          value={minSize}
          onChange={(e) => setMinSize(Number(e.target.value))}
          className="rounded-md border border-[var(--color-line)] bg-[var(--color-panel)] px-3 py-2 text-sm"
          aria-label="Tamanho mínimo"
        >
          {MIN_SIZES.map((s) => (
            <option key={s.value} value={s.value}>
              {s.label}
            </option>
          ))}
        </select>
        {hashed === null ? (
          <button
            type="button"
            disabled={!root}
            onClick={search}
            className="rounded-md bg-[var(--color-accent)] px-4 py-2 text-sm font-semibold text-black"
          >
            Procurar
          </button>
        ) : (
          <button
            type="button"
            onClick={() => api.cancelDuplicates()}
            className="rounded-md border border-[var(--color-line)] px-4 py-2 text-sm"
          >
            Cancelar ({formatCount(hashed)} comparados)
          </button>
        )}
        {!!picked.size && (
          <button
            type="button"
            onClick={remove}
            className="ml-auto rounded-md bg-[var(--color-danger)] px-4 py-2 text-sm font-semibold text-black"
          >
            <span aria-hidden="true">🕳️ </span>Mandar {formatCount(picked.size)} para a Lixeira ({formatBytes(wasted)})
          </button>
        )}
      </div>
      {error && <p className="text-sm text-[var(--color-danger)]">{error}</p>}
      {groups?.length === 0 && <p className="text-[var(--color-muted)]">Nenhuma duplicata encontrada. 🎉</p>}
      {!!groups?.length && (
        <p className="text-sm text-[var(--color-muted)]" role="status">
          {formatCount(groups.length)} grupos · a seleção vale para todos, inclusive os que ainda não apareceram abaixo.
        </p>
      )}
      <ul className="flex flex-col gap-3">
        {groups?.slice(0, shown).map((g) => (
          <li key={g.hash} className="rounded-lg bg-[var(--color-panel)] p-3">
            <div className="mb-2 flex justify-between text-sm">
              <span className="truncate font-medium">{baseName(g.files[0].path)}</span>
              <span className="shrink-0 text-[var(--color-muted)]">
                {g.files.length} cópias × {formatBytes(g.size)}
              </span>
            </div>
            {g.files.map((f, i) => (
              <label
                key={f.path}
                data-path={f.path}
                data-color="#8b5cd6"
                className="flex cursor-pointer items-center gap-3 rounded px-2 py-1 text-sm hover:bg-[var(--color-panel-2)]"
              >
                <input type="checkbox" checked={picked.has(f.path)} onChange={() => toggle(g, f.path)} />
                <span
                  className={`min-w-0 flex-1 truncate ${picked.has(f.path) ? "text-[var(--color-danger)] line-through" : ""}`}
                  title={f.path}
                >
                  {f.path}
                </span>
                {i === 0 && (
                  <span className="rounded bg-[var(--color-panel-2)] px-1.5 text-[10px] uppercase">original</span>
                )}
                <span className="shrink-0 text-xs text-[var(--color-muted)]">{formatAge(f.modified)}</span>
              </label>
            ))}
          </li>
        ))}
      </ul>
      {groups && groups.length > shown && (
        <button
          type="button"
          onClick={() => setShown((n) => n + PAGE)}
          className="self-center rounded-md border border-[var(--color-line)] px-4 py-2 text-sm hover:bg-[var(--color-panel-2)]"
        >
          Mostrar mais {Math.min(PAGE, groups.length - shown)} (faltam {formatCount(groups.length - shown)})
        </button>
      )}
    </div>
  );
}
