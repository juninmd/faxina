import { useEffect, useRef, useState } from "react";
import { formatBytes } from "../lib/format";
import { kindColor } from "../lib/kinds";
import type { Mark } from "../lib/marks";

interface Props {
  items: Mark[];
  onClear: () => void;
  onUnmark: (path: string) => void;
  onDelete: (permanent: boolean) => void;
}

/** DaisyDisk-style collector: marked items wait here until the user confirms. */
export function Collector({ items, onClear, onUnmark, onDelete }: Props) {
  const [confirming, setConfirming] = useState(false);
  const [permanent, setPermanent] = useState(false);
  const dialog = useRef<HTMLDialogElement>(null);
  const total = items.reduce((s, m) => s + m.size, 0);

  useEffect(() => {
    if (confirming) dialog.current?.showModal();
    else dialog.current?.close();
  }, [confirming]);

  if (!items.length) return null;

  return (
    <>
      <div className="pop-in flex items-center gap-4 border-t border-[var(--color-line)] bg-[var(--color-panel)] px-5 py-3">
        <div className="flex min-w-0 flex-1 items-center gap-2 overflow-x-auto">
          {items.slice(0, 12).map((m) => (
            <button
              key={m.path}
              type="button"
              onClick={() => onUnmark(m.path)}
              title={`${m.path} — clique para tirar`}
              className="flex shrink-0 items-center gap-1.5 rounded-full border border-[var(--color-line)] px-2.5 py-1 text-xs hover:border-[var(--color-danger)]"
            >
              <span className="h-2 w-2 rounded-full" style={{ background: kindColor(m.kind) }} />
              <span className="max-w-40 truncate">{m.name}</span>
              <span className="text-[var(--color-muted)]">{formatBytes(m.size)}</span>
            </button>
          ))}
          {items.length > 12 && <span className="text-xs text-[var(--color-muted)]">+{items.length - 12}</span>}
        </div>
        <button type="button" onClick={onClear} className="text-sm text-[var(--color-muted)] hover:text-white">
          Limpar seleção
        </button>
        <button
          type="button"
          onClick={() => setConfirming(true)}
          className="rounded-lg bg-[var(--color-danger)] px-4 py-2 font-semibold text-black shadow-[0_0_24px_rgba(240,96,93,0.35)] hover:brightness-110"
        >
          🕳️ Excluir {formatBytes(total)}
        </button>
      </div>

      <dialog
        ref={dialog}
        onClose={() => setConfirming(false)}
        className="m-auto w-[440px] rounded-xl border border-[var(--color-line)] bg-[var(--color-panel)] p-6 text-[var(--color-text)] backdrop:bg-black/60"
      >
        <h2 className="text-lg font-semibold">
          Excluir {items.length} {items.length === 1 ? "item" : "itens"}?
        </h2>
        <p className="mt-2 text-sm text-[var(--color-muted)]">
          {formatBytes(total)} serão{" "}
          {permanent ? "apagados para sempre" : "movidos para a Lixeira — dá pra recuperar depois"}.
        </p>
        <label className="mt-4 flex items-center gap-2 text-sm">
          <input type="checkbox" checked={permanent} onChange={(e) => setPermanent(e.target.checked)} />
          Excluir permanentemente (sem Lixeira)
        </label>
        <div className="mt-6 flex justify-end gap-2">
          <button
            type="button"
            onClick={() => setConfirming(false)}
            className="rounded-md px-4 py-2 text-sm hover:bg-[var(--color-panel-2)]"
          >
            Cancelar
          </button>
          <button
            type="button"
            onClick={() => {
              setConfirming(false);
              onDelete(permanent);
            }}
            className="rounded-md bg-[var(--color-danger)] px-4 py-2 text-sm font-semibold text-black"
          >
            Excluir
          </button>
        </div>
      </dialog>
    </>
  );
}
