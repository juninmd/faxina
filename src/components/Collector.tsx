import { useEffect, useRef, useState } from "react";
import { formatBytes } from "../lib/format";
import { kindColor } from "../lib/kinds";
import type { Mark } from "../lib/marks";
import CountUp from "./CountUp";

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
  // Permanent delete needs its own acknowledgement, same as the confirmation dialog.
  const [acknowledged, setAcknowledged] = useState(false);
  // Cancelling must not leave "sem Lixeira" ticked for the next open.
  const closeConfirm = () => {
    setConfirming(false);
    setPermanent(false);
    setAcknowledged(false);
  };
  const dialog = useRef<HTMLDialogElement>(null);
  const total = items.reduce((s, m) => s + m.size, 0);

  useEffect(() => {
    if (confirming) dialog.current?.showModal();
    else dialog.current?.close();
  }, [confirming]);

  // The dialog unmounts with the list, so a stale "confirming" would never reopen it.
  // "Sem Lixeira" must not survive either, or the next delete is permanent without a choice.
  useEffect(() => {
    if (!items.length) {
      setConfirming(false);
      setPermanent(false);
    }
  }, [items.length]);

  if (!items.length) return null;

  return (
    <>
      <div className="pop-in sticky bottom-3 z-20 mx-4 mb-3 flex items-center gap-4 rounded-2xl border border-slate-700/80 bg-slate-950/95 px-5 py-3.5 shadow-2xl backdrop-blur-xl">
        <div className="flex shrink-0 flex-col pr-1">
          <span className="text-[10px] font-bold tracking-wider text-slate-400 uppercase">
            {items.length} {items.length === 1 ? "item" : "itens"}
          </span>
          <span className="font-mono text-lg font-bold text-white tabular-nums">
            <CountUp value={total} format={formatBytes} />
          </span>
        </div>
        <div className="flex min-w-0 flex-1 items-center gap-2 overflow-x-auto">
          {items.slice(0, 12).map((m) => (
            <button
              key={m.path}
              type="button"
              onClick={() => onUnmark(m.path)}
              title={`${m.path} — clique para tirar`}
              className="flex shrink-0 cursor-pointer items-center gap-1.5 rounded-full border border-slate-700/80 bg-slate-900/60 px-3 py-1 text-xs text-slate-200 transition-colors hover:border-rose-500 hover:text-white"
            >
              <span className="h-2 w-2 rounded-full" style={{ background: kindColor(m.kind) }} />
              <span className="max-w-40 truncate">{m.name}</span>
              <span className="font-mono text-slate-400">{formatBytes(m.size)}</span>
              <span className="text-slate-400 hover:text-rose-400">×</span>
            </button>
          ))}
          {items.length > 12 && <span className="text-xs text-slate-400">+{items.length - 12}</span>}
        </div>
        <button
          type="button"
          onClick={onClear}
          className="cursor-pointer text-xs font-semibold text-slate-400 transition-colors hover:text-white"
        >
          Limpar seleção
        </button>
        <button
          type="button"
          onClick={() => setConfirming(true)}
          className="inline-flex cursor-pointer items-center gap-1.5 rounded-xl bg-gradient-to-r from-rose-500 to-rose-600 px-5 py-2.5 text-xs font-bold text-white shadow-lg shadow-rose-500/25 transition-all hover:brightness-110 active:scale-[0.98]"
        >
          <span aria-hidden="true">🕳️</span>
          <span>Excluir seleção</span>
        </button>
      </div>

      <dialog
        ref={dialog}
        onClose={closeConfirm}
        aria-labelledby="collector-title"
        className="m-auto w-full max-w-md rounded-3xl border border-slate-700/80 bg-slate-900/95 p-7 text-slate-100 shadow-2xl backdrop:bg-black/75 backdrop:backdrop-blur-md backdrop-blur-xl animate-in fade-in zoom-in-95 duration-200"
      >
        <h2 id="collector-title" className="text-lg font-bold text-white">
          Excluir {items.length} {items.length === 1 ? "item" : "itens"}?
        </h2>
        <p className="mt-2 text-xs leading-relaxed text-slate-400">
          <strong className="font-mono text-slate-200">{formatBytes(total)}</strong> serão{" "}
          {permanent ? "apagados para sempre" : "movidos para a Lixeira — dá pra recuperar depois"}.
        </p>
        <label className="mt-4 flex cursor-pointer items-center gap-2.5 text-xs text-slate-300">
          <input
            type="checkbox"
            checked={permanent}
            onChange={(e) => {
              setPermanent(e.target.checked);
              setAcknowledged(false);
            }}
            className="h-4 w-4 cursor-pointer rounded border-slate-700 accent-rose-500"
          />
          <span>Excluir permanentemente (sem Lixeira)</span>
        </label>
        {permanent && (
          <label className="mt-3 flex cursor-pointer items-start gap-2.5 rounded-xl border border-rose-500/30 bg-rose-500/10 p-3 text-xs leading-relaxed text-rose-200">
            <input
              type="checkbox"
              checked={acknowledged}
              onChange={(e) => setAcknowledged(e.target.checked)}
              className="mt-0.5 h-4 w-4 cursor-pointer rounded border-slate-700 accent-rose-500"
            />
            <span>Entendo que a exclusão permanente não pode ser desfeita pela Lixeira.</span>
          </label>
        )}
        <div className="mt-6 flex justify-end gap-2.5 pt-4 border-t border-slate-800">
          <button
            type="button"
            onClick={closeConfirm}
            className="cursor-pointer rounded-xl border border-slate-700 bg-slate-800 px-4 py-2 text-xs font-semibold text-slate-300 transition-colors hover:bg-slate-750"
          >
            Cancelar
          </button>
          <button
            type="button"
            disabled={permanent && !acknowledged}
            onClick={() => {
              setConfirming(false);
              onDelete(permanent);
            }}
            className="cursor-pointer rounded-xl bg-gradient-to-r from-rose-500 to-rose-600 px-5 py-2 text-xs font-bold text-white shadow-lg shadow-rose-500/25 transition-all hover:brightness-110 active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-40"
          >
            Excluir
          </button>
        </div>
      </dialog>
    </>
  );
}
