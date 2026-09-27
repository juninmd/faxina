import { hierarchy, partition } from "d3-hierarchy";
import { useMemo, useState } from "react";
import { useSize } from "../hooks/useSize";
import type { ViewNode } from "../lib/api";
import { formatBytes, splitBytes } from "../lib/format";
import { kindColor } from "../lib/kinds";
import type { Marks } from "../lib/marks";

interface Props {
  root: ViewNode;
  selected: string | null;
  marked: Marks;
  onSelect: (n: ViewNode) => void;
  onOpen: (n: ViewNode) => void;
  onUp: () => void;
  onToggleMark: (n: ViewNode) => void;
}

const RINGS = 5;

function arc(a0: number, a1: number, r0: number, r1: number): string {
  const span = Math.min(a1 - a0, Math.PI * 2 - 1e-4);
  const end = a0 + span;
  const large = span > Math.PI ? 1 : 0;
  const p = (a: number, r: number) => `${r * Math.sin(a)},${-r * Math.cos(a)}`;
  return `M${p(a0, r1)}A${r1},${r1} 0 ${large} 1 ${p(end, r1)}L${p(end, r0)}A${r0},${r0} 0 ${large} 0 ${p(a0, r0)}Z`;
}

/** DaisyDisk-style rings: the center is the current folder, each ring one level deeper. */
export function Sunburst({ root, selected, marked, onSelect, onOpen, onUp, onToggleMark }: Props) {
  const [ref, { width, height }] = useSize<HTMLDivElement>();
  const [hover, setHover] = useState<ViewNode | null>(null);
  const radius = Math.max(0, Math.min(width, height) / 2 - 8);
  const ring = radius / (RINGS + 1);

  const arcs = useMemo(() => {
    const h = hierarchy(root, (d) => d.children).sum((n) =>
      n.children.length ? Math.max(0, n.size - n.children.reduce((s, c) => s + c.size, 0)) : n.size,
    );
    return partition<ViewNode>()
      .size([Math.PI * 2, RINGS + 1])(h)
      .descendants()
      .filter((d) => d.depth > 0 && d.depth <= RINGS && d.x1 - d.x0 > 0.004);
  }, [root]);

  const focus = hover ?? root;
  const [num, unit] = splitBytes(focus.size);

  return (
    <div ref={ref} className="relative flex h-full w-full items-center justify-center">
      <svg
        width={width}
        height={height}
        viewBox={`${-width / 2} ${-height / 2} ${width} ${height}`}
        role="img"
        aria-label="Mapa em anéis"
      >
        {arcs.map((d) => {
          const n = d.data;
          const isMarked = marked.has(n.path);
          const lift = selected === n.path || hover?.path === n.path;
          return (
            // biome-ignore lint/a11y/useSemanticElements: SVG arcs cannot be <button> elements
            <path
              key={n.path}
              data-path={n.path}
              data-color={kindColor(n.kind)}
              d={arc(d.x0, d.x1, d.y0 * ring + 1, d.y1 * ring - 1)}
              fill={`color-mix(in srgb, ${kindColor(n.kind)} ${lift ? 85 : 62 - d.depth * 6}%, #0e1117)`}
              stroke={isMarked ? "var(--color-danger)" : "#0e1117"}
              strokeWidth={isMarked ? 2.5 : 1}
              className="cursor-pointer transition-[fill] duration-150"
              onMouseEnter={() => setHover(n)}
              onMouseLeave={() => setHover(null)}
              role="button"
              tabIndex={d.depth === 1 ? 0 : -1}
              aria-label={`${n.name}, ${formatBytes(n.size)}`}
              onFocus={() => onSelect(n)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && n.isDir) onOpen(n);
              }}
              onClick={() => onSelect(n)}
              onDoubleClick={() => n.isDir && onOpen(n)}
              onContextMenu={(e) => {
                e.preventDefault();
                if (!n.grouped) onToggleMark(n);
              }}
            >
              <title>{`${n.name} — ${formatBytes(n.size)}`}</title>
            </path>
          );
        })}
        {/* biome-ignore lint/a11y/useSemanticElements: SVG shapes cannot be <button> elements */}
        <circle
          r={Math.max(0, ring - 2)}
          fill="#161b24"
          className="cursor-pointer"
          role="button"
          tabIndex={0}
          aria-label="Voltar um nível"
          onClick={onUp}
          onKeyDown={(e) => e.key === "Enter" && onUp()}
        >
          <title>Voltar um nível</title>
        </circle>
      </svg>
      <div
        className="pointer-events-none absolute flex flex-col items-center text-center"
        style={{ width: ring * 1.7 }}
      >
        <span className="w-full truncate text-xs text-[var(--color-muted)]">{focus.name}</span>
        <span className="text-2xl font-semibold tabular-nums">
          {num}
          <span className="ml-1 text-sm text-[var(--color-muted)]">{unit}</span>
        </span>
      </div>
    </div>
  );
}
