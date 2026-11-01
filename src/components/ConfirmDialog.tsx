import { useEffect, useRef, useState } from "react";
import { gb, type Report } from "../domain";
import Icon from "./Icon";

export default function ConfirmDialog({
  amount,
  onClose,
  onConfirm,
}: {
  amount: number;
  onClose: () => void;
  onConfirm: (destination: Report["destination"]) => void;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const [destination, setDestination] = useState<Report["destination"]>("trash");
  const [acknowledged, setAcknowledged] = useState(false);

  // Every switch needs a fresh acknowledgement, so an earlier tick never carries over.
  const choose = (next: Report["destination"]) => {
    setDestination(next);
    setAcknowledged(false);
  };

  useEffect(() => {
    const dialog = ref.current;
    const previous = document.activeElement;
    dialog?.showModal();
    return () => {
      dialog?.close();
      if (previous instanceof HTMLElement) previous.focus({ preventScroll: true });
    };
  }, []);

  return (
    <dialog
      ref={ref}
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
      aria-labelledby="confirm-title"
      className="m-auto w-full max-w-lg rounded-3xl border border-slate-700/80 bg-slate-900/95 p-7 text-slate-100 shadow-2xl backdrop:bg-black/75 backdrop:backdrop-blur-md backdrop-blur-xl animate-in fade-in zoom-in-95 duration-200"
    >
      <div className="flex items-center justify-between">
        <span className="flex h-12 w-12 items-center justify-center rounded-2xl border border-amber-500/30 bg-amber-500/15 text-amber-400 shadow-sm">
          <Icon name="trash" size={24} />
        </span>
        <button
          type="button"
          className="cursor-pointer rounded-xl p-2 text-slate-400 transition-colors hover:bg-slate-800 hover:text-white"
          aria-label="Fechar confirmação"
          onClick={onClose}
        >
          <Icon name="close" size={18} />
        </button>
      </div>

      <p className="mt-4 text-[10px] font-bold tracking-widest text-amber-400 uppercase">Última revisão</p>
      <h2 id="confirm-title" className="mt-1 text-2xl font-bold tracking-tight text-white">
        Como você quer limpar?
      </h2>
      <p className="mt-1 mb-5 text-sm text-slate-400">
        <strong className="font-mono text-slate-200">{gb(amount)}</strong> selecionados para limpeza.
      </p>

      <div className="flex flex-col gap-3">
        <label
          className={`flex cursor-pointer items-start gap-4 rounded-2xl border p-4 transition-all ${
            destination === "trash"
              ? "border-emerald-500/50 bg-emerald-500/10 shadow-sm shadow-emerald-500/10"
              : "border-slate-800 bg-slate-850/50 hover:border-slate-700"
          }`}
        >
          <input
            type="radio"
            name="destination"
            checked={destination === "trash"}
            onChange={() => choose("trash")}
            className="mt-1 h-4 w-4 cursor-pointer accent-emerald-500"
          />
          <div className="flex flex-col">
            <div className="flex items-center gap-2">
              <strong className="text-sm font-semibold text-white">Enviar à Lixeira</strong>
              <span className="rounded-full border border-emerald-500/30 bg-emerald-500/20 px-2 py-0.5 text-[10px] font-bold tracking-wider text-emerald-300 uppercase">
                Recomendado
              </span>
            </div>
            <span className="mt-1 text-xs leading-relaxed text-slate-400">
              Você pode restaurar pelo sistema quando a Lixeira aceitar o item. O espaço ainda não fica disponível de
              imediato.
            </span>
          </div>
        </label>

        <label
          className={`flex cursor-pointer items-start gap-4 rounded-2xl border p-4 transition-all ${
            destination === "permanent"
              ? "border-rose-500/50 bg-rose-500/10 shadow-sm shadow-rose-500/10"
              : "border-slate-800 bg-slate-850/50 hover:border-slate-700"
          }`}
        >
          <input
            type="radio"
            name="destination"
            checked={destination === "permanent"}
            onChange={() => choose("permanent")}
            className="mt-1 h-4 w-4 cursor-pointer accent-rose-500"
          />
          <div className="flex flex-col">
            <strong className="text-sm font-semibold text-white">Excluir permanentemente</strong>
            <span className="mt-1 text-xs leading-relaxed text-slate-400">
              Sem restauração pela Lixeira. Libera espaço imediato de armazenamento no disco.
            </span>
          </div>
        </label>
      </div>

      {destination === "permanent" && (
        <label className="mt-3 flex cursor-pointer items-start gap-3 rounded-xl border border-rose-500/30 bg-rose-500/10 p-3 text-xs leading-relaxed text-rose-200">
          <input
            type="checkbox"
            checked={acknowledged}
            onChange={(e) => setAcknowledged(e.target.checked)}
            className="mt-0.5 h-4 w-4 cursor-pointer accent-rose-500"
          />
          <span>Entendo que a exclusão permanente não pode ser desfeita pela Lixeira do sistema.</span>
        </label>
      )}

      <div className="mt-6 flex items-center justify-end gap-3 pt-4 border-t border-slate-800">
        <button
          type="button"
          className="cursor-pointer rounded-xl border border-slate-700 bg-slate-800 px-5 py-2.5 text-xs font-semibold text-slate-300 transition-colors hover:bg-slate-750"
          onClick={onClose}
        >
          Voltar à revisão
        </button>
        <button
          type="button"
          className="inline-flex cursor-pointer items-center gap-2 rounded-xl bg-gradient-to-r from-amber-500 to-amber-400 px-6 py-2.5 text-xs font-bold text-slate-950 shadow-lg shadow-amber-500/20 transition-all hover:brightness-110 active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-40"
          disabled={destination === "permanent" && !acknowledged}
          onClick={() => onConfirm(destination)}
        >
          <span>Limpar agora</span>
          <Icon name="arrow" size={15} />
        </button>
      </div>
    </dialog>
  );
}
