import { useCallback, useEffect, useRef, useState } from "react";
import { api, type RecycleBinInfo } from "../lib/api";
import { formatBytes, formatCount } from "../lib/format";
import { platformName } from "../lib/platform";
import Icon from "./Icon";

export default function RecycleBinCard() {
  const dialog = useRef<HTMLDialogElement>(null);
  const [info, setInfo] = useState<RecycleBinInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [emptying, setEmptying] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");

  const refresh = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      setInfo(await api.recycleBinInfo());
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (platformName() === "Windows") void refresh();
  }, [refresh]);

  useEffect(() => {
    const current = dialog.current;
    if (confirm) {
      current?.showModal();
      return () => current?.close();
    }
    current?.close();
  }, [confirm]);

  if (platformName() !== "Windows") return null;

  const empty = async () => {
    if (!info || info.items === 0) return;
    setEmptying(true);
    setError("");
    setNotice("");
    try {
      const updated = await api.emptyRecycleBin(info.items, info.bytes);
      setInfo(updated);
      setConfirm(false);
      setNotice("A Lixeira foi esvaziada.");
    } catch (e) {
      setConfirm(false);
      await refresh();
      setError(String(e));
    } finally {
      setEmptying(false);
    }
  };

  return (
    <>
      <section
        aria-label="Lixeira do Windows"
        className="flex flex-wrap items-center justify-between gap-4 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-5 shadow-lg"
      >
        <div className="flex min-w-0 items-center gap-4">
          <span className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl border border-slate-700 bg-slate-800 text-amber-400">
            <Icon name="trash" size={21} />
          </span>
          <div className="min-w-0">
            <h2 className="text-sm font-bold text-white">Lixeira do Windows</h2>
            <p className="mt-1 text-xs text-slate-400">
              {loading
                ? "Consultando todas as unidades…"
                : info
                  ? info.items
                    ? `${formatCount(info.items)} itens · ${formatBytes(info.bytes)}`
                    : "Vazia em todas as unidades."
                  : "Não foi possível consultar a Lixeira."}
            </p>
            {error && (
              <p className="mt-1 text-xs text-rose-300" role="alert">
                {error}
              </p>
            )}
            {notice && (
              <p className="mt-1 text-xs text-emerald-300" role="status">
                {notice}
              </p>
            )}
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            className="cursor-pointer rounded-xl border border-slate-700/80 bg-slate-850 px-3 py-2 text-xs font-semibold text-slate-200 transition-colors hover:bg-slate-800 disabled:cursor-not-allowed disabled:opacity-50"
            disabled={loading || emptying}
            onClick={() => void refresh()}
          >
            Atualizar
          </button>
          <button
            type="button"
            className="inline-flex cursor-pointer items-center gap-2 rounded-xl border border-rose-500/30 bg-rose-500/10 px-3 py-2 text-xs font-semibold text-rose-200 transition-colors hover:bg-rose-500/20 disabled:cursor-not-allowed disabled:opacity-50"
            disabled={loading || emptying || !info?.items}
            onClick={() => setConfirm(true)}
          >
            <Icon name="trash" size={14} />
            Esvaziar
          </button>
        </div>
      </section>

      <dialog
        ref={dialog}
        onCancel={(event) => {
          event.preventDefault();
          setConfirm(false);
        }}
        aria-labelledby="recycle-bin-title"
        className="m-auto w-full max-w-lg rounded-3xl border border-rose-500/30 bg-slate-900/95 p-7 text-slate-100 shadow-2xl backdrop:bg-black/75 backdrop:backdrop-blur-md"
      >
        <span className="flex h-12 w-12 items-center justify-center rounded-2xl border border-rose-500/30 bg-rose-500/10 text-rose-300">
          <Icon name="trash" size={24} />
        </span>
        <h2 id="recycle-bin-title" className="mt-4 text-xl font-bold text-white">
          Esvaziar a Lixeira em todas as unidades?
        </h2>
        <p className="mt-2 text-sm leading-relaxed text-slate-300">
          {info && `${formatCount(info.items)} itens (${formatBytes(info.bytes)}) serão apagados permanentemente.`} Você
          não poderá restaurá-los pela Lixeira.
        </p>
        <div className="mt-6 flex justify-end gap-3 border-t border-slate-800 pt-4">
          <button
            type="button"
            className="cursor-pointer rounded-xl border border-slate-700 bg-slate-800 px-4 py-2.5 text-xs font-semibold text-slate-300 hover:bg-slate-750"
            disabled={emptying}
            onClick={() => setConfirm(false)}
          >
            Cancelar
          </button>
          <button
            type="button"
            className="cursor-pointer rounded-xl bg-rose-500 px-5 py-2.5 text-xs font-bold text-white hover:bg-rose-400 disabled:cursor-wait disabled:opacity-50"
            disabled={emptying}
            onClick={() => void empty()}
          >
            {emptying ? "Esvaziando…" : "Apagar permanentemente"}
          </button>
        </div>
      </dialog>
    </>
  );
}
