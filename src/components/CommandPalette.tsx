import { useEffect, useMemo, useRef, useState } from "react";
import { type Command, filterCommands } from "../lib/spaces";
import Icon from "./Icon";

export type PaletteCommand = Command & { run: () => void };

export default function CommandPalette({ commands, onClose }: { commands: PaletteCommand[]; onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null);
  const [query, setQuery] = useState("");
  const [index, setIndex] = useState(0);
  const list = useMemo(() => filterCommands(commands, query), [commands, query]);
  const active = Math.min(index, Math.max(0, list.length - 1));

  useEffect(() => {
    const dialog = ref.current;
    const previous = document.activeElement;
    dialog?.showModal();
    return () => {
      dialog?.close();
      if (previous instanceof HTMLElement) previous.focus({ preventScroll: true });
    };
  }, []);

  useEffect(() => {
    ref.current?.querySelector(`[data-index="${active}"]`)?.scrollIntoView({ block: "nearest" });
  }, [active]);

  const run = (c: PaletteCommand | undefined) => {
    if (!c) return;
    onClose();
    c.run();
  };

  return (
    <dialog
      ref={ref}
      aria-label="Paleta de comandos"
      className="palette"
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
      onKeyDown={(e) => {
        if (e.key === "ArrowDown") {
          e.preventDefault();
          setIndex(Math.min(list.length - 1, active + 1));
        } else if (e.key === "ArrowUp") {
          e.preventDefault();
          setIndex(Math.max(0, active - 1));
        } else if (e.key === "Enter") {
          e.preventDefault();
          run(list[active]);
        }
      }}
    >
      <div className="palette-box">
        <div className="flex items-center gap-3 border-b border-slate-800 px-4">
          <Icon name="search" size={18} />
          <input
            autoFocus
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setIndex(0);
            }}
            placeholder="Ir para, analisar, mapear…"
            aria-label="Buscar comando"
            aria-controls="palette-list"
            aria-activedescendant={list[active] ? `cmd-${list[active].id}` : undefined}
            className="w-full bg-transparent py-4 text-base text-white outline-none placeholder:text-slate-500"
          />
          <kbd className="kbd">Esc</kbd>
        </div>
        <div id="palette-list" role="listbox" className="max-h-80 overflow-y-auto p-2">
          {list.map((c, i) => (
            <div
              key={c.id}
              id={`cmd-${c.id}`}
              role="option"
              tabIndex={-1}
              aria-selected={i === active}
              data-index={i}
              className="palette-item"
              onMouseMove={() => setIndex(i)}
              onClick={() => run(c)}
              onKeyDown={() => {}}
            >
              <span>{c.label}</span>
              <small>{c.hint}</small>
            </div>
          ))}
          {!list.length && (
            <p className="px-3 py-6 text-center text-sm text-slate-400">Nenhum comando com “{query}”.</p>
          )}
        </div>
      </div>
    </dialog>
  );
}
