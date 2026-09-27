import { hierarchy, treemap, treemapSquarify } from "d3-hierarchy";
import { useMemo } from "react";
import { useSize } from "../hooks/useSize";
import type { ViewNode } from "../lib/api";
import { formatBytes } from "../lib/format";
import { kindColor } from "../lib/kinds";
import type { Marks } from "../lib/marks";

interface Props {
  root: ViewNode;
  selected: string | null;
  marked: Marks;
  onSelect: (n: ViewNode) => void;
  onOpen: (n: ViewNode) => void;
  onToggleMark: (n: ViewNode) => void;
}

const HEADER = 22;

/** Space a directory keeps for itself: its size minus the children we were sent. */
function ownValue(n: ViewNode): number {
  if (!n.children.length) return n.size;
  return Math.max(0, n.size - n.children.reduce((s, c) => s + c.size, 0));
}

export function Treemap({ root, selected, marked, onSelect, onOpen, onToggleMark }: Props) {
  const [ref, { width, height }] = useSize<HTMLDivElement>();

  const nodes = useMemo(() => {
    if (width < 10 || height < 10) return [];
    const h = hierarchy(root, (d) => d.children).sum(ownValue);
    const layout = treemap<ViewNode>()
      .tile(treemapSquarify.ratio(1.3))
      .size([width, height])
      .paddingInner(2)
      .paddingOuter(3)
      .paddingTop((d) => (d.depth > 0 && d.children && d.y1 - d.y0 > HEADER * 2 ? HEADER : 3))
      .round(true);
    return layout(h)
      .descendants()
      .filter((d) => d.depth > 0 && d.x1 - d.x0 >= 3 && d.y1 - d.y0 >= 3);
  }, [root, width, height]);

  return (
    <div ref={ref} className="relative h-full w-full overflow-hidden rounded-lg" role="tree">
      {nodes.map((d) => {
        const n = d.data;
        const w = d.x1 - d.x0;
        const h = d.y1 - d.y0;
        const internal = !!d.children;
        const color = kindColor(n.kind);
        const isMarked = marked.has(n.path);
        const isSelected = selected === n.path;
        const showLabel = w > 54 && h > 20;
        return (
          <div
            key={n.path}
            data-path={n.path}
            data-color={color}
            role="treeitem"
            aria-selected={isSelected}
            aria-label={`${n.name}, ${formatBytes(n.size)}`}
            tabIndex={d.depth === 1 ? 0 : -1}
            onFocus={() => onSelect(n)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && n.isDir) onOpen(n);
            }}
            className={`tile absolute overflow-hidden rounded-[3px] ${n.reclaimable && !internal ? "hatch" : ""}`}
            style={{
              left: d.x0,
              top: d.y0,
              width: w,
              height: h,
              background: `color-mix(in srgb, ${color} ${internal ? 26 : 48}%, #0e1117)`,
              borderTop: internal ? `2px solid color-mix(in srgb, ${color} 80%, white)` : undefined,
              outline: isMarked ? "2px solid var(--color-danger)" : isSelected ? "2px solid #fff" : undefined,
              outlineOffset: -2,
              zIndex: isSelected || isMarked ? 2 : undefined,
              cursor: n.grouped ? "default" : "pointer",
            }}
            onClick={(e) => {
              e.stopPropagation();
              onSelect(n);
            }}
            onDoubleClick={(e) => {
              e.stopPropagation();
              if (n.isDir) onOpen(n);
            }}
            onContextMenu={(e) => {
              e.preventDefault();
              e.stopPropagation();
              if (!n.grouped) onToggleMark(n);
            }}
          >
            {showLabel && (
              <div
                className={`pointer-events-none flex gap-2 px-1.5 text-[12px] leading-[20px] ${internal ? "justify-between" : "flex-col gap-0 pt-0.5"}`}
              >
                <span className="truncate font-medium text-white/90">{n.name}</span>
                {(internal || h > 36) && (
                  <span className="shrink-0 text-[11px] text-white/55 tabular-nums leading-4">
                    {formatBytes(n.size)}
                  </span>
                )}
              </div>
            )}
            {isMarked && w > 18 && h > 18 && (
              <span className="pointer-events-none absolute right-1 bottom-1 rounded bg-[var(--color-danger)] px-1 text-[10px] font-bold text-black">
                ✕
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
}
