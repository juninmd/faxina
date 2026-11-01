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
      if (e.target instanceof HTMLElement && e.target.closest("input, select, textarea, button")) return;
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
      <div className="flex min-w-0 flex-1 flex-col gap-3 p-5">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <nav aria-label="Caminho" className="flex min-w-0 flex-1 items-center gap-1 overflow-hidden text-xs">
            {crumbs(view.path, root).map((c, i, all) => (
              <span key={c.path} className="flex min-w-0 items-center gap-1">
                {i > 0 && <span className="text-slate-400">/</span>}
                <button
                  type="button"
                  onClick={() => open(c.path)}
                  className={`cursor-pointer truncate rounded-lg px-2 py-1 font-mono transition-colors hover:bg-slate-800 hover:text-white ${
                    i === all.length - 1 ? "bg-slate-800 text-amber-300 font-semibold" : "text-slate-400"
                  }`}
                >
                  {c.label}
                </button>
              </span>
            ))}
          </nav>
          <div className="flex items-center gap-2">
            <fieldset className="flex rounded-xl border border-slate-700/80 bg-slate-850 p-1 text-xs">
              <legend className="sr-only">Modo de visualização</legend>
              {(["treemap", "rings"] as const).map((m) => (
                <button
                  key={m}
                  type="button"
                  aria-pressed={mode === m}
                  onClick={() => setMode(m)}
                  className={`cursor-pointer rounded-lg px-3 py-1 font-medium transition-all ${
                    mode === m
                      ? "bg-slate-700 text-white shadow-xs font-semibold"
                      : "text-slate-400 hover:text-slate-200"
                  }`}
                >
                  {m === "treemap" ? "Blocos" : "Anéis"}
                </button>
              ))}
            </fieldset>
            <div className="flex items-center gap-1.5 rounded-xl border border-slate-700/80 bg-slate-850 px-3 py-1 text-xs">
              <span className="text-slate-400">Profundidade</span>
              <span className="font-mono font-bold text-white">{depth}</span>
              <div className="flex items-center gap-0.5 ml-1">
                <button
                  type="button"
                  aria-label="Menos profundidade"
                  onClick={() => setDepth((d) => Math.max(1, d - 1))}
                  className="cursor-pointer rounded-md px-1.5 py-0.5 hover:bg-slate-700 text-slate-300 hover:text-white font-bold"
                >
                  −
                </button>
                <button
                  type="button"
                  aria-label="Mais profundidade"
                  onClick={() => setDepth((d) => Math.min(8, d + 1))}
                  className="cursor-pointer rounded-md px-1.5 py-0.5 hover:bg-slate-700 text-slate-300 hover:text-white font-bold"
                >
                  +
                </button>
              </div>
            </div>
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5 text-xs text-slate-400">
          <span className="font-semibold text-slate-200">
            {formatBytes(view.size)} · {formatCount(view.files)} arquivos
          </span>
          <span className="flex items-center gap-1.5">
            <span className="hatch inline-block h-2.5 w-2.5 rounded-xs bg-slate-800" />
            <span>Recuperável</span>
          </span>
          {KIND_ORDER.map((k) => (
            <span key={k} className="flex items-center gap-1.5">
              <span className="inline-block h-2.5 w-2.5 rounded-xs" style={{ background: KINDS[k].color }} />
              <span>{KINDS[k].label}</span>
            </span>
          ))}
        </div>

        <div className="min-h-0 flex-1 rounded-xl border border-slate-800/80 bg-slate-950/60 p-1">
          {mode === "treemap" ? <Treemap {...mapProps} /> : <Sunburst {...mapProps} onUp={up} />}
        </div>

        <footer className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-slate-400">
          {KEYS.map(([k, label]) => (
            <span key={k} className="flex items-center gap-1">
              <kbd className="rounded-md border border-slate-700/80 bg-slate-800/80 px-1.5 py-0.5 font-mono text-[10px] text-slate-200">
                {k}
              </kbd>
              <span className="text-[11px] text-slate-400">{label}</span>
            </span>
          ))}
          <span className="ml-auto font-mono text-[11px] text-slate-400">
            análise em {elapsed.toLocaleString("pt-BR", { maximumFractionDigits: 2 })} s
          </span>
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
