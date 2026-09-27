import { useEffect, useState } from "react";
import type { useScan } from "../hooks/useScan";
import type { ViewNode } from "../lib/api";
import { formatBytes, formatCount } from "../lib/format";
import { KIND_ORDER, KINDS } from "../lib/kinds";
import type { Mark, Marks } from "../lib/marks";
import { Sidebar } from "./Sidebar";
import { Sunburst } from "./Sunburst";
import { Treemap } from "./Treemap";

type Scan = ReturnType<typeof useScan>;

interface Props {
  scan: Scan;
  marked: Marks;
  onToggleMark: (m: Mark) => void;
}

const KEYS: [string, string][] = [
  ["espaço", "marcar"],
  ["enter", "abrir"],
  ["⌫", "voltar"],
  ["[ ]", "profundidade"],
  ["t", "modo"],
  ["r", "reanalisar"],
  ["botão direito", "marcar"],
];

function crumbs(path: string, root: string): { label: string; path: string }[] {
  const rest = path.slice(root.length).split(/[\\/]/).filter(Boolean);
  const out = [{ label: root, path: root }];
  let acc = root.replace(/[\\/]$/, "");
  const sep = root.includes("\\") ? "\\" : "/";
  for (const part of rest) {
    acc = `${acc}${sep}${part}`;
    out.push({ label: part, path: acc });
  }
  return out;
}

export function MapView({ scan, marked, onToggleMark }: Props) {
  const { view, root, total, depth, setDepth, suggestions, disk, elapsed, open, up } = scan;
  const [mode, setMode] = useState<"treemap" | "rings">("treemap");
  const [selected, setSelected] = useState<ViewNode | null>(null);

  // biome-ignore lint/correctness/useExhaustiveDependencies: selection resets whenever the focused folder changes
  useEffect(() => setSelected(null), [view?.path]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement) return;
      // A modal (delete confirmation) owns the keyboard; don't navigate the map behind it.
      if (document.querySelector("dialog[open]")) return;
      const sel = selected;
      if (e.key === " " && sel && !sel.grouped) onToggleMark(sel);
      else if (e.key === "Enter" && sel?.isDir) open(sel.path);
      else if (e.key === "Backspace") up();
      else if (e.key === "[") setDepth((d) => Math.max(1, d - 1));
      else if (e.key === "]") setDepth((d) => Math.min(8, d + 1));
      else if (e.key === "t") setMode((m) => (m === "treemap" ? "rings" : "treemap"));
      else if (e.key === "r") scan.scan(root);
      else if (e.key === "Escape") setSelected(null);
      else return;
      e.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [selected, open, up, setDepth, scan, root, onToggleMark]);

  if (!view) return null;
  const focus = selected ?? view;
  const mapProps = {
    root: view,
    selected: selected?.path ?? null,
    marked,
    onSelect: setSelected,
    onOpen: (n: ViewNode) => open(n.path),
    onToggleMark,
  };

  return (
    <div className="flex min-h-0 flex-1">
      <div className="flex min-w-0 flex-1 flex-col gap-3 p-4">
        <div className="flex items-center gap-3">
          <nav aria-label="Caminho" className="flex min-w-0 flex-1 items-center gap-1 overflow-hidden text-sm">
            {crumbs(view.path, root).map((c, i, all) => (
              <span key={c.path} className="flex min-w-0 items-center gap-1">
                {i > 0 && <span className="text-[var(--color-muted)]">/</span>}
                <button
                  type="button"
                  onClick={() => open(c.path)}
                  className={`truncate rounded px-1.5 py-0.5 hover:bg-[var(--color-panel-2)] ${i === all.length - 1 ? "bg-[var(--color-panel-2)]" : "text-[var(--color-muted)]"}`}
                >
                  {c.label}
                </button>
              </span>
            ))}
          </nav>
          <fieldset className="flex rounded-md border border-[var(--color-line)] p-0.5 text-sm">
            <legend className="sr-only">Modo de visualização</legend>
            {(["treemap", "rings"] as const).map((m) => (
              <button
                key={m}
                type="button"
                aria-pressed={mode === m}
                onClick={() => setMode(m)}
                className={`rounded px-3 py-1 ${mode === m ? "bg-[var(--color-panel-2)]" : "text-[var(--color-muted)]"}`}
              >
                {m === "treemap" ? "Blocos" : "Anéis"}
              </button>
            ))}
          </fieldset>
          <div className="flex items-center gap-1 rounded-md border border-[var(--color-line)] px-2 py-1 text-sm">
            <span className="text-[var(--color-muted)]">Profundidade {depth}</span>
            <button
              type="button"
              aria-label="Menos profundidade"
              onClick={() => setDepth((d) => Math.max(1, d - 1))}
              className="px-1.5"
            >
              −
            </button>
            <button
              type="button"
              aria-label="Mais profundidade"
              onClick={() => setDepth((d) => Math.min(8, d + 1))}
              className="px-1.5"
            >
              +
            </button>
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-[var(--color-muted)]">
          <span className="text-[var(--color-text)]">
            {formatBytes(view.size)} · {formatCount(view.files)} arquivos
          </span>
          <span className="flex items-center gap-1.5">
            <span className="hatch inline-block h-3 w-3 rounded-sm bg-[var(--color-panel-2)]" /> Recuperável
          </span>
          {KIND_ORDER.map((k) => (
            <span key={k} className="flex items-center gap-1.5">
              <span className="inline-block h-3 w-3 rounded-sm" style={{ background: KINDS[k].color }} />
              {KINDS[k].label}
            </span>
          ))}
        </div>
        <div className="min-h-0 flex-1">
          {mode === "treemap" ? <Treemap {...mapProps} /> : <Sunburst {...mapProps} onUp={up} />}
        </div>
        <footer className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-[var(--color-muted)]">
          {KEYS.map(([k, label]) => (
            <span key={k}>
              <kbd className="rounded border border-[var(--color-line)] px-1.5 py-0.5 font-mono text-[var(--color-text)]">
                {k}
              </kbd>{" "}
              {label}
            </span>
          ))}
          <span className="ml-auto">análise em {elapsed.toLocaleString("pt-BR", { maximumFractionDigits: 2 })} s</span>
        </footer>
      </div>
      <Sidebar
        selection={focus}
        scanTotal={total}
        suggestions={suggestions}
        disk={disk}
        marked={marked}
        onToggleMark={onToggleMark}
        onReveal={(p) => navigator.clipboard.writeText(p)}
      />
    </div>
  );
}
